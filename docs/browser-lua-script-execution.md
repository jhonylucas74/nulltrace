# In-Game Browser Lua Script Execution — Architecture Change

This document describes the changes to how the in-game Browser executes Lua scripts. The system moved from a **client-side (local) execution model** to a **server-side (cluster) execution model** with a bidirectional gRPC stream.

---

## Summary of Changes

| Aspect | Before (Legacy) | After (New) |
|--------|-----------------|-------------|
| **Execution location** | Client (Tauri) via `ntml_run_handler` | Server (cluster VM) via gRPC stream |
| **Script storage** | Tab state in client (`ntml_create_tab_state`) | Lua code sent to cluster on tab load |
| **Event handling** | `ntml_run_handler` with tabId, action, formValues | `browser_process_send_event` with JSON event payload |
| **Output** | HTML returned from handler | JSON lines streamed over gRPC (`patch`, `print`, `navigate`, etc.) |
| **HTTP requests** | Client-side fetch | Proxied via cluster `curl` (`browser_http_request`) |
| **Card requests** | `ntml_submit_card_selection` + `ntml_run_handler` | `browser_process_send_event` with card data |

---

## Step-by-Step Architecture

### 1. Proto Definition (game.proto)

A new bidirectional gRPC RPC was added:

```protobuf
rpc BrowserProcessStream (stream BrowserProcessClientMessage)
    returns (stream BrowserProcessServerMessage);
```

**Client → Server messages:**
- `OpenBrowserProcess` — First message; contains `lua_code` to execute.
- `BrowserProcessEvent` — Subsequent messages; JSON lines (events, HTTP responses).

**Server → Client messages:**
- `BrowserProcessOpened` — Session ready; contains `session_id` and `pid`.
- `BrowserProcessOutput` — JSON line from process stdout.
- `BrowserProcessClosed` — Process ended normally.
- `BrowserProcessError` — Error message (e.g. memory limit, spawn failure).

---

### 2. Server-Side: Browser Process Hub

**New file:** `nulltrace-core/src/cluster/browser_process_hub.rs`

A hub similar to `TerminalHub` that bridges gRPC connections to the game loop:

- **`pending_spawns`** — Requests to spawn a browser process; drained each game tick.
- **`sessions`** — Active sessions (session_id → `BrowserSession`) with channels for stdin/stdout/error.
- **`pending_kills`** — (vm_id, pid) to kill when a session disconnects.

**`BrowserSessionReady`** is sent back to the gRPC handler when the process is spawned; it includes:
- `session_id`, `vm_id`, `pid`
- `stdout_rx` — Game loop sends JSON lines here.
- `stdin_tx` — gRPC handler sends events here; game loop injects into process stdin.
- `error_rx` — Game loop sends errors (memory limit, process terminated).

---

### 3. Server-Side: Restricted Lua Environment

**New file:** `nulltrace-core/src/cluster/lua_api/browser_env.rs`

Browser processes run in a **sandboxed environment** instead of full VM globals:

**Exposed:**
- Safe Lua builtins: `math`, `string`, `table`, `type`, `tostring`, `tonumber`, `pairs`, `ipairs`, `pcall`, `xpcall`, `error`, `assert`, etc.
- `io.read` / `io.write` — Routed through `VmContext` stdin/stdout.
- `print` — Writes `{"type":"print","message":"..."}` JSON line to stdout.
- `ui.*` — Patch operations (`set_text`, `set_visible`, `set_class`, etc.) as JSON lines.
- `browser.request_card(origin, callback)` — Writes `request_card` JSON line.
- `json_encode` / `json_decode` — Via serde_json.
- `str` table — If registered globally.

**Excluded (security):**
- `fs`, `net`, `os`, `mail`, `fkebank`, `crypto`, `load`, `dofile`, `require`, etc.

**Event loop injection:** `BROWSER_WHILE_LOOP_INJECTION` is appended to user code. It:
- Defines `http.*` (get, post, put, patch, delete) — Writes `http_request` JSON to stdout, reads `http_response` from stdin.
- Runs `while true do ... io.read() ...` — Dispatches events to `_ENV[action]` handlers.

---

### 4. Server-Side: Process Spawning

**Modified:** `nulltrace-core/src/cluster/process.rs`

- **`Process::new_with_env`** — Creates a process whose Lua code runs in an explicit environment table (`lua.load(code).set_environment(env)`). Used for browser processes.

**Modified:** `nulltrace-core/src/cluster/os.rs`

- **`OS::spawn_browser_process`** — Spawns a process with `display_name = Some("[browser]")` and the restricted env.

---

### 5. Server-Side: Game Loop Integration

**Modified:** `nulltrace-core/src/cluster/vm_manager.rs`

**Browser process hub handling (each tick):**

1. **Drain `pending_spawns`:**
   - Resolve player's VM.
   - Build restricted env via `browser_env::create_browser_env`.
   - Append `BROWSER_WHILE_LOOP_INJECTION` to user code.
   - Spawn process with `spawn_browser_process`.
   - Create `BrowserSession` with stdout/stdin/error channels.
   - Send `BrowserSessionReady` to gRPC handler.

2. **Drain `pending_kills`:**
   - Kill process and descendants when session ends.

3. **Per-session I/O:**
   - Drain `stdin_rx` → inject into process stdin via `push_stdin_line`.
   - Drain process stdout → send newline-delimited JSON lines on `stdout_tx`.
   - If process finished → remove session, send error, kill process.

4. **Memory exceeded:**
   - Notify browser sessions for affected VMs before reset; send error and remove sessions.

---

### 6. Server-Side: gRPC Handler

**Modified:** `nulltrace-core/src/cluster/grpc.rs`

**`browser_process_stream`** implementation:

