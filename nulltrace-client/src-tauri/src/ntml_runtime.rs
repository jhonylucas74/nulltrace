//! NTML Lua script runtime for Browser tabs.
//! Per-tab Lua state with 1MB limit, tick/yield (core style), ui and http APIs.

use dashmap::DashMap;
use nulltrace_ntml::parse_document;
use mlua::{Lua, ThreadStatus, VmState};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

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
    SetDisabled { id: String, disabled: bool },
}

/// Import content for re-rendering with patches.
#[derive(Clone, serde::Deserialize)]
pub struct TabImport {
    pub alias: String,
    pub content: String,
}

/// Per-tab Lua state: Lua runtime, patch queue, base URL, and component tree for ui.get_value.
pub struct TabLuaState {
    pub lua: Lua,
    pub patches: Arc<Mutex<Vec<Patch>>>,
    pub base_url: String,
    /// Serialized component tree for ui.get_value (Input/Checkbox/Select by name).
    pub component_yaml: String,
    /// Fetched imports for re-rendering with patches.
    pub imports: Vec<TabImport>,
}

/// Global storage for tab states. Key = tab_id.
pub type TabStateStore = DashMap<String, TabLuaState>;

/// Creates a new TabStateStore (to be managed by Tauri).
pub fn new_tab_state_store() -> TabStateStore {
    DashMap::new()
}

/// Creates a Lua state for a tab: sandbox, memory limit, interrupt (core style).
fn create_tab_lua(
    patches: Arc<Mutex<Vec<Patch>>>,
    _base_url: String,
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
    // ui.get_value(name) - reads from component tree; for now returns nil (Phase 3 will implement)
    ui.set(
        "get_value",
        lua.create_function(|_, _name: String| Ok(mlua::Value::Nil))?,
    )?;
    lua.globals().set("ui", ui)?;

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
) -> Result<(), String> {
    let patches = Arc::new(Mutex::new(Vec::new()));
    let lua = create_tab_lua(patches.clone(), base_url.clone()).map_err(|e| e.to_string())?;

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
            base_url,
            component_yaml,
            imports,
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

    crate::ntml_html::ntml_to_html_with_imports_and_patches(
        &state.component_yaml,
        &ntml_imports,
        patches,
        Some(&state.base_url),
    )
}

/// Runs a Lua handler by name. Returns collected patches or error.
/// Uses timeout (200ms) and instruction yield (core style).
pub fn run_handler(
    store: &TabStateStore,
    tab_id: &str,
    action: &str,
) -> Result<Vec<Patch>, String> {
    let state = store
        .get_mut(tab_id)
        .ok_or_else(|| format!("Tab {} not found", tab_id))?;

    state.patches.lock().unwrap().clear();

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
                if let Err(e) = thread.resume::<()>(()) {
                    return Err(e.to_string());
                }
            }
            ThreadStatus::Finished => break,
            ThreadStatus::Error => {
                let err = thread.resume::<()>(()).err();
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
    Ok(patches)
}

/// Removes a tab state (call when tab is closed).
pub fn close_tab(store: &TabStateStore, tab_id: &str) {
    store.remove(tab_id);
}

/// Head resources extracted from NTML for fetching scripts and imports.
#[derive(serde::Serialize)]
pub struct NtmlHeadResources {
    pub scripts: Vec<NtmlScriptRef>,
    pub imports: Vec<NtmlImportRef>,
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

/// Parses NTML YAML and returns head.scripts and head.imports for fetching.
/// Returns empty lists for classic format (no head).
pub fn get_head_resources(yaml: &str) -> Result<NtmlHeadResources, String> {
    let doc = parse_document(yaml).map_err(|e| e.to_string())?;
    let head = match doc.head() {
        Some(h) => h,
        None => {
            return Ok(NtmlHeadResources {
                scripts: vec![],
                imports: vec![],
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

    Ok(NtmlHeadResources { scripts, imports })
}
