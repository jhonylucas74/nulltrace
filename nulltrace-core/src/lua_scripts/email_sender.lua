-- Emailbox NPC: discover credentials, send 10 emails to Haru, then append received emails to /var/www/logs.
-- Run as: lua /var/www/email_sender.lua (started from /etc/bootstrap).

local TICKS_PER_20_SEC = 600   -- ~20 seconds at 30 TPS
local HARU_ADDRESS = "haru@mail.null"
local LOGS_PATH = "/var/www/logs"
local LOGS_SEEN_PATH = "/var/www/logs.seen"
local INDEX_PATH = "/var/www/index"   -- sent emails (emailbox.null/index)
local BODY_PATH = "/var/www/body.txt"

-- Discover own address: single directory under /etc/mail
local entries = fs.ls("/etc/mail")
if not entries or #entries < 1 then
  io.write("email_sender: no /etc/mail account found\n")
  return
end
local address = entries[1].name
local token = fs.read("/etc/mail/" .. address .. "/token")
if not token or token == "" then
  io.write("email_sender: could not read token for " .. address .. "\n")
  return
end

-- Read body from file (or use fallback)
local body = fs.read(BODY_PATH)
if not body or body == "" then
  body = "Hello from the emailbox NPC."
end

-- Send 10 emails to Haru, ~20 seconds apart; append each to /var/www/index
for i = 1, 10 do
  local subject = "Test " .. tostring(i) .. "/10"
  local ok, err = pcall(mail.send, address, token, HARU_ADDRESS, subject, body)
  if ok then
    local line = string.format("to=%s subject=%s n=%d\n", HARU_ADDRESS, subject, i)
    local old_index = fs.read(INDEX_PATH)
    local new_index = (old_index or "") .. line
    fs.write(INDEX_PATH, new_index, "text/plain")
  else
    io.write("email_sender: send failed: " .. tostring(err) .. "\n")
  end
  if i < 10 then
    -- Delay ~20 seconds: run for TICKS_PER_20_SEC ticks (VM yields automatically)
    for _ = 1, TICKS_PER_20_SEC do
      -- busy-wait; VM preemption yields between ticks
    end
  end
end

-- Load set of already-logged email ids (one id per line)
local function load_seen_ids()
  local seen = {}
  local content = fs.read(LOGS_SEEN_PATH)
  if content then
    for line in content:gmatch("[^\r\n]+") do
      local id = line:match("^%s*(.-)%s*$")
      if id and id ~= "" then
        seen[id] = true
      end
    end
  end
  return seen
end

-- Append id to logs.seen and append formatted line to logs
local function append_email_to_logs(seen, email)
  local id = email.id
  if seen[id] then
    return
  end
  seen[id] = true
  local line = string.format(
    "from=%s to=%s subject=%s id=%s date=%s\n",
    email.from_address or "",
    email.to_address or "",
    email.subject or "",
    id,
    tostring(email.sent_at_ms or "")
  )
  local old_logs = fs.read(LOGS_PATH)
  local new_logs = (old_logs or "") .. line
  fs.write(LOGS_PATH, new_logs, "text/plain")
  local old_seen = fs.read(LOGS_SEEN_PATH)
  local new_seen = (old_seen or "") .. id .. "\n"
  fs.write(LOGS_SEEN_PATH, new_seen, "text/plain")
end

-- Loop: list inbox, append new emails to logs (run forever)
local seen = load_seen_ids()
while true do
  local ok, list_result = pcall(mail.list, address, token, "inbox")
  if ok and list_result and type(list_result) == "table" then
    for j = 1, #list_result do
      local email = list_result[j]
      if type(email) == "table" and email.id then
        append_email_to_logs(seen, email)
      end
    end
  end
  -- Poll every ~5 seconds (150 ticks at 30 TPS)
  for _ = 1, 150 do
  end
end