1. Authenticate request.
2. Wait for first message: `OpenBrowserProcess` with `lua_code`.
3. Push `(player_id, lua_code, response_tx)` into `browser_process_hub.pending_spawns`.
4. Wait (up to 10s) for `BrowserSessionReady` from game loop.
5. Send `BrowserProcessOpened` to client.
6. Spawn task: forward `stdout_rx` → `BrowserProcessOutput` messages.
7. Spawn task: read `BrowserProcessEvent` from client → send to `stdin_tx`.
8. On client disconnect: push `pending_kills`, remove session.

**Modified:** `nulltrace-core/src/server/main.rs`

- Standalone server: `browser_process_stream` returns an empty stream (cluster binary is required for browser processes).

---

### 7. Client-Side: Tauri Commands

**Modified:** `nulltrace-client/src-tauri/src/grpc.rs`

**New commands:**

- **`browser_process_connect(token, lua_code)`** — Opens gRPC stream, sends `OpenBrowserProcess`, waits for `BrowserProcessOpened`, returns `session_id`. Spawns background task that:
  - Listens for `BrowserProcessOutput` → emits `browser-process-output` event.
  - Listens for `BrowserProcessEvent` requests → sends to gRPC stream.
- **`browser_process_send_event(session_id, json_line)`** — Sends JSON event to process stdin.
- **`browser_process_disconnect(session_id)`** — Removes session; dropping stdin sender closes the stream.
- **`browser_http_request(token, method, url, body, headers)`** — Proxies HTTP via cluster `curl`; returns `{ status, body }`.

**State:** `BrowserProcessSessionsState` — `HashMap<session_id, Sender<String>>` for stdin events.

---

### 8. Client-Side: Browser Component

**Modified:** `nulltrace-client/src/components/Browser.tsx`

**Tab load (fetchVmUrl):**

- If `scriptSources.length > 0` and `token`:
  - Concatenate scripts into `luaCode`.
  - Call `browser_process_connect(token, luaCode)`.
  - Store `tabId → sessionId` in `tabSessions` ref.
- Else (no scripts):
  - Use legacy `ntml_create_tab_state`.

**Event handling (ntml:handler):**

- If `tabSessions.current.get(activeTabId)` exists:
  - Build JSON: `{ type: "event", action, form_values, event_data }`.
  - Call `browser_process_send_event(sessionId, jsonLine)`.
- Else:
  - Use legacy `ntml_run_handler`.

**New listener: `browser-process-output`**

Handles JSON lines from the server:

| `msg.type` | Action |
|------------|--------|
| `patch` | Apply DOM patch (set_text, set_visible, set_class, etc.) to iframe |
| `print` | Push to DevTools console |
| `request_card` | Open card picker modal; store `sessionId` in modal state |
| `navigate` | Navigate (same tab or new tab) |
| `http_request` | Call `browser_http_request`, then send `http_response` JSON back via `browser_process_send_event` |

**Card picker confirm:**

- If `modalSessionId`:
  - Send card data as event via `browser_process_send_event`.
- Else:
  - Use legacy `ntml_submit_card_selection` + `ntml_run_handler`.

**Tab close:**

- If tab has `sessionId`: call `browser_process_disconnect`, remove from `tabSessions`.

---

## JSON Message Protocol

### Client → Process (stdin)

```json
{"type":"event","action":"on_click","form_values":{},"event_data":{"id":"btn1"}}
```

### Process → Client (stdout)

| type | Fields | Purpose |
|------|--------|---------|
| `patch` | `op`, `id`, `text`/`visible`/`class`/`value`/`disabled` | DOM update |
| `print` | `message` | Console log |
| `request_card` | `request_id`, `origin`, `callback` | Card picker |
| `navigate` | `url`, `target` | Navigate same/new tab |
| `http_request` | `id`, `method`, `url`, `body`, `headers` | HTTP proxy request |

### Client → Process (http_response)

```json
{"type":"http_response","id":"1","status":200,"body":"...","headers":{}}
```

---

## Backward Compatibility

- **Tabs without scripts** — Still use `ntml_create_tab_state` and `ntml_run_handler`.
- **Card flow without sessionId** — Still use `ntml_submit_card_selection` + `ntml_run_handler`.
- The Browser component branches on `sessionId` / `tabSessions` to choose the flow.

---

## File Summary

| File | Change |
|------|--------|
| `nulltrace-client/proto/game.proto` | Added `BrowserProcessStream` RPC and messages |
| `nulltrace-core/proto/game.proto` | Same |
| `nulltrace-client/src-tauri/src/grpc.rs` | `browser_process_connect`, `send_event`, `disconnect`, `browser_http_request` |
| `nulltrace-client/src-tauri/src/lib.rs` | Registered new commands and state |
| `nulltrace-client/src/components/Browser.tsx` | Server-side flow, `tabSessions`, `browser-process-output` listener |
| `nulltrace-core/src/cluster/grpc.rs` | `browser_process_stream` handler |
| `nulltrace-core/src/cluster/main.rs` | `browser_process_hub` creation and wiring |
| `nulltrace-core/src/cluster/browser_process_hub.rs` | **New** — Hub for sessions |
| `nulltrace-core/src/cluster/lua_api/browser_env.rs` | **New** — Sandboxed env and event loop |
| `nulltrace-core/src/cluster/lua_api/mod.rs` | Export `browser_env` |
| `nulltrace-core/src/cluster/os.rs` | `spawn_browser_process` |
| `nulltrace-core/src/cluster/process.rs` | `Process::new_with_env` |
| `nulltrace-core/src/cluster/vm_manager.rs` | Hub handling in game loop |
| `nulltrace-core/src/server/main.rs` | Stub `browser_process_stream` (empty stream) |
