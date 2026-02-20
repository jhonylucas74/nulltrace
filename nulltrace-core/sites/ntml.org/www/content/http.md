# HTTP protocol (Lua)

The http table builds and parses raw HTTP/1.1 messages. Use with net.send/recv for building in-VM servers and clients. This is the low-level API - for UI scripts, use http.get/post in the Lua API instead.

---

## Functions

```lua
-- Build raw HTTP request string
http.build_request(method, path, body?)
-- Returns: string

-- Parse raw HTTP request string
http.parse_request(data)
-- Returns: { method, path, headers, body }

-- Build raw HTTP response string
http.build_response(status, body?)
-- Returns: string

-- Parse raw HTTP response string
http.parse_response(data)
-- Returns: { status, reason, headers, body }
```

---

## Server pattern

Listen on a port, receive packets, parse requests, and send responses. Runs inside a VM bootstrap script.

```lua
net.listen(80)
while true do
  local pkt = net.recv()
  if pkt and pkt.dst_port == 80 then
    local req = http.parse_request(pkt.data)
    local body = "Hello from " .. req.path
    local res = http.build_response(200, body)
    net.send(pkt.src_ip, pkt.src_port, res)
  end
end
```

---

## Client pattern

```lua
local host = "10.0.1.50"
local conn = net.connect(host, 80)
conn:send(http.build_request("GET", "/api/status", nil))
while true do
  local r = conn:recv()
  if r then
    local res = http.parse_response(r.data)
    print(res.status, res.body)
    break
  end
end
conn:close()
```

localhost and 127.x.x.x resolve to loopback (same VM). Use net.connect for cross-VM calls.
