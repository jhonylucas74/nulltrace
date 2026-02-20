//! Lua API for httpd: serve(root, path) — path resolution, file lookup, 404 fallback.
//! Replaces heavy Lua logic in the httpd bin program with a single Rust call.

use super::context::VmContext;
use crate::db::fs_service::FsService;
use uuid::Uuid;
use mlua::{Lua, Result, Value};
use std::sync::Arc;

/// Content-Type mapping by extension (matches httpd Lua).
fn content_type_for_ext(ext: &str) -> &'static str {
    match ext {
        "ntml" => "application/x-ntml",
        "txt" | "md" => "text/plain; charset=utf-8",
        "html" => "text/html",
        _ => "application/octet-stream",
    }
}

/// Split filename into (base, ext). Last dot separates; ext must be non-empty after dot.
fn file_ext(name: &str) -> (&str, Option<&str>) {
    if let Some(last_dot) = name.rfind('.') {
        if last_dot + 1 < name.len() {
            return (&name[..last_dot], Some(&name[last_dot + 1..]));
        }
    }
    (name, None)
}

/// Normalize request path: strip leading /, map empty to "index", reject "..".
fn normalize_path(path: &str) -> Option<String> {
    let p = path.trim_start_matches('/');
    if p.is_empty() {
        return Some("index".to_string());
    }
    if p.contains("..") {
        return None;
    }
    Some(p.to_string())
}

/// Register the `httpd` table on the Lua state.
pub fn register(lua: &Lua, fs_service: Arc<FsService>) -> Result<()> {
    let httpd = lua.create_table()?;

    // httpd.serve(root, path) -> (body, status, headers)
    {
        let svc = fs_service.clone();
        httpd.set(
            "serve",
            lua.create_function(move |lua, (root, path): (String, String)| {
                let ctx = lua
                    .app_data_ref::<VmContext>()
                    .ok_or_else(|| mlua::Error::runtime("No VM context"))?;
                let vm_id = ctx.vm_id;
                drop(ctx);

                let path = path.trim();
                let normalized = match normalize_path(path) {
                    Some(p) => p,
                    None => {
                        // 404: path contained ..
                        let (body, ct) = read_404_sync(&svc, vm_id, &root);
                        let headers = lua.create_table()?;
                        headers.set("Content-Type", ct)?;
                        return Ok((body, 404u16, Value::Table(headers)));
                    }
                };

                // Split normalized path into (subdir, filename).
                // e.g. "components/flex" → subdir="components", filename="flex"
                //      "dashboard"       → subdir="",            filename="dashboard"
                let (subdir, filename) = match normalized.rfind('/') {
                    Some(slash) => (&normalized[..slash], &normalized[slash + 1..]),
                    None => ("", normalized.as_str()),
                };

                let lookup_dir = if subdir.is_empty() {
                    root.trim_end_matches('/').to_string()
                } else {
                    format!("{}/{}", root.trim_end_matches('/'), subdir)
                };

                let (base, ext) = file_ext(filename);

                let entries = tokio::task::block_in_place(|| {
                    tokio::runtime::Handle::current().block_on(async {
                        svc.ls(vm_id, &lookup_dir).await
                    })
                })
                .map_err(|e| mlua::Error::runtime(e.to_string()))?;

                let files: Vec<&str> = entries
                    .iter()
                    .filter(|e| e.node_type == "file")
                    .map(|e| e.name.as_str())
                    .collect();

                let mut target_file: Option<String> = None;
                let mut target_ext: Option<&str> = None;

                if let Some(e) = ext {
                    let full = format!("{}.{}", base, e);
                    if files.iter().any(|f| *f == full) {
                        target_file = Some(full);
                        target_ext = Some(e);
                    }
                } else {
                    let matches: Vec<&str> = files
                        .iter()
                        .filter(|f| file_ext(f).0 == base)
                        .copied()
                        .collect();
                    if matches.len() == 1 {
                        target_file = Some(matches[0].to_string());
                        target_ext = file_ext(matches[0]).1;
                    }
                }

                let (body, status, content_type) = if let Some(f) = target_file {
                    let full_path = format!("{}/{}", lookup_dir, f);
                    let content = tokio::task::block_in_place(|| {
                        tokio::runtime::Handle::current().block_on(async {
                            svc.read_file(vm_id, &full_path).await
                        })
                    })
                    .map_err(|e| mlua::Error::runtime(e.to_string()))?;

                    match content {
                        Some((data, _)) => {
                            let s = String::from_utf8(data)
                                .unwrap_or_else(|e| String::from_utf8_lossy(e.as_bytes()).into_owned());
                            let ct = content_type_for_ext(target_ext.unwrap_or(""));
                            (s, 200u16, ct)
                        }
                        None => {
                            let (body, ct) = read_404_sync(&svc, vm_id, &root);
                            (body, 404u16, ct)
                        }
                    }
                } else {
                    let (body, ct) = read_404_sync(&svc, vm_id, &root);
                    (body, 404u16, ct)
                };

                let headers = lua.create_table()?;
                headers.set("Content-Type", content_type)?;

                Ok((body, status, Value::Table(headers)))
            })?,
        )?;
    }

    lua.globals().set("httpd", httpd)?;
    Ok(())
}

/// Try 404.ntml, then 404.txt, else empty. Returns (body, content_type).
fn read_404_sync(svc: &Arc<FsService>, vm_id: Uuid, root: &str) -> (String, &'static str) {
    let svc = Arc::clone(svc);
    let root = root.to_string();
    tokio::task::block_in_place(|| {
        tokio::runtime::Handle::current().block_on(async move {
            let root_trimmed = root.trim_end_matches('/');
            if let Ok(Some((data, _))) = svc.read_file(vm_id, &format!("{}/404.ntml", root_trimmed)).await {
                return (String::from_utf8(data).unwrap_or_default(), "application/x-ntml");
            }
            if let Ok(Some((data, _))) = svc.read_file(vm_id, &format!("{}/404.txt", root_trimmed)).await {
                return (String::from_utf8(data).unwrap_or_default(), "text/plain");
            }
            (String::new(), "application/x-ntml")
        })
    })
}
