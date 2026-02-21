-- Emailbox NPC: discover credentials and send 10 emails to Haru; append each to /var/www/index.
-- Inbox is served on-demand by emailbox_httpd.lua (GET /inbox returns first page via mail.list).
-- Run as: lua /var/www/email_sender.lua (started from /etc/bootstrap).

local TICKS_PER_20_SEC = 600   -- ~20 seconds at 30 TPS
local HARU_ADDRESS = "haru@mail.null"
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
