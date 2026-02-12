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

/// Shell: reads stdin, parses as bin command (no spawn_path). When no child: spawn(program, args).
/// When has child: forwards stdin to child. Every loop relays child stdout to own stdout.
pub const SH: &str = r#"
local child_pid = nil
while true do
  if child_pid then
    local out = os.read_stdout(child_pid)
    if out and out ~= "" then io.write(out) end
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
        child_pid = os.spawn(t.program, t.args or {})
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

/// Programs to include when bootstrapping a new VM. User can delete them later.
pub const DEFAULT_BIN_PROGRAMS: &[(&str, &str)] = &[
    ("cat", CAT),
    ("echo", ECHO),
    ("echo_stdin", ECHO_STDIN),
    ("ls", LS),
    ("rm", RM),
    ("sh", SH),
    ("touch", TOUCH),
];
