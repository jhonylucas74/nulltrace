# In-Game Browser DevTools

## Overview

DevTools opens via **F12** when the browser window is focused. It is a separate virtual window in the game's window manager (type: `"devtools"`). It has 4 tabs:

| Tab | Description |
|-----|-------------|
| **Sources** | Raw NTML YAML of the current page |
| **Network** | All HTTP requests (browser curl + Lua `http.*`) with timing |
| **Storage** | Fake localStorage per origin, editable from DevTools |
| **Console** | Lua `print()` output from NTML scripts |

## Architecture

### Shared State: `DevToolsContext`
- Lives at `src/contexts/DevToolsContext.tsx`
- `DevToolsContextProvider` wraps inside `WindowManagerProvider` in `Desktop.tsx`
- **Browser writes**: network entries, NTML source, console lines, inspected tab
- **DevTools reads**: displays all of the above

### Window Type
- `"devtools"` added to `WindowType` in `WindowManagerContext.tsx`
- Default size: 900×560

### Components
- `src/components/DevTools.tsx` — UI with 4 tabs
- `src/components/DevTools.module.css` — terminal-themed styles

## Rust Backend

### Print Capture
- `TabLuaState.print_log: Arc<Mutex<Vec<String>>>` captures `print()` calls
- `run_handler()` drains and returns `print_output: Vec<String>`
- `NtmlRunHandlerResult` now includes `print_output`

### Storage API
- **Lua**: `storage.set(k, v)`, `storage.get(k)`, `storage.remove(k)`, `storage.clear()`, `storage.keys()`
- **Rust state**: `BrowserStorageStore = Arc<DashMap<String, HashMap<String, String>>>` (origin → kv map)
- **Tauri commands**: `browser_storage_get_all`, `browser_storage_set`, `browser_storage_delete`, `browser_storage_clear`

### HTTP API (future)
- Lua `http.get(url)`, `http.post(url, body)` — planned, not yet implemented
- When added: results emitted as `devtools-network` Tauri event with `origin: "lua"`

## Key Files

| File | Purpose |
|------|---------|
| `src/contexts/DevToolsContext.tsx` | Shared DevTools state |
| `src/components/DevTools.tsx` | DevTools UI component |
| `src/components/DevTools.module.css` | Styles |
| `src/contexts/WindowManagerContext.tsx` | `"devtools"` window type |
| `src/screens/Desktop.tsx` | Provider + rendering |
| `src/components/Browser.tsx` | F12, network tracking, source, console |
| `src-tauri/src/ntml_runtime.rs` | print_log, storage Lua API |
| `src-tauri/src/lib.rs` | Commands + NtmlRunHandlerResult |
