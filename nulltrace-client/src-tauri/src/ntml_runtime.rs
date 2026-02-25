//! NTML Lua script runtime for Browser tabs.
//! Per-tab Lua state with 1MB limit, tick/yield (core style), ui and http APIs.

use dashmap::DashMap;
use nulltrace_ntml::parse_document;
use mlua::{Lua, ThreadStatus, VmState};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use tauri::{Emitter, Manager};
use uuid::Uuid;

/// Lua heap limit per tab: 1 MB (same as core VM).
pub const LUA_MEMORY_LIMIT_BYTES: usize = 1024 * 1024;

/// Handler timeout per spec: 200ms.
const HANDLER_TIMEOUT_MS: u64 = 200;

/// UI mutation patches collected during handler execution.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Patch {
    SetText { id: String, text: String },
    SetVisible { id: String, visible: bool },
    SetValue { id: String, value: f64 },
    SetInputValue { id: String, value: String },
    SetDisabled { id: String, disabled: bool },
    /// Replace the element's entire CSS class string.
    SetClass { id: String, class: String },
    /// Append one or more space-separated class tokens to the element's class.
    AddClass { id: String, class: String },
    /// Remove one or more space-separated class tokens from the element's class.
    RemoveClass { id: String, class: String },
}

/// Import content for re-rendering with patches.
#[derive(Clone, serde::Deserialize)]
pub struct TabImport {
    pub alias: String,
    pub content: String,
}

/// Network entry from Lua http.post, for DevTools Network tab.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct HandlerNetworkEntry {
    pub origin: String,
    pub url: String,
    pub method: String,
    pub status: Option<u16>,
    pub duration_ms: Option<u64>,
    pub content_type: Option<String>,
    pub size: Option<usize>,
    pub response: Option<String>,
    pub timestamp: u64,
}

/// Per-tab Lua state: Lua runtime, patch queue, base URL, and component tree for ui.get_value.
pub struct TabLuaState {
    pub lua: Lua,
    pub patches: Arc<Mutex<Vec<Patch>>>,
    /// Accumulated patches across handler runs (visibility, text, etc.) so UI state persists.
    pub accumulated_patches: Arc<Mutex<Vec<Patch>>>,
    pub base_url: String,
    /// Serialized component tree for ui.get_value (Input/Checkbox/Select by name).
    pub component_yaml: String,
    /// Fetched imports for re-rendering with patches.
    pub imports: Vec<TabImport>,
    /// Fetched external Markdown contents: (src_path, markdown_text).
    pub markdown_contents: Vec<(String, String)>,
    /// Form values for current handler invocation (set before run_handler, read by ui.get_value).
    pub form_values: Arc<Mutex<Option<std::collections::HashMap<String, String>>>>,
    /// Captured print() output during handler execution.
    pub print_log: Arc<Mutex<Vec<String>>>,
    /// Token for current handler run (set before run_handler, used by http.post).
    pub current_token: Arc<Mutex<Option<String>>>,
    /// Network entries from http.post during current handler run (for DevTools).
    pub network_entries: Arc<Mutex<Vec<HandlerNetworkEntry>>>,
}

/// Global key-value storage: origin → (key → value). Shared across all tabs of same origin.
pub type BrowserStorageStore = Arc<DashMap<String, std::collections::HashMap<String, String>>>;

pub fn new_browser_storage_store() -> BrowserStorageStore {
    Arc::new(DashMap::new())
}

/// Pending card request: stored when Lua calls browser.request_card, consumed when user confirms.
#[derive(Clone, Debug)]
pub struct CardRequestPending {
    pub tab_id: String,
    pub callback_action: String,
    pub origin: String,
}

/// Global store: request_id → CardRequestPending.
pub type CardRequestStore = Arc<DashMap<String, CardRequestPending>>;

pub fn new_card_request_store() -> CardRequestStore {
    Arc::new(DashMap::new())
}

fn extract_origin(base_url: &str) -> String {
    let u = base_url.replace("http://", "").replace("https://", "");
    let idx = u.find('/');
    match idx {
        Some(i) => u[..i].to_string(),
        None => u.to_string(),
    }
}

/// Global storage for tab states. Key = tab_id.
pub type TabStateStore = DashMap<String, TabLuaState>;

/// Creates a new TabStateStore (to be managed by Tauri).
pub fn new_tab_state_store() -> TabStateStore {
    DashMap::new()
}

