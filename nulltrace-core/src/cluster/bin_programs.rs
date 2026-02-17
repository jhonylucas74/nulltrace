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
/// Resolves relative paths with os.get_work_dir.
pub const CAT: &str = r#"
local args = os.get_args()
for i = 1, #args do
    local path = args[i]
    if path:sub(1, 1) ~= "/" then
      path = os.path_resolve(os.get_work_dir(), path)
    end
    local st = fs.stat(path)
    if not st then
        io.write("<red>cat: " .. path .. ": No such file or directory</red>\n")
    elseif st.type == "directory" then
        io.write("<red>cat: " .. path .. ": Is a directory</red>\n")
    else
        local content = fs.read(path)
        if content then
            io.write(content)
        else
            io.write("<red>cat: " .. path .. ": Cannot read file</red>\n")
        end
    end
end
"#;

/// Ls: lists directory entries. Args: path (default current work dir).
/// Resolves relative paths with os.get_work_dir. Output is written in one go.
pub const LS: &str = r#"
local args = os.get_args()
local path = (#args >= 1) and args[1] or os.get_work_dir()
if path:sub(1, 1) ~= "/" then
  path = os.path_resolve(os.get_work_dir(), path)
end
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

local lines = {}
for i = 1, #entries do
  local e = entries[i]
  local name_padded = pad_right(e.name, w_name)
  local type_padded = pad_right(e.type, w_type)
  local size_padded = pad_left(tostring(e.size), w_size)
  lines[#lines + 1] = name_padded .. "  " .. type_padded .. "  " .. size_padded .. "  " .. e.owner
end
io.write(table.concat(lines, "\n") .. "\n")
"#;

/// Touch: creates empty file at each path in args. Resolves relative paths with os.get_work_dir.
pub const TOUCH: &str = r#"
local args = os.get_args()
for i = 1, #args do
    local path = args[i]
    if path:sub(1, 1) ~= "/" then
      path = os.path_resolve(os.get_work_dir(), path)
    end
    fs.write(path, "", nil)
end
"#;

/// Rm: removes each path in args. Resolves relative paths with os.get_work_dir.
pub const RM: &str = r#"
local args = os.get_args()
for i = 1, #args do
    local path = args[i]
    if path:sub(1, 1) ~= "/" then
      path = os.path_resolve(os.get_work_dir(), path)
    end
    fs.rm(path)
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

/// Shell: reads stdin, parses as bin command (no spawn_path). Maintains cwd via os.chdir / os.get_work_dir; cd and pwd are builtins.
/// Child programs receive raw args and resolve paths themselves with os.get_work_dir + os.path_resolve.
pub const SH: &str = r#"
local child_pid = nil
local last_program = nil
local last_not_found_pid = nil
local child_ever_ran = false
os.chdir(os.get_home() or "/")
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
        local ok, replacement = pcall(function() return os.autocomplete(line, os.get_work_dir()) end)
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
          io.write(os.get_work_dir() .. "\n")
        elseif prog == "cd" then
          if #args < 1 then
            os.chdir(os.get_home() or "/")
          else
            local arg1 = args[1]
            local resolved = os.path_resolve(os.get_work_dir(), arg1)
            local st = fs.stat(resolved)
            if not st then
              io.write("<red>cd: no such file or directory: " .. arg1 .. "</red>\n")
            elseif st.type ~= "directory" then
              io.write("<red>cd: not a directory: " .. arg1 .. "</red>\n")
            else
              os.chdir(resolved)
            end
          end
        else
          local spawn_args = args
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

/// Lua: runs a Lua file. Args: path [ -d ]. -help/--help prints usage; -d runs script as daemon (child process, no stdout/stdin relay).
/// Usage: lua script.lua  |  lua script.lua -d  |  lua -help
pub const LUA: &str = r#"
local args = os.get_args()
if not args or #args < 1 then
  io.write("lua: usage: lua <file> [ -d ]\n")
  io.write("  -d  run as daemon (no stdout/stdin relay)\n")
  return
end
if args[1] == "-help" or args[1] == "--help" or args[1] == "-d" then
  io.write("lua: usage: lua <file> [ -d ]\n")
  io.write("  -d  run as daemon (no stdout/stdin relay)\n")
  return
end
if #args >= 2 and args[#args] == "-d" then
  local path = args[1]
  if path:sub(1, 1) ~= "/" then path = os.path_resolve(os.get_work_dir(), path) end
  os.spawn_path(path, {}, { forward_stdout = false })
  return
end
local path = args[1]
if path:sub(1, 1) ~= "/" then path = os.path_resolve(os.get_work_dir(), path) end
local content = fs.read(path)
if not content then
  io.write("lua: cannot read file: " .. path .. "\n")
  return
end
local fn, err = load(content, "=" .. path, "t")
if not fn then
  io.write("lua: " .. tostring(err) .. "\n")
  return
end
fn()
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

/// Grep: search for pattern in files. Args: pattern, then optional paths (default: work dir). Resolves relative paths.
pub const GREP: &str = r#"
local args = os.get_args()
if not args or #args < 1 then
  io.write("grep: usage: grep pattern [path ...]\n")
  return
end
local pattern = args[1]
local paths = {}
if #args >= 2 then
  for i = 2, #args do
    local p = args[i]
    if p:sub(1, 1) ~= "/" then p = os.path_resolve(os.get_work_dir(), p) end
    paths[#paths + 1] = p
  end
else
  paths[#paths + 1] = os.get_work_dir()
end
local function grep_file(path)
  local st = fs.stat(path)
  if not st then return end
  if st.type == "directory" then return end
  local content = fs.read(path)
  if not content then return end
  local s = content .. "\n"
  local pos = 1
  local line_num = 0
  while pos <= #s do
    local next_nl = s:find("\n", pos, true)
    if not next_nl then break end
    line_num = line_num + 1
    local line = s:sub(pos, next_nl - 1)
    local found = false
    for i = 1, #line - #pattern + 1 do
      if line:sub(i, i + #pattern - 1) == pattern then found = true; break end
    end
    if found then
      io.write(path .. ":" .. tostring(line_num) .. ":" .. line .. "\n")
    end
    pos = next_nl + 1
  end
end
local function grep_path(path)
  local st = fs.stat(path)
  if not st then return end
  if st.type == "directory" then
    local entries = fs.ls(path)
    for i = 1, #entries do
      local name = entries[i].name
      local ends_slash = #path >= 1 and path:sub(-1) == "/"
      local full = (path == "." or ends_slash) and (path .. name) or (path .. "/" .. name)
      grep_path(full)
    end
  else
    grep_file(path)
  end
end
for i = 1, #paths do
  grep_path(paths[i])
end
"#;

/// Find: list files and dirs recursively. Args: path (default work dir). Resolves relative paths.
/// Find: find [path] [-name "pattern"] [-iname "pattern"] [-type f|d] [-size +n|-n|n] [-user name] [-mtime n].
/// Path defaults to current work dir. Only paths matching all given predicates are printed.
pub const FIND: &str = r#"
local args = os.get_args()
local path = nil
local name_pattern = nil
local iname_pattern = nil
local type_filter = nil
local size_spec = nil
local user_filter = nil
local mtime_days = nil
local i = 1
while i <= #args do
  local a = args[i]
  if a == "-name" then
    if i + 1 <= #args then name_pattern = args[i + 1]; i = i + 2 else i = i + 1 end
  elseif a == "-iname" then
    if i + 1 <= #args then iname_pattern = args[i + 1]; i = i + 2 else i = i + 1 end
  elseif a == "-type" then
    if i + 1 <= #args then type_filter = args[i + 1]; i = i + 2 else i = i + 1 end
  elseif a == "-size" then
    if i + 1 <= #args then size_spec = args[i + 1]; i = i + 2 else i = i + 1 end
  elseif a == "-user" then
    if i + 1 <= #args then user_filter = args[i + 1]; i = i + 2 else i = i + 1 end
  elseif a == "-mtime" then
    if i + 1 <= #args then mtime_days = tonumber(args[i + 1]); i = i + 2 else i = i + 1 end
  else
    if not path then path = a end
    i = i + 1
  end
end
if not path then path = os.get_work_dir() end
if path:sub(1, 1) ~= "/" then
  path = os.path_resolve(os.get_work_dir(), path)
end

local function glob_to_lua(pat)
  local r = {}
  for j = 1, #pat do
    local c = pat:sub(j, j)
    if c == "*" then r[#r + 1] = ".*"
    elseif c == "%" or c == "." or c == "+" or c == "-" or c == "?" or c == "[" or c == "]" or c == "(" or c == ")" then
      r[#r + 1] = "%" .. c
    else r[#r + 1] = c end
  end
  return table.concat(r)
end

local function basename(p)
  local idx = 0
  for j = 1, #p do if p:sub(j, j) == "/" then idx = j end end
  if idx == 0 then return p end
  return p:sub(idx + 1)
end

-- Pure Lua glob match (no C string.match) to avoid yield-across-C-boundary. Pattern from glob_to_lua: ^ .* %c literal $.
local function glob_match(s, lua_pat)
  local n, pn = #s, #lua_pat
  local si, pi = 1, 1
  while pi <= pn do
    local c = lua_pat:sub(pi, pi)
    if c == "^" then
      pi = pi + 1
    elseif c == "$" then
      return (pi >= pn) and (si > n)
    elseif c == "." and lua_pat:sub(pi + 1, pi + 1) == "*" then
      pi = pi + 2
      if pi > pn then return true end
      local rest = lua_pat:sub(pi)
      for k = si, n + 1 do
        if glob_match(s:sub(k), rest) then return true end
      end
      return false
    elseif c == "%" then
      local lit = lua_pat:sub(pi + 1, pi + 1)
      pi = pi + 2
      if si > n or s:sub(si, si) ~= lit then return false end
      si = si + 1
    else
      if si > n or s:sub(si, si) ~= c then return false end
      si = si + 1
      pi = pi + 1
    end
  end
  return si > n
end

local function name_matches(basename_str, pattern, case_insensitive)
  local b = basename_str
  local pat = pattern
  if case_insensitive then b = b:lower(); pat = pat:lower() end
  local lua_pat = "^" .. glob_to_lua(pat) .. "$"
  -- Use glob_match (pure Lua) to avoid yield across C boundary from string.match/find
  return glob_match(b, lua_pat)
end

local function size_matches(st_size, spec)
  if not spec then return true end
  local s = spec
  local sign = 0
  if s:sub(1, 1) == "+" then sign = 1; s = s:sub(2)
  elseif s:sub(1, 1) == "-" then sign = -1; s = s:sub(2) end
  local mult = 1
  if s:sub(-1) == "K" then mult = 1024; s = s:sub(1, -2)
  elseif s:sub(-1) == "M" then mult = 1024 * 1024; s = s:sub(1, -2)
  elseif s:sub(-1) == "G" then mult = 1024 * 1024 * 1024; s = s:sub(1, -2) end
  local n = (tonumber(s) or 0) * mult
  if sign == 1 then return st_size > n end
  if sign == -1 then return st_size < n end
  return st_size == n
end

local function mtime_matches(st_mtime, days)
  if days == nil then return true end
  local now_secs = os.time and os.time() or 0
  if not st_mtime or st_mtime == 0 then return false end
  local age_days = (now_secs - st_mtime) / 86400
  if days < 0 then return age_days <= math.abs(days) end
  if days > 0 then return age_days >= days end
  return age_days < 1
end

local function matches(st, p)
  if name_pattern and not name_matches(basename(p), name_pattern, false) then return false end
  if iname_pattern and not name_matches(basename(p), iname_pattern, true) then return false end
  if type_filter == "f" and st.type ~= "file" then return false end
  if type_filter == "d" and st.type ~= "directory" then return false end
  if size_spec and not size_matches(st.size, size_spec) then return false end
  if user_filter and st.owner ~= user_filter then return false end
  if mtime_days ~= nil and not mtime_matches(st.mtime, mtime_days) then return false end
  return true
end

local function visit(root)
  local st = fs.stat(root)
  if not st then return end
  if matches(st, root) then io.write(root .. "\n") end
  if st.type ~= "directory" then return end
  local entries = fs.ls(root)
  for i = 1, #entries do
    local name = entries[i].name
    local ends_slash = #root >= 1 and root:sub(-1) == "/"
    local full = (root == "" or ends_slash) and (root .. name) or (root .. "/" .. name)
    visit(full)
  end
end
visit(path)
"#;

/// Sed: substitute pattern with replacement. Args: pattern, replacement, [file]. Resolves relative file path.
pub const SED: &str = r#"
local args = os.get_args()
if not args or #args < 2 then
  io.write("sed: usage: sed pattern replacement [file]\n")
  return
end
local pattern = args[1]
local replacement = args[2]
local function do_sed(content)
  local from = 1
  while from <= #content do
    local start_pos, end_pos = content:find(pattern, from, true)
    if not start_pos then
      io.write(content:sub(from))
      break
    end
    io.write(content:sub(from, start_pos - 1))
    io.write(replacement)
    from = end_pos + 1
  end
end
if #args >= 3 then
  local path = args[3]
  if path:sub(1, 1) ~= "/" then
    path = os.path_resolve(os.get_work_dir(), path)
  end
  local content = fs.read(path)
  if not content then
    io.write("sed: cannot read " .. path .. "\n")
    return
  end
  do_sed(content)
else
  while true do
    local line = io.read()
    if not line then break end
    do_sed(line .. "\n")
  end
end
"#;

/// Programs to include when bootstrapping a new VM. User can delete them later.
pub const DEFAULT_BIN_PROGRAMS: &[(&str, &str)] = &[
    ("cat", CAT),
    ("coin", COIN),
    ("echo", ECHO),
    ("echo_stdin", ECHO_STDIN),
    ("find", FIND),
    ("grep", GREP),
    ("ls", LS),
    ("lua", LUA),
    ("mem_stress", MEM_STRESS),
    ("rm", RM),
    ("sed", SED),
    ("sh", SH),
    ("ssh", SSH),
    ("ssh-server", SSH_SERVER),
    ("touch", TOUCH),
];
