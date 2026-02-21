-- Emailbox NPC: discover credentials, send 10 emails to Haru, then append received emails to /var/www/inbox.
-- Run as: lua /var/www/email_sender.lua (started from /etc/bootstrap).

local TICKS_PER_20_SEC = 600   -- ~20 seconds at 30 TPS
local HARU_ADDRESS = "haru@mail.null"
local INBOX_PATH = "/var/www/inbox"
local INBOX_SEEN_PATH = "/var/www/.inbox.seen"
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
  local content = fs.read(INBOX_SEEN_PATH)
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

-- Append id to .inbox.seen and append formatted line to inbox
local function append_email_to_inbox(seen, email)
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
  local old_inbox = fs.read(INBOX_PATH)
  local new_inbox = (old_inbox or "") .. line
  fs.write(INBOX_PATH, new_inbox, "text/plain")
  local old_seen = fs.read(INBOX_SEEN_PATH)
  local new_seen = (old_seen or "") .. id .. "\n"
  fs.write(INBOX_SEEN_PATH, new_seen, "text/plain")
end

-- Loop: list inbox by page, append new emails to /var/www/inbox (run forever)
local seen = load_seen_ids()
while true do
  local page = 0
  local has_more = true
  while has_more do
    local ok, r = pcall(mail.list, address, token, "inbox", page)
    if ok and r and type(r) == "table" then
      local emails = r.emails or {}
      for j = 1, #emails do
        local email = emails[j]
        if type(email) == "table" and email.id then
          append_email_to_inbox(seen, email)
        end
      end
      has_more = r.has_more
    else
      has_more = false
    end
    page = page + 1
  end
  -- Poll every ~5 seconds (150 ticks at 30 TPS)
  for _ = 1, 150 do
  end
end