/// Parses raw HTTP response (from curl stdout) into status and body.
fn parse_http_response(raw: &str) -> (u16, String) {
    let status = raw
        .lines()
        .next()
        .and_then(|line| {
            line.split_whitespace()
                .nth(1)
                .and_then(|s| s.parse::<u16>().ok())
        })
        .unwrap_or(0);
    let body = raw
        .split("\r\n\r\n")
        .nth(1)
        .or_else(|| raw.split("\n\n").nth(1))
        .unwrap_or("")
        .to_string();
    (status, body)
}

/// Extracts Content-Type from raw HTTP response headers.
fn parse_content_type_from_response(raw: &str) -> Option<String> {
    let head = raw.split("\r\n\r\n").next().or_else(|| raw.split("\n\n").next())?;
    for line in head.lines().skip(1) {
        if line.to_lowercase().starts_with("content-type:") {
            let value = line.splitn(2, ':').nth(1)?.trim();
            return Some(value.split(';').next()?.trim().to_string());
        }
    }
    None
}

/// Creates a Lua state for a tab: sandbox, memory limit, interrupt (core style).
fn create_tab_lua(
    patches: Arc<Mutex<Vec<Patch>>>,
    _base_url: String,
    form_values: Arc<Mutex<Option<std::collections::HashMap<String, String>>>>,
    print_log: Arc<Mutex<Vec<String>>>,
    current_token: Arc<Mutex<Option<String>>>,
    network_entries: Arc<Mutex<Vec<HandlerNetworkEntry>>>,
    storage_store: BrowserStorageStore,
    origin: String,
    user_id: String,
    tab_id: String,
    card_request_store: CardRequestStore,
    app: tauri::AppHandle,
) -> Result<Lua, mlua::Error> {
    let lua = Lua::new();
    let _ = lua.sandbox(true);

    // Block dangerous globals (per spec)
    for name in ["io", "os", "file", "require", "loadfile", "dofile", "coroutine", "debug"] {
        let msg = format!("{} is not available in NTML scripts", name);
        lua.globals().set(
            name,
            lua.create_function(move |_, _: mlua::Value| {
                Err::<(), _>(mlua::Error::RuntimeError(msg.clone()))
            })?,
        )?;
    }

    // Register print() → captured to print_log
    let pl = print_log.clone();
    lua.globals().set(
        "print",
        lua.create_function(move |_, args: mlua::Variadic<mlua::Value>| {
            let parts: Vec<String> = args
                .iter()
                .map(|v| match v {
                    mlua::Value::String(s) => String::from_utf8_lossy(&s.as_bytes()).to_string(),
                    mlua::Value::Integer(i) => i.to_string(),
                    mlua::Value::Number(n) => n.to_string(),
                    mlua::Value::Boolean(b) => b.to_string(),
                    mlua::Value::Nil => "nil".to_string(),
                    _ => format!("{:?}", v),
                })
                .collect();
            pl.lock().unwrap().push(parts.join("\t"));
            Ok(())
        })?,
    )?;

    // Interrupt: yield every 2 calls (same style as nulltrace-core os.rs)
    let count = AtomicU64::new(0);
    lua.set_interrupt(move |_| {
        if count.fetch_add(1, Ordering::Relaxed) % 2 == 0 {
            return Ok(VmState::Yield);
        }
        Ok(VmState::Continue)
    });

    lua.set_memory_limit(LUA_MEMORY_LIMIT_BYTES)?;

    // Register ui API
    let ui = lua.create_table()?;
    let p1 = patches.clone();
    ui.set(
        "set_text",
        lua.create_function(move |_, (id, text): (String, String)| {
            p1.lock().unwrap().push(Patch::SetText { id, text });
            Ok(())
        })?,
    )?;
    let p2 = patches.clone();
    ui.set(
        "set_visible",
        lua.create_function(move |_, (id, visible): (String, bool)| {
            p2.lock().unwrap().push(Patch::SetVisible { id, visible });
            Ok(())
        })?,
    )?;
    let p3 = patches.clone();
    ui.set(
        "set_value",
        lua.create_function(move |_, (id, value): (String, f64)| {
            p3.lock().unwrap().push(Patch::SetValue { id, value });
            Ok(())
        })?,
    )?;
    let p4 = patches.clone();
    ui.set(
        "set_disabled",
        lua.create_function(move |_, (id, disabled): (String, bool)| {
            p4.lock().unwrap().push(Patch::SetDisabled { id, disabled });
            Ok(())
        })?,
    )?;
    let p5 = patches.clone();
    ui.set(
        "set_input_value",
        lua.create_function(move |_, (id, value): (String, String)| {
            p5.lock().unwrap().push(Patch::SetInputValue { id, value });
            Ok(())
        })?,
    )?;
    let p6 = patches.clone();
    ui.set(
        "set_class",
        lua.create_function(move |_, (id, class): (String, String)| {
            p6.lock().unwrap().push(Patch::SetClass { id, class });
            Ok(())
        })?,
    )?;
    let p7 = patches.clone();
    ui.set(
        "add_class",
        lua.create_function(move |_, (id, class): (String, String)| {
            p7.lock().unwrap().push(Patch::AddClass { id, class });
            Ok(())
        })?,
    )?;
    let p8 = patches.clone();
    ui.set(
        "remove_class",
        lua.create_function(move |_, (id, class): (String, String)| {
            p8.lock().unwrap().push(Patch::RemoveClass { id, class });
            Ok(())
        })?,
    )?;
    // ui.get_value(name) - reads from form_values passed at handler invocation
    let fv = form_values.clone();
    ui.set(
        "get_value",
        lua.create_function(move |lua, name: String| {
            let guard = fv.lock().unwrap();
            let val = guard
                .as_ref()
                .and_then(|m| m.get(&name).cloned());
            match val {
                Some(s) => Ok(mlua::Value::String(lua.create_string(&s)?)),
                None => Ok(mlua::Value::Nil),
            }
        })?,
    )?;
    lua.globals().set("ui", ui)?;

    // Register storage API (per-user, per-origin; write-through to disk)
    let storage = lua.create_table()?;
    let composite_key = storage_composite_key(&user_id, &origin);

    let ss1 = storage_store.clone();
    let ck1 = composite_key.clone();
    let app1 = app.clone();
    let uid1 = user_id.clone();
    let or1 = origin.clone();
    storage.set(
        "set",
        lua.create_function(move |_, (key, value): (String, String)| {
            ss1.entry(ck1.clone()).or_default().insert(key, value);
            persist_origin_storage(&app1, &uid1, &or1, &ss1);
            Ok(())
        })?,
    )?;

    let ss2 = storage_store.clone();
    let ck2 = composite_key.clone();
    storage.set(
        "get",
        lua.create_function(move |lua, key: String| {
            let val = ss2.get(&ck2).and_then(|m| m.get(&key).cloned());
            match val {
                Some(s) => Ok(mlua::Value::String(lua.create_string(&s)?)),
                None => Ok(mlua::Value::Nil),
            }
        })?,
    )?;

    let ss3 = storage_store.clone();
    let ck3 = composite_key.clone();
    let app3 = app.clone();
    let uid3 = user_id.clone();
    let or3 = origin.clone();
    storage.set(
        "remove",
        lua.create_function(move |_, key: String| {
            if let Some(mut m) = ss3.get_mut(&ck3) {
                m.remove(&key);
            }
            persist_origin_storage(&app3, &uid3, &or3, &ss3);
            Ok(())
        })?,
    )?;

    let ss4 = storage_store.clone();
    let ck4 = composite_key.clone();
    let app4 = app.clone();
    let uid4 = user_id.clone();
    let or4 = origin.clone();
    storage.set(
        "clear",
        lua.create_function(move |_, _: ()| {
            ss4.remove(&ck4);
            // Remove the file from disk too
            if let Some(path) = storage_file_path(&app4, &uid4, &or4) {
                let _ = std::fs::remove_file(&path);
            }
            Ok(())
        })?,
    )?;

    let ss5 = storage_store.clone();
    let ck5 = composite_key.clone();
    storage.set(
        "keys",
        lua.create_function(move |lua, _: ()| {
            let tbl = lua.create_table()?;
            if let Some(m) = ss5.get(&ck5) {
                for (i, key) in m.keys().enumerate() {
                    tbl.set(i + 1, key.clone())?;
                }
            }
            Ok(tbl)
        })?,
    )?;

    lua.globals().set("storage", storage)?;

    // Register http API (post: run curl in VM for in-game URLs)
    let http_tbl = lua.create_table()?;
    let ct = current_token.clone();
    let net_entries = network_entries.clone();
    http_tbl.set(
        "post",
        lua.create_function(move |lua, (url, body): (String, Option<String>)| {
            let token = ct.lock().unwrap().clone().ok_or_else(|| {
                mlua::Error::RuntimeError("http.post: no auth token (handler not invoked with token)".to_string())
            })?;
            let url_norm = url
                .trim()
                .replace("https://", "")
                .replace("http://", "");
            let full_url = format!("http://{}", url_norm);
            let body_str = body.unwrap_or_default();
            let args = if body_str.is_empty() {
                vec![url_norm]
            } else {
                vec![url_norm, body_str]
            };
            let start = std::time::Instant::now();
            let timestamp = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_millis() as u64;
            match crate::grpc::run_process_blocking("curl".to_string(), args, token) {
                Ok(res) => {
                    let duration_ms = start.elapsed().as_millis() as u64;
                    let (status, resp_body) = parse_http_response(&res.stdout);
                    let content_type = parse_content_type_from_response(&res.stdout);
                    let size = resp_body.len();
                    let response_preview = if resp_body.len() > 8000 {
                        resp_body.chars().take(8000).collect::<String>()
                    } else {
                        resp_body.clone()
                    };
                    net_entries.lock().unwrap().push(HandlerNetworkEntry {
                        origin: "lua".to_string(),
                        url: full_url,
                        method: "POST".to_string(),
                        status: Some(status),
                        duration_ms: Some(duration_ms),
                        content_type: content_type,
                        size: Some(size),
                        response: Some(response_preview),
                        timestamp,
                    });
                    let tbl = lua.create_table()?;
                    tbl.set("status", status)?;
                    tbl.set("body", resp_body)?;
                    Ok(mlua::Value::Table(tbl))
                }
                Err(e) => {
                    let duration_ms = start.elapsed().as_millis() as u64;
                    net_entries.lock().unwrap().push(HandlerNetworkEntry {
                        origin: "lua".to_string(),
                        url: full_url,
                        method: "POST".to_string(),
                        status: None,
                        duration_ms: Some(duration_ms),
                        content_type: None,
                        size: None,
                        response: Some(e.clone()),
                        timestamp,
                    });
                    Err(mlua::Error::RuntimeError(format!("http.post failed: {}", e)))
                }
            }
        })?,
    )?;
    lua.globals().set("http", http_tbl)?;

    // Register browser API (request_card: show OS modal to pick a saved card)
    let browser = lua.create_table()?;
    let tab_id_browser = tab_id.clone();
    let crs = card_request_store.clone();
    let app_browser = app.clone();
    browser.set(
        "request_card",
        lua.create_function(move |lua, (origin_arg, callback_action): (String, String)| {
            let request_id = Uuid::new_v4().to_string();
            crs.insert(
                request_id.clone(),
                CardRequestPending {
                    tab_id: tab_id_browser.clone(),
                    callback_action: callback_action.clone(),
                    origin: origin_arg.clone(),
                },
            );
            let payload = serde_json::json!({
                "request_id": request_id,
                "tab_id": tab_id_browser,
                "origin": origin_arg,
                "callback_action": callback_action,
            });
            let _ = app_browser.emit("ntml:request-card", payload);
            Ok(mlua::Value::String(lua.create_string(&request_id)?))
        })?,
    )?;
    lua.globals().set("browser", browser)?;

    Ok(lua)
}

