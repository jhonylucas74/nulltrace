# HTTP-like Protocol for VM Communication

This document describes the HTTP-like protocol used for communication between VMs (or within the same VM via loopback) in nulltrace. The protocol is text-based, similar to HTTP/1.0, and is designed to work over the game's existing network layer.

---

## Overview

The HTTP protocol layer provides:

- **Request/response structure** — Clients send requests with a method, path, and optional body; servers respond with a status code and body.
- **Rust utilities** — Build and parse requests/responses in Rust (`http_proto` module).
- **Lua API** — Build and parse from Lua scripts (`http` table), enabling HTTP servers and clients over `net.send`, `net.connect`, and `net.recv`.

The underlying transport is the game's packet-based network: messages are sent as raw bytes over TCP-like packets. The HTTP layer adds structure on top of that.

---

## Localhost and Loopback

The network resolves `localhost` and `127.0.0.x` addresses to the **same machine** (the sender VM).

### Host resolution

When calling `net.send(host, port, data)` or `net.connect(host, port)`:

- `"localhost"` → resolved to `127.0.0.1`
- `"127.0.0.1"`, `"127.1.1.1"`, or any `127.x.x.x` → accepted as loopback

### Loopback delivery

Packets destined for a loopback address (`127.x.x.x`) are **not routed** through the normal router. Instead, they are delivered directly to the **sender VM's NIC** — as if the packet never left the machine. This allows a VM to run both a server and a client process that communicate via `localhost`.

---

## Protocol Format

### Request

```
METHOD /path HTTP/1.0\r\n
Header-Name: value\r\n
\r\n
[optional body]
```

Example:

```
GET / HTTP/1.0\r\n
Host: localhost\r\n
\r\n
```

### Response

```
HTTP/1.0 STATUS_CODE Reason-Phrase\r\n
Header-Name: value\r\n
\r\n
[optional body]
```

Example:

```
HTTP/1.0 200 OK\r\n
Content-Length: 12\r\n
\r\n
Hello NTML
```

### Supported methods

- GET, POST, PUT, PATCH, DELETE, HEAD

---

## Rust API

> File: `src/cluster/net/http_proto.rs`

### Building requests

```rust
use crate::net::http_proto::{HttpRequest, HttpMethod};

let req = HttpRequest::get("/");
let bytes = req.to_bytes();

let req = HttpRequest::post("/api", b"{\"key\":\"value\"}");
let bytes = req.to_bytes();
```

### Building responses

```rust
use crate::net::http_proto::HttpResponse;

let res = HttpResponse::ok(b"Hello");
let bytes = res.to_bytes();

let res = HttpResponse::not_found();
let bytes = res.to_bytes();
```

### Parsing

```rust
use crate::net::http_proto::{parse_http_request, parse_http_response};

let req = parse_http_request(raw_bytes)?;
// req.method, req.path, req.headers, req.body

let res = parse_http_response(raw_bytes)?;
// res.status_code, res.reason_phrase, res.headers, res.body
```

---

## Lua API

The `http` table is available in Lua scripts (when using the full VM Lua state).

### http.build_request(method, path, body?)

Builds an HTTP request and returns the raw string (bytes).

- `method` — "GET", "POST", "PUT", "PATCH", "DELETE", "HEAD"
- `path` — e.g. "/" or "/api/users"
- `body` — optional string (nil for GET, DELETE, HEAD)

```lua
local req = http.build_request("GET", "/", nil)
local req = http.build_request("POST", "/api", "{\"x\":1}")
```

### http.parse_request(data)

Parses an HTTP request. Returns a table: `{ method, path, headers, body }`.

```lua
local r = net.recv()
if r then
  local req = http.parse_request(r.data)
  if req then
    -- req.method, req.path, req.headers, req.body
  end
end
```

### http.build_response(status, body?)

Builds an HTTP response. Returns the raw string (bytes).

- `status` — e.g. 200, 404
- `body` — optional string

```lua
local res = http.build_response(200, "Hello NTML")
local res = http.build_response(404)
```

### http.parse_response(data)

Parses an HTTP response. Returns a table: `{ status, reason, headers, body }`.

```lua
local r = conn:recv()
if r then
  local res = http.parse_response(r.data)
  if res and res.status == 200 then
    io.write(res.body)
  end
end
```

---

## Example: HTTP Server and Client

### Server (Lua)

```lua
net.listen(80)
while true do
  local r = net.recv()
  if r then
    local req = http.parse_request(r.data)
    if req and req.path == "/" then
      local res = http.build_response(200, "Hello NTML")
      net.send(r.src_ip, r.src_port, res)
    end
  end
end
```

### Client (Lua)

```lua
local req = http.build_request("GET", "/", nil)
local conn = net.connect("10.0.1.2", 80)  -- or "localhost" for loopback
conn:send(req)
while true do
  local r = conn:recv()
  if r then
    local res = http.parse_response(r.data)
    if res and res.status == 200 then
      io.write(res.body)
    end
    conn:close()
    break
  end
end
```

---

## Tests

Integration tests are in `nulltrace-core/src/cluster/vm_manager.rs`:

- **test_http_protocol_two_vms** — Two VMs: server on port 80, client sends GET / and receives "Hello NTML".
- **test_http_protocol_loopback** — Single VM: server and client on same machine; client connects to `localhost:80` and receives "Hello loopback".

Run with:

```bash
cargo test test_http_protocol --no-fail-fast
```
