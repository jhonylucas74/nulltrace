//! Lua source for basic /bin programs.
//! Each program uses os.get_args() and io.write/fs.* as needed.

/// Echo: prints args joined by space, newline at end.
pub const ECHO: &str = r#"
local args = os.get_args()
local out = {}
for i = 1, #args do
    out[#out + 1] = args[i]
end
io.write(table.concat(out, " ") .. "\n")
"#;

/// Cat: reads each file path from args and writes content to stdout.
pub const CAT: &str = r#"
local args = os.get_args()
for i = 1, #args do
    local path = args[i]
    local content = fs.read(path)
    if content then
        io.write(content)
    end
end
"#;

/// Ls: lists directory entries. Args: path (default ".")
pub const LS: &str = r#"
local args = os.get_args()
local path = (#args >= 1) and args[1] or "."
local entries = fs.ls(path)
for i = 1, #entries do
    local e = entries[i]
    local line = e.name .. "\t" .. e.type .. "\t" .. tostring(e.size) .. "\t" .. e.owner .. "\n"
    io.write(line)
end
"#;

/// Touch: creates empty file at each path in args.
pub const TOUCH: &str = r#"
local args = os.get_args()
for i = 1, #args do
    fs.write(args[i], "", nil)
end
"#;

/// Rm: removes each path in args.
pub const RM: &str = r#"
local args = os.get_args()
for i = 1, #args do
    fs.rm(args[i])
end
"#;

/// Shell: reads stdin, parses as bin command (no spawn_path). When no child: spawn(program, args, { forward_stdout = true }).
/// When has child: forwards stdin to child. Child stdout is forwarded to shell natively by the VM.
pub const SH: &str = r#"
local child_pid = nil
while true do
  if child_pid then
    local st = os.process_status(child_pid)
    if st == "finished" or st == "not_found" then child_pid = nil end
  end
  local line = io.read()
  if line and line ~= "" then
    if child_pid then
      os.write_stdin(child_pid, line)
    else
      local t = os.parse_cmd(line)
      if t and t.program and t.program ~= "" then
        child_pid = os.spawn(t.program, t.args or {}, { forward_stdout = true })
      end
    end
  end
end
"#;

/// Echo stdin: reads one line from stdin and writes "got:" .. line (for shell forward-stdin tests).
pub const ECHO_STDIN: &str = r#"
while true do
  local l = io.read()
  if l and l ~= "" then
    io.write("got:" .. l)
    break
  end
end
"#;

/// SSH client: connects to a host on port 22, forwards stdin to the remote shell and stdout from it.
/// Usage: ssh [user@]host. Uses connection API (ephemeral port); no net.listen(0).
pub const SSH: &str = r#"
local args = os.get_args()
local server_arg = (args and args[1]) and args[1] or ""
if server_arg == "" then
  io.write("ssh: usage: ssh [user@]host\n")
  return
end
local host = server_arg
local at = string.find(server_arg, "@")
if at then
  host = string.sub(server_arg, at + 1)
end
local conn = net.connect(host, 22)
while true do
  local line = io.read()
  if line and line ~= "" then
    conn:send(line)
  end
  local r = conn:recv()
  if r then
    io.write(r.data)
  end
end
"#;

/// SSH server (daemon): listens on port 22, spawns one shell per client, routes stdin/stdout via network.
pub const SSH_SERVER: &str = r#"
net.listen(22)
local connections = {}
while true do
  local pkt = net.recv()
  if pkt and (tonumber(pkt.dst_port) or 0) == 22 then
    local key = pkt.src_ip .. ":" .. tostring(pkt.src_port)
    if not connections[key] then
      local pid = os.spawn("sh", {}, {})
      connections[key] = {
        pid = pid,
        src_ip = pkt.src_ip,
        src_port = pkt.src_port,
        last_sent = 0,
      }
    end
    if pkt.data and pkt.data ~= "" then
      os.write_stdin(connections[key].pid, pkt.data)
    end
  end
  local to_remove = {}
  for key, conn in pairs(connections) do
    local st = os.process_status(conn.pid)
    if st == "finished" or st == "not_found" then
      to_remove[#to_remove + 1] = key
    else
      local out = os.read_stdout(conn.pid)
      if out and #out > conn.last_sent then
        local data = out:sub(conn.last_sent + 1)
        net.send(conn.src_ip, conn.src_port, data)
        conn.last_sent = #out
      end
    end
  end
  for i = 1, #to_remove do
    connections[to_remove[i]] = nil
  end
end
"#;

/// Programs to include when bootstrapping a new VM. User can delete them later.
pub const DEFAULT_BIN_PROGRAMS: &[(&str, &str)] = &[
    ("cat", CAT),
    ("echo", ECHO),
    ("echo_stdin", ECHO_STDIN),
    ("ls", LS),
    ("rm", RM),
    ("sh", SH),
    ("ssh", SSH),
    ("ssh-server", SSH_SERVER),
    ("touch", TOUCH),
];