/// Script source: path and content.
#[derive(serde::Deserialize)]
pub struct ScriptSource {
    pub src: String,
    pub content: String,
}

/// Creates and stores a tab state. Loads scripts into Lua. Returns Ok(()) on success.
pub fn create_tab_state(
    store: &TabStateStore,
    tab_id: String,
    base_url: String,
    script_sources: Vec<ScriptSource>,
    component_yaml: String,
    imports: Vec<TabImport>,
    markdown_contents: Vec<(String, String)>,
    storage_store: BrowserStorageStore,
    user_id: String,
    card_request_store: CardRequestStore,
    app: tauri::AppHandle,
) -> Result<(), String> {
    let patches = Arc::new(Mutex::new(Vec::new()));
    let accumulated_patches = Arc::new(Mutex::new(Vec::new()));
    let form_values = Arc::new(Mutex::new(None));
    let print_log = Arc::new(Mutex::new(Vec::new()));
    let current_token = Arc::new(Mutex::new(None));
    let network_entries = Arc::new(Mutex::new(Vec::new()));
    let origin = extract_origin(&base_url);

    // Hydrate memory from disk before the Lua state is created (scripts may read storage immediately)
    ensure_origin_loaded(&app, &user_id, &origin, &storage_store);

    let lua = create_tab_lua(
        patches.clone(),
        base_url.clone(),
        form_values.clone(),
        print_log.clone(),
        current_token.clone(),
        network_entries.clone(),
        storage_store,
        origin,
        user_id,
        tab_id.clone(),
        card_request_store,
        app,
    )
    .map_err(|e| e.to_string())?;

    // Load scripts in order
    let mut all_script = String::new();
    for s in script_sources {
        all_script.push_str(&s.content);
        all_script.push('\n');
    }
    if !all_script.is_empty() {
        lua.load(&all_script)
            .exec()
            .map_err(|e| format!("Script load error: {}", e))?;
    }

    store.insert(
        tab_id,
        TabLuaState {
            lua,
            patches,
            accumulated_patches,
            base_url,
            component_yaml,
            imports,
            markdown_contents,
            form_values,
            print_log,
            current_token,
            network_entries,
        },
    );
    Ok(())
}

