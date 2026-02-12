# Lua script examples

This document shows practical Lua examples for scripts that run inside nulltrace VMs. The VM exposes **net**, **os**, **io**, and **fs** APIs. Execution is **tick-based**: use loops when waiting for I/O so the process stays alive across ticks.

---

## 1. Basic I/O and arguments

Read program arguments and write to stdout:

```lua
local args = os.get_args()
for i = 1, #args do
    io.write("arg[" .. i .. "] = " .. args[i] .. "\n")
end
```

Read one line from stdin and echo it:

```lua
local line = io.read()
if line and line ~= "" then
    io.write("got: " .. line .. "\n")
end
```

---

## 2. File system

List a directory (default `.`):

```lua
local args = os.get_args()
local path = (#args >= 1) and args[1] or "."
local entries = fs.ls(path)
for i = 1, #entries do
    local e = entries[i]
    io.write(e.name .. "\t" .. e.type .. "\t" .. tostring(e.size) .. "\t" .. e.owner .. "\n")
end
```

Read a file and write to stdout:

```lua
local path = (os.get_args()[1]) or "/etc/hostname"
local content = fs.read(path)
if content then
    io.write(content)
else
    io.write("file not found: " .. path .. "\n")
end
```

Write a file and create a directory:

```lua
fs.mkdir("/tmp/mydir")
fs.write("/tmp/mydir/hello.txt", "Hello, VM!\n", nil)
```

Remove a file:

```lua
local path = os.get_args()[1]
if path then
    fs.rm(path)
end
```

---

## 3. Process and OS info

Print hostname, PID, and current user:

```lua
io.write("hostname: " .. os.hostname() .. "\n")
io.write("pid: " .. tostring(os.pid()) .. "\n")
io.write("user: " .. os.whoami() .. " (uid " .. tostring(os.uid()) .. ")\n")
io.write("vm_id: " .. os.vm_id() .. "\n")
if os.is_root() then
    io.write("running as root\n")
end
```

Spawn a child and read its stdout:

```lua
local pid = os.spawn("echo", {"hello", "world"}, {})
while true do
    local st = os.process_status(pid)
    if st == "finished" or st == "not_found" then
        break
    end
end
local out = os.read_stdout(pid)
if out then
    io.write("child wrote: " .. out .. "\n")
end
```

Inject stdin into a child (e.g. a shell):

```lua
local pid = os.spawn("sh", {}, { forward_stdout = true })
os.write_stdin(pid, "echo from child\n")
-- Child stdout is forwarded to this process when forward_stdout = true
```

---

## 4. Network — client (connection API)

Connect to a server, send a request, and read the response. No `net.listen(0)` needed; the stack allocates an ephemeral port for the connection.

```lua
local server_ip = "10.0.1.3"   -- target VM's IP
local port = 7777

local conn = net.connect(server_ip, port)
conn:send("ping")

while true do
    local r = conn:recv()
    if r then
        io.write("response: " .. r.data .. "\n")
        conn:close()
        break
    end
end
```

Multiple requests on the same connection:

```lua
local conn = net.connect(server_ip, 9999)
conn:send("req1")
local n = 0
while true do
    local r = conn:recv()
    if r then
        io.write(r.data)
        n = n + 1
        if n == 4 then
            conn:send("req2")
        end
    end
end
```

---

## 5. Network — server (listen + recv)

Listen on a port and reply to each request. Use `net.recv()` in a loop; packets arrive in later ticks.

```lua
net.listen(8080)
while true do
    local r = net.recv()
    if r then
        net.send(r.src_ip, r.src_port, "echo: " .. r.data)
    end
end
```

Reply with the same payload (echo server):

```lua
net.listen(80)
while true do
    local r = net.recv()
    if r then
        net.send(r.src_ip, r.src_port, r.data)
    end
end
```

---

## 6. Request–response (client + server)

**Server (VM B):** listen and respond with a fixed message.

```lua
net.listen(9999)
while true do
    local r = net.recv()
    if r then
        net.send(r.src_ip, r.src_port, "pong")
    end
end
```

**Client (VM A):** connect, send, and read the response.

```lua
local conn = net.connect("10.0.1.3", 9999)   -- B's IP
conn:send("ping")
while true do
    local r = conn:recv()
    if r then
        io.write(r.data)   -- "pong"
        conn:close()
        break
    end
end
```

---

## 7. Get this VM's IP

Useful when passing the IP to another script or logging:

```lua
local ip = net.ip()
if ip then
    io.write("my IP: " .. ip .. "\n")
else
    io.write("no IP (no NIC)\n")
end
```

---

## 8. Low-level send (no connection)

For one-off packets or when you do not need a connection object (e.g. server replying with `net.send(r.src_ip, r.src_port, data)`):

```lua
-- Send a single packet to host:port (src_port will be 0; prefer net.connect for clients)
net.send("10.0.1.2", 80, "hello")
```

---

## Notes

- **Tick-based execution:** A single `net.recv()` or `conn:recv()` often returns `nil` until the next tick. Use `while true do ... end` and check `if r then ...` so the process keeps running until data arrives.
- **Connection API:** Prefer `net.connect(host, port)` and `conn:send` / `conn:recv` / `conn:close` for clients. Ephemeral ports are allocated automatically and released on `conn:close()` or process exit.
- **Server replies:** Always reply with `net.send(r.src_ip, r.src_port, data)` so the packet reaches the sender (or the sender’s connection).
- **Stdin:** Use a loop when reading stdin; `io.read()` may return `nil` until input is injected in a later tick.
