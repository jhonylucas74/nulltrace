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
/// Columns are aligned by computing max widths from all entries, then padding each field.
pub const LS: &str = r#"
local args = os.get_args()
local path = (#args >= 1) and args[1] or "."
local entries = fs.ls(path)

if #entries == 0 then
  return
end

-- Minimum column widths for a single entry or short names
local min_name, min_type, min_size, min_owner = 8, 9, 6, 4
local w_name, w_type, w_size, w_owner = min_name, min_type, min_size, min_owner

for i = 1, #entries do
  local e = entries[i]
  local size_str = tostring(e.size)
  if #e.name > w_name then w_name = #e.name end
  if #e.type > w_type then w_type = #e.type end
  if #size_str > w_size then w_size = #size_str end
  if #e.owner > w_owner then w_owner = #e.owner end
end

local function pad_right(s, width)
  return s .. string.rep(" ", math.max(0, width - #s))
end
local function pad_left(s, width)
  return string.rep(" ", math.max(0, width - #s)) .. s
end

for i = 1, #entries do
  local e = entries[i]
  local name_padded = pad_right(e.name, w_name)
  local type_padded = pad_right(e.type, w_type)
  local size_padded = pad_left(tostring(e.size), w_size)
  io.write(name_padded .. "  " .. type_padded .. "  " .. size_padded .. "  " .. e.owner .. "\n")
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

/// Mem-stress: allocates memory gradually to test the 1 MB Lua limit.
/// Args: [chunk_kb] (default 4). Each iteration allocates chunk_kb KB.
/// Prints progress every 10 chunks. When limit is hit, VM resets (MemoryError).
pub const MEM_STRESS: &str = r#"
local args = os.get_args()
local chunk_kb = 4
if #args >= 1 then
  chunk_kb = tonumber(args[1]) or 4
end
if chunk_kb < 1 then chunk_kb = 1 end
if chunk_kb > 64 then chunk_kb = 64 end

local chunk_size = chunk_kb * 1024
local t = {}
local iter = 0

io.write("mem_stress: chunk=" .. chunk_kb .. " KB, limit=1 MB. Allocating...\n")

while true do
  iter = iter + 1
  t[iter] = string.rep("x", chunk_size)
  if iter % 10 == 0 then
    local kb = iter * chunk_kb
    io.write("mem_stress: " .. iter .. " chunks (~" .. kb .. " KB)\n")
  end
end
"#;

/// Coin flip game: loop, store results in table, print probability.
/// Args: [max_history] (default 1000). Bounded history keeps memory under 1 MB.
pub const COIN: &str = r#"
local args = os.get_args()
local max_history = 1000
if #args >= 1 then
  max_history = tonumber(args[1]) or 1000
end
if max_history < 1 then max_history = 1 end
if max_history > 10000 then max_history = 10000 end

local heads, tails = 0, 0
local history = {}

io.write("coin: flipping (max_history=" .. max_history .. "). Press Ctrl+C to stop.\n")

while true do
  local flip = (math.random(1, 2) == 1) and "heads" or "tails"
  if flip == "heads" then heads = heads + 1 else tails = tails + 1 end

  history[#history + 1] = flip
  if #history > max_history then
    table.remove(history, 1)
  end

  local total = heads + tails
  local p_heads = (heads / total) * 100
  local p_tails = (tails / total) * 100
  io.write("Flip " .. total .. ": " .. flip .. ". P(heads)=" .. string.format("%.1f", p_heads) .. "% P(tails)=" .. string.format("%.1f", p_tails) .. "%\n")
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
        os.clear_foreground_pid()
        child_pid = nil
        last_not_found_pid = nil
        child_ever_ran = false
      -- Same tick as spawn the child is not created yet; wait one more tick.
      elseif last_not_found_pid == child_pid then
        io.write("<red>Command not found: " .. (last_program or "?") .. "</red>\n")
        os.clear_foreground_pid()
        child_pid = nil
        last_not_found_pid = nil
      else
        last_not_found_pid = child_pid
      end
    elseif st == "finished" then
      os.clear_foreground_pid()
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
      local action = os.handle_special_stdin(line, child_pid)
      if action == "kill_child" then
        os.request_kill(child_pid)
        os.clear_foreground_pid()
        child_pid = nil
      elseif action == "forward" then
        os.write_stdin(child_pid, line)
      elseif action == "pass" then
        os.write_stdin(child_pid, line)
      end
      -- "discard": do not forward (e.g. Tab when child is not ssh)
    else
      -- Tab autocomplete: line contains \x09; Rust resolves from cwd and /bin; protocol \x01TABCOMPLETE\t + replacement
      -- pcall so any error from os.autocomplete (Rust or Lua) never kills the shell process.
      -- Use pure-Lua for tab detection and stripping to avoid "attempt to yield across metamethod/C-call boundary" in Luau.
      local function line_has_tab(s)
        for i = 1, #s do if s:sub(i, i) == "\x09" then return true end end
        return false
      end
      local is_tab = (line == "\x09") or line_has_tab(line)
      if is_tab then
        local ok, replacement = pcall(function() return os.autocomplete(line, cwd) end)
        if ok and replacement and type(replacement) == "string" and replacement ~= "" then
          io.write("\x01TABCOMPLETE\t" .. replacement .. "\n")
        end
        -- When no completion: do not send TABCOMPLETE so the terminal UI leaves the input unchanged.
      elseif line ~= "\x03" then
      -- Ignore Ctrl+C (\x03) when no foreground child
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
          os.set_foreground_pid(child_pid)
        end
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
    ("coin", COIN),
    ("echo", ECHO),
    ("echo_stdin", ECHO_STDIN),
    ("ls", LS),
    ("mem_stress", MEM_STRESS),
    ("rm", RM),
    ("sh", SH),
    ("ssh", SSH),
    ("ssh-server", SSH_SERVER),
    ("touch", TOUCH),
];