/// Renders the tab's document with patches applied. Returns HTML.
pub fn render_with_patches(
    store: &TabStateStore,
    tab_id: &str,
    patches: &[Patch],
) -> Result<String, String> {
    let state = store
        .get(tab_id)
        .ok_or_else(|| format!("Tab {} not found", tab_id))?;

    let ntml_imports: Vec<crate::ntml_html::NtmlImport> = state
        .imports
        .iter()
        .map(|i| crate::ntml_html::NtmlImport {
            alias: i.alias.clone(),
            content: i.content.clone(),
        })
        .collect();

    let md_map: std::collections::HashMap<String, String> =
        state.markdown_contents.iter().cloned().collect();

    crate::ntml_html::ntml_to_html_with_imports_and_patches(
        &state.component_yaml,
        &ntml_imports,
        &md_map,
        patches,
        Some(&state.base_url),
    )
}

/// Merges new patches into accumulated state and renders. UI state (visibility, text, etc.) persists across handler runs.
pub fn render_with_accumulated_patches(
    store: &TabStateStore,
    tab_id: &str,
    new_patches: &[Patch],
) -> Result<String, String> {
    let state = store
        .get(tab_id)
        .ok_or_else(|| format!("Tab {} not found", tab_id))?;

    let patches = {
        let mut acc = state.accumulated_patches.lock().unwrap();
        acc.extend(new_patches.iter().cloned());
        acc.clone()
    };

    let ntml_imports: Vec<crate::ntml_html::NtmlImport> = state
        .imports
        .iter()
        .map(|i| crate::ntml_html::NtmlImport {
            alias: i.alias.clone(),
            content: i.content.clone(),
        })
        .collect();

    let md_map: std::collections::HashMap<String, String> =
        state.markdown_contents.iter().cloned().collect();

    crate::ntml_html::ntml_to_html_with_imports_and_patches(
        &state.component_yaml,
        &ntml_imports,
        &md_map,
        &patches,
        Some(&state.base_url),
    )
}

