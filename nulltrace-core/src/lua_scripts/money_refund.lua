-- money.null: when a transfer arrives (USD or crypto), send double back to the sender.
-- Send 1 USD -> get 2 USD back. Send 1 BTC -> get 2 BTC back.
-- Uses incoming_money.listen_usd / listen + try_recv (no polling, no refunded_ids files).
-- Failed refunds: /var/www/refund_errors.txt
-- Debug log: /var/www/refund_debug.log (visible on money.null site)

os.server_log("money_refund.lua STARTED (first line)")
local token_path = "/etc/wallet/fkebank/token"
local crypto_addrs_path = "/etc/wallet/crypto_addresses"
local keys_dir = "/etc/wallet/keys"
local refund_errors_path = "/var/www/refund_errors.txt"
local refund_debug_path = "/var/www/refund_debug.log"

local MIN_USD_CENTS = 1
local MIN_CRYPTO_CENTS = 1

-- Debug: only os.server_log (avoids yield across C-call from fs/string ops)
local function debug_log(msg)
  pcall(os.server_log, msg)
end

local function append_refund_error(currency, from_key, amount_cents, reason)
  local msg = string.format("[%s] %s refund failed: %s -> %s cents, reason: %s",
    os.date("!%Y-%m-%d %H:%M:%S"), currency, from_key or "?", amount_cents or 0, reason or "unknown")
  pcall(os.server_log, msg)
  -- Append to file for money.null site (minimal ops to avoid yield boundary)
  local ok_read, content = pcall(fs.read, refund_errors_path)
  content = (ok_read and content) and content or ""
  local sep = (#content == 0) and "" or "\n"
  pcall(fs.write, refund_errors_path, content .. sep .. msg .. "\n", "text/plain")
end

-- USD: pass token path (resolves to fkebank key internally)
debug_log("refund process starting, registering USD...")
incoming_money.listen_usd(token_path)
debug_log("USD registered")

-- Crypto: pass public address directly for each currency (use fs.parse_key_value_line to avoid string.match yield boundary)
local crypto_count = 0
local addrs_lines = fs.read_lines(crypto_addrs_path)
if addrs_lines and type(addrs_lines) == "table" then
  for i = 1, #addrs_lines do
    local line = addrs_lines[i]
    if type(line) == "string" and line ~= "" then
      local currency, addr = fs.parse_key_value_line(line)
      if currency and addr and addr ~= "" then
        incoming_money.listen(addr)
        crypto_count = crypto_count + 1
      end
    end
  end
end
debug_log("crypto addresses registered: " .. tostring(crypto_count))

local loop_count = 0
-- Process loop: try_recv (non-blocking) so VM can yield between ticks; poll loop delivers tx
while true do
  loop_count = loop_count + 1
  if loop_count % 5000 == 1 and loop_count > 1 then
    debug_log("alive, loop=" .. tostring(loop_count) .. " (waiting for tx)")
  end

  local tx = incoming_money.try_recv()
  if tx and type(tx) == "table" then
    debug_log(string.format("RECV tx id=%s currency=%s from_key=%s to_key=%s amount=%s",
      tostring(tx.id or "?"), tostring(tx.currency or "?"),
      tostring(tx.from_key or "?"), tostring(tx.to_key or "?"), tostring(tx.amount or "?")))

    if tx.from_key and tx.from_key ~= "system" then
      local amount = (tx.amount or 0) * 2
      if tx.currency == "USD" and amount >= MIN_USD_CENTS then
        debug_log(string.format("SENDING USD double back: to=%s amount=%s cents", tx.from_key, tostring(amount)))
        local ok, result = pcall(fkebank.transfer, token_path, tx.from_key, amount, "Double back")
        if ok and result == true then
          debug_log("OK: USD transfer succeeded")
        else
          debug_log(string.format("FAIL: USD transfer ok=%s result=%s", tostring(ok), tostring(result)))
          append_refund_error("USD", tx.from_key, amount, ok and tostring(result) or "error")
        end
      elseif amount >= MIN_CRYPTO_CENTS then
        local priv_path = keys_dir .. "/" .. string.lower(tx.currency) .. ".priv"
        debug_log(string.format("SENDING %s double back: from=%s to=%s amount=%s cents",
          tx.currency, tx.to_key, tx.from_key, tostring(amount)))
        local ok, result = pcall(crypto.transfer, tx.currency, tx.to_key, priv_path, tx.from_key, amount, "Double back")
        if ok and result == true then
          debug_log("OK: " .. tx.currency .. " transfer succeeded")
        else
          debug_log(string.format("FAIL: %s transfer ok=%s result=%s", tx.currency, tostring(ok), tostring(result)))
          append_refund_error(tx.currency, tx.from_key, amount, ok and tostring(result) or "error")
        end
      elseif amount > 0 then
        local min = (tx.currency == "USD") and MIN_USD_CENTS or MIN_CRYPTO_CENTS
        debug_log(string.format("SKIP: amount %s below minimum %s", tostring(amount), tostring(min)))
        append_refund_error(tx.currency, tx.from_key, amount, "below minimum " .. min .. " cents (double)")
      end
    else
      debug_log("SKIP: from_key is system or nil")
    end
  end
end
