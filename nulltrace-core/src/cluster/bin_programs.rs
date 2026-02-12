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

/// Shell: reads stdin, parses as bin command (no spawn_path). Maintains cwd; cd and pwd are builtins.
/// Commands ls, touch, cat, rm get path args resolved against cwd. Unknown commands print "<red>Command not found</red>".
pub const SH: &str = r#"
local child_pid = nil
local last_program = nil
local last_not_found_pid = nil
local child_ever_ran = false
local cwd = os.get_home() or "/"
while true do
  if child_pid then
    local st = os.process_status(child_pid)
    if st == "not_found" then
      -- Process was reaped after exit: do not print "Command not found".
      if child_ever_ran then
        child_pid = nil
        last_not_found_pid = nil
        child_ever_ran = false
      -- Same tick as spawn the child is not created yet; wait one more tick.
      elseif last_not_found_pid == child_pid then
        io.write("<red>Command not found: " .. (last_program or "?") .. "</red>\n")
        child_pid = nil
        last_not_found_pid = nil
      else
        last_not_found_pid = child_pid
      end
    elseif st == "finished" then
      child_pid = nil
      last_not_found_pid = nil
      child_ever_ran = false
    else
      child_ever_ran = true
      last_not_found_pid = nil
    end
  end
  local line = io.read()
  if line and line ~= "" then
    if child_pid then
      os.write_stdin(child_pid, line)
    else
      local t = os.parse_cmd(line)
      if t and t.program and t.program ~= "" then
        local prog = t.program
        local args = t.args or {}
        if prog == "pwd" then
          io.write(cwd .. "\n")
        elseif prog == "cd" then
          if #args < 1 then
            cwd = os.get_home() or "/"
          else
            local arg1 = args[1]
            local resolved = os.path_resolve(cwd, arg1)
            local st = fs.stat(resolved)
            if not st then
              io.write("<red>cd: no such file or directory: " .. arg1 .. "</red>\n")
            elseif st.type ~= "directory" then
              io.write("<red>cd: not a directory: " .. arg1 .. "</red>\n")
            else
              cwd = resolved
            end
          end
        else
          local spawn_args = args
          if prog == "ls" or prog == "touch" or prog == "cat" or prog == "rm" then
            spawn_args = {}
            if prog == "ls" and #args < 1 then
              spawn_args[1] = cwd
            else
              for i = 1, #args do
                local p = args[i]
                spawn_args[i] = (p:sub(1, 1) == "/") and p or os.path_resolve(cwd, p)
              end
            end
          end
          last_program = prog
          last_not_found_pid = nil
          child_ever_ran = false
          child_pid = os.spawn(prog, spawn_args, { forward_stdout = true })
        end
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