/// Runs a Lua handler by name. Returns collected patches and print output, or error.
/// Uses timeout (200ms) and instruction yield (core style).
/// `token` is set for the duration of the handler so http.post can authenticate.
pub fn run_handler(
    store: &TabStateStore,
    tab_id: &str,
    action: &str,
    form_values: Option<std::collections::HashMap<String, String>>,
    event_data: Option<std::collections::HashMap<String, String>>,
    token: Option<String>,
) -> Result<(Vec<Patch>, Vec<String>, Vec<HandlerNetworkEntry>), String> {
    let state = store
        .get_mut(tab_id)
        .ok_or_else(|| format!("Tab {} not found", tab_id))?;

    state.patches.lock().unwrap().clear();
    state.network_entries.lock().unwrap().clear();

    // Set token for http.post (cleared after handler)
    *state.current_token.lock().unwrap() = token;

    // Build context object (ctx) to pass as first argument to handler (React-style)
    let ctx = state.lua.create_table().map_err(|e| e.to_string())?;
    let event_data_tbl = state.lua.create_table().map_err(|e| e.to_string())?;
    if let Some(ref ed) = event_data {
        for (k, v) in ed {
            event_data_tbl.set(k.clone(), v.clone()).map_err(|e| e.to_string())?;
        }
    }
    ctx.set("eventData", event_data_tbl).map_err(|e| e.to_string())?;
    let form_values_tbl = state.lua.create_table().map_err(|e| e.to_string())?;
    if let Some(ref fv) = form_values {
        for (k, v) in fv {
            form_values_tbl.set(k.clone(), v.clone()).map_err(|e| e.to_string())?;
        }
    }
    ctx.set("formValues", form_values_tbl).map_err(|e| e.to_string())?;
    let target_id_val: mlua::Value = event_data
        .as_ref()
        .and_then(|ed| ed.get("id"))
        .and_then(|s| state.lua.create_string(s).ok())
        .map(mlua::Value::String)
        .unwrap_or(mlua::Value::Nil);
    ctx.set("targetId", target_id_val).map_err(|e| e.to_string())?;

    // Set form_values for ui.get_value
    *state.form_values.lock().unwrap() = form_values;

    // Set event_row, event_col, event_data for Lua handlers (backward compat)
    if let Some(ref ed) = event_data {
        if let Some(r) = ed.get("row") {
            let _ = state.lua.globals().set("event_row", r.parse::<i32>().unwrap_or(0));
        }
        if let Some(c) = ed.get("col") {
            let _ = state.lua.globals().set("event_col", c.parse::<i32>().unwrap_or(0));
        }
        let table = state.lua.create_table().map_err(|e| e.to_string())?;
        for (k, v) in ed {
            table.set(k.clone(), v.clone()).map_err(|e| e.to_string())?;
        }
        let _ = state.lua.globals().set("event_data", table);
    } else {
        let _ = state.lua.globals().set("event_row", 0i32);
        let _ = state.lua.globals().set("event_col", 0i32);
    }

    let func = state
        .lua
        .globals()
        .get::<mlua::Function>(action)
        .map_err(|_| format!("Lua function '{}' not found", action))?;

    let thread = state
        .lua
        .create_thread(func)
        .map_err(|e| format!("Failed to create thread: {}", e))?;

    // Store start time for interrupt to check timeout
    let start_epoch = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_millis() as u64;
    let start_atomic = Arc::new(std::sync::atomic::AtomicU64::new(start_epoch));

    // Replace interrupt with one that also checks timeout (200ms per spec)
    let start_check = start_atomic.clone();
    let count = AtomicU64::new(0);
    state.lua.set_interrupt(move |_| {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64;
        let elapsed = now.saturating_sub(start_check.load(Ordering::Relaxed));
        if elapsed > HANDLER_TIMEOUT_MS {
            return Ok(VmState::Yield);
        }
        if count.fetch_add(1, Ordering::Relaxed) % 2 == 0 {
            return Ok(VmState::Yield);
        }
        Ok(VmState::Continue)
    });

    loop {
        match thread.status() {
            ThreadStatus::Resumable => {
                if let Err(e) = thread.resume::<()>(ctx.clone()) {
                    return Err(e.to_string());
                }
            }
            ThreadStatus::Finished => break,
            ThreadStatus::Error => {
                let err = thread.resume::<()>(ctx.clone()).err();
                return Err(err
                    .map(|e| e.to_string())
                    .unwrap_or_else(|| "Lua runtime error".to_string()));
            }
            ThreadStatus::Running => break,
        }
    }

    // Restore default interrupt (yield every 2)
    let count2 = AtomicU64::new(0);
    state.lua.set_interrupt(move |_| {
        if count2.fetch_add(1, Ordering::Relaxed) % 2 == 0 {
            return Ok(VmState::Yield);
        }
        Ok(VmState::Continue)
    });

    let patches = state.patches.lock().unwrap().clone();

    // Drain print_log and network_entries
    let print_output: Vec<String> = std::mem::take(&mut *state.print_log.lock().unwrap());
    let network_entries: Vec<HandlerNetworkEntry> = std::mem::take(&mut *state.network_entries.lock().unwrap());

    // Clear form_values and token after handler
    *state.form_values.lock().unwrap() = None;
    *state.current_token.lock().unwrap() = None;

    Ok((patches, print_output, network_entries))
}

/// Removes a tab state (call when tab is closed).
pub fn close_tab(store: &TabStateStore, tab_id: &str) {
    store.remove(tab_id);
}

/// Head resources extracted from NTML for fetching scripts, imports, and external markdowns.
#[derive(serde::Serialize)]
pub struct NtmlHeadResources {
    pub scripts: Vec<NtmlScriptRef>,
    pub imports: Vec<NtmlImportRef>,
    pub markdowns: Vec<NtmlMarkdownRef>,
}

#[derive(serde::Serialize)]
pub struct NtmlScriptRef {
    pub src: String,
}

#[derive(serde::Serialize)]
pub struct NtmlImportRef {
    pub src: String,
    pub alias: String,
}

#[derive(serde::Serialize)]
pub struct NtmlMarkdownRef {
    pub src: String,
}

/// Recursively collects all `Markdown { src: Some(_) }` srcs from a component tree.
fn collect_markdown_srcs(components: &[nulltrace_ntml::Component]) -> Vec<String> {
    use nulltrace_ntml::components::*;
    let mut srcs = Vec::new();
    for comp in components {
        match comp {
            Component::Markdown(m) => {
                if let Some(src) = &m.src {
                    srcs.push(src.clone());
                }
            }
            Component::Container(c) => {
                if let Some(ch) = &c.children { srcs.extend(collect_markdown_srcs(ch)); }
            }
            Component::Flex(c) => {
                if let Some(ch) = &c.children { srcs.extend(collect_markdown_srcs(ch)); }
            }
            Component::Grid(c) => {
                if let Some(ch) = &c.children { srcs.extend(collect_markdown_srcs(ch)); }
            }
            Component::Stack(c) => {
                if let Some(ch) = &c.children { srcs.extend(collect_markdown_srcs(ch)); }
            }
            Component::Row(c) => {
                if let Some(ch) = &c.children { srcs.extend(collect_markdown_srcs(ch)); }
            }
            Component::Column(c) => {
                if let Some(ch) = &c.children { srcs.extend(collect_markdown_srcs(ch)); }
            }
            Component::Button(c) => {
                if let Some(ch) = &c.children { srcs.extend(collect_markdown_srcs(ch)); }
            }
            Component::Link(c) => {
                if let Some(ch) = &c.children { srcs.extend(collect_markdown_srcs(ch)); }
            }
            Component::List(c) => {
                if let Some(ch) = &c.children { srcs.extend(collect_markdown_srcs(ch)); }
            }
            Component::ListItem(c) => {
                if let Some(ch) = &c.children { srcs.extend(collect_markdown_srcs(ch)); }
            }
            Component::Blockquote(c) => {
                if let Some(ch) = &c.children { srcs.extend(collect_markdown_srcs(ch)); }
            }
            Component::Details(c) => {
                if let Some(ch) = &c.children { srcs.extend(collect_markdown_srcs(ch)); }
            }
            _ => {}
        }
    }
    srcs
}

/// Parses NTML YAML and returns head.scripts, head.imports, and body markdown srcs for fetching.
/// Returns empty lists for classic format (no head).
pub fn get_head_resources(yaml: &str) -> Result<NtmlHeadResources, String> {
    let doc = parse_document(yaml).map_err(|e| e.to_string())?;

    // Collect markdown srcs from body regardless of format
    let body_root = doc.root_component();
    let md_srcs = {
        let mut v = collect_markdown_srcs(std::slice::from_ref(body_root));
        v.dedup();
        v
    };
    let markdowns = md_srcs.into_iter().map(|src| NtmlMarkdownRef { src }).collect();

    let head = match doc.head() {
        Some(h) => h,
        None => {
            return Ok(NtmlHeadResources {
                scripts: vec![],
                imports: vec![],
                markdowns,
            });
        }
    };

    let scripts = head
        .scripts
        .as_ref()
        .map(|s| {
            s.iter()
                .map(|i| NtmlScriptRef {
                    src: i.src.clone(),
                })
                .collect()
        })
        .unwrap_or_default();

    let imports = head
        .imports
        .as_ref()
        .map(|i| {
            i.iter()
                .map(|c| NtmlImportRef {
                    src: c.src.clone(),
                    alias: c.alias.clone(),
                })
                .collect()
        })
        .unwrap_or_default();

    Ok(NtmlHeadResources { scripts, imports, markdowns })
}

// ── Storage persistence ───────────────────────────────────────────────────────

/// Builds the in-memory DashMap key for a (user_id, origin) pair.
pub fn storage_composite_key(user_id: &str, origin: &str) -> String {
    format!("{}:{}", user_id, origin)
}

/// Turns an origin like "localhost:8080" into a safe filename "localhost_8080".
fn sanitize_origin_for_filename(origin: &str) -> String {
    origin.replace(':', "_")
}

/// Returns the path to the JSON file that stores a (user_id, origin) pair on disk.
pub fn storage_file_path(app: &tauri::AppHandle, user_id: &str, origin: &str) -> Option<std::path::PathBuf> {
    let base = app.path().app_data_dir().ok()?;
    let sanitized = sanitize_origin_for_filename(origin);
    Some(base.join("browser_storage").join(user_id).join(format!("{}.json", sanitized)))
}

/// Writes the current in-memory data for a (user_id, origin) pair to disk.
pub fn persist_origin_storage(app: &tauri::AppHandle, user_id: &str, origin: &str, store: &BrowserStorageStore) {
    let key = storage_composite_key(user_id, origin);
    let data: std::collections::HashMap<String, String> = store
        .get(&key)
        .map(|m| m.clone())
        .unwrap_or_default();
    if let Some(path) = storage_file_path(app, user_id, origin) {
        if let Ok(json) = serde_json::to_string(&data) {
            if let Some(parent) = path.parent() {
                let _ = std::fs::create_dir_all(parent);
            }
            let _ = std::fs::write(&path, json);
        }
    }
}

/// Loads a (user_id, origin) pair from disk into memory if not already present.
/// No-op if the key is already in memory or if there is no file on disk.
pub fn ensure_origin_loaded(app: &tauri::AppHandle, user_id: &str, origin: &str, store: &BrowserStorageStore) {
    let key = storage_composite_key(user_id, origin);
    if store.contains_key(&key) {
        return; // already hydrated
    }
    let Some(path) = storage_file_path(app, user_id, origin) else { return; };
    if let Ok(contents) = std::fs::read_to_string(&path) {
        if let Ok(map) = serde_json::from_str::<std::collections::HashMap<String, String>>(&contents) {
            store.insert(key, map);
        }
    }
}

// ── Console REPL ──────────────────────────────────────────────────────────────

/// Result of evaluating arbitrary Lua code in a tab's context.
#[derive(serde::Serialize)]
pub struct EvalLuaResult {
    pub output: Vec<String>,
    pub error: Option<String>,
}

/// Strips `local` from variable/function declarations at the start of each line.
///
/// In Lua, `local` variables are scoped to the chunk. Since each REPL eval is a
/// new chunk, locals die immediately. By promoting them to globals (which live in
/// the tab's sandboxed Lua state), variables defined in one REPL input persist
/// to the next — matching the interactive console experience users expect.
///
/// Only strips `local ` at the logical start of a line (after whitespace), so
/// `local` inside strings or comments that don't start the line are unaffected.
fn strip_repl_locals(code: &str) -> String {
    if !code.contains("local ") {
        return code.to_string();
    }
    code.lines()
        .map(|line| {
            let trimmed = line.trim_start();
            if trimmed.starts_with("local ") {
                let indent_len = line.len() - trimmed.len();
                format!("{}{}", &line[..indent_len], &trimmed["local ".len()..])
            } else {
                line.to_string()
            }
        })
        .collect::<Vec<_>>()
        .join("\n")
}

/// Evaluates arbitrary Lua code in an existing tab's Lua state.
/// Captures print() output. Returns output lines and optional error message.
/// If the tab has no Lua state (built-in page), returns an error.
///
/// `local` declarations are automatically promoted to tab-global scope so that
/// variables defined in one REPL input are accessible in subsequent inputs.
pub fn eval_lua(store: &TabStateStore, tab_id: &str, code: &str) -> EvalLuaResult {
    let state = match store.get_mut(tab_id) {
        Some(s) => s,
        None => {
            return EvalLuaResult {
                output: vec![],
                error: Some("No Lua context for this tab (built-in page)".into()),
            }
        }
    };

    // Promote `local` declarations to globals so they persist across REPL inputs.
    let processed = strip_repl_locals(code);

    // Clear print log before execution
    state.print_log.lock().unwrap().clear();

    // Execute the processed code in the tab's Lua state
    let result = state.lua.load(&processed).exec();

    // Collect print() output
    let output = std::mem::take(&mut *state.print_log.lock().unwrap());

    EvalLuaResult {
        output,
        error: result.err().map(|e| e.to_string()),
    }
}
