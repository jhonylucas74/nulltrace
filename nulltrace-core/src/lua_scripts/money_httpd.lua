-- money.null HTTP daemon: GET / returns balances, public keys, and recent transactions.
-- Requires: /etc/wallet/fkebank/token (USD), optional /etc/wallet/crypto_addresses (BTC=addr per line).
-- All amounts are stored in hundredths (cents); we display in units (e.g. 2 -> 0.02 BTC).

local token_path = "/etc/wallet/fkebank/token"
local crypto_addrs_path = "/etc/wallet/crypto_addresses"
local refund_errors_path = "/var/www/refund_errors.txt"
local refund_debug_path = "/var/www/refund_debug.log"
local root = (os.get_args() and os.get_args()[1]) or "/var/www"

local function format_usd(cents)
  if type(cents) ~= "number" then return "0.00" end
  return string.format("%.2f", cents / 100)
end
local function format_crypto(cents)
  if type(cents) ~= "number" then return "0.0000" end
  return string.format("%.4f", cents / 100)
end

net.listen(80)
while true do
  local pkt = net.recv()
  if pkt and (tonumber(pkt.dst_port) or 0) == 80 then
    local body, status, headers
    local req_ok, req = pcall(http.parse_request, pkt.data)
    if not req_ok or not req then
      body = "# Bad request.\n"
      status = 400
      headers = { ["Content-Type"] = "text/plain; charset=utf-8" }
    else
      local handler_ok, r1, r2, r3 = pcall(function()
        local path_only = http.normalize_request_path(req.path or "/")
        if path_only ~= "/" and path_only ~= "" and path_only ~= "index" and path_only ~= "index/" then
          return httpd.serve(root, path_only)
        end

        local lines = { "# money.null - Balances and transactions\n" }

        -- USD: show account key (company key) and balance, same format as crypto
        local ok_key, usd_key = pcall(fkebank.key, token_path)
        local usd_key_str = (ok_key and usd_key and type(usd_key) == "string" and usd_key ~= "") and usd_key or nil
        local ok_usd, usd_balance = pcall(fkebank.balance, token_path)
        if ok_usd and type(usd_balance) == "number" then
          if usd_key_str then
            lines[#lines + 1] = string.format("USD: %s (%s)\n", usd_key_str, format_usd(usd_balance))
          else
            lines[#lines + 1] = string.format("USD: %s\n", format_usd(usd_balance))
          end
        else
          if usd_key_str then
            lines[#lines + 1] = string.format("USD: %s (no balance)\n", usd_key_str)
          else
            lines[#lines + 1] = "USD: (no account or token)\n"
          end
        end

        local addrs_lines = fs.read_lines(crypto_addrs_path)
        if addrs_lines and type(addrs_lines) == "table" then
          for i = 1, #addrs_lines do
            local line = addrs_lines[i]
            if type(line) == "string" and line ~= "" then
              local currency, addr = fs.parse_key_value_line(line)
              if currency and addr and addr ~= "" then
                local ok, bal = pcall(crypto.balance, addr)
                local display = (ok and type(bal) == "number") and format_crypto(bal) or "?"
                lines[#lines + 1] = string.format("%s: %s (%s)\n", currency, addr, display)
              end
            end
          end
        end

        -- Collect all transactions (USD + all crypto addresses), then sort by date descending
        local all_tx = {}
        local ok_h, usd_history = pcall(fkebank.history, token_path, "")
        if ok_h and usd_history and type(usd_history) == "table" then
          for i = 1, #usd_history do
            local tx = usd_history[i]
            if type(tx) == "table" and tx.created_at_ms then
              all_tx[#all_tx + 1] = {
                created_at_ms = tx.created_at_ms,
                currency = "USD",
                from_key = tostring(tx.from_key or ""),
                to_key = tostring(tx.to_key or ""),
                amount = tx.amount or 0,
                description = tostring(tx.description or ""),
              }
            end
          end
        end
        if addrs_lines and type(addrs_lines) == "table" then
          for i = 1, #addrs_lines do
            local line = addrs_lines[i]
            if type(line) == "string" and line ~= "" then
              local currency, addr = line:match("^(%w+)=(.+)$")
              if currency and addr and addr ~= "" then
                addr = (addr and type(addr) == "string") and addr:match("^%s*(.-)%s*$") or addr
                local ok_h2, crypto_history = pcall(crypto.history, addr, "")
                if ok_h2 and crypto_history and type(crypto_history) == "table" then
                  for j = 1, #crypto_history do
                    local tx = crypto_history[j]
                    if type(tx) == "table" and tx.created_at_ms then
                      all_tx[#all_tx + 1] = {
                        created_at_ms = tx.created_at_ms,
                        currency = currency,
                        from_key = tostring(tx.from_key or ""),
                        to_key = tostring(tx.to_key or ""),
                        amount = tx.amount or 0,
                        description = tostring(tx.description or ""),
                      }
                    end
                  end
                end
              end
            end
          end
        end
        table.sort(all_tx, function(a, b) return (a.created_at_ms or 0) > (b.created_at_ms or 0) end)
        local max_tx = 100
        lines[#lines + 1] = "\n# All transactions (USD + all keys, newest first)\n"
        for i = 1, math.min(max_tx, #all_tx) do
          local tx = all_tx[i]
          if tx.currency == "USD" then
            lines[#lines + 1] = string.format("%s -> %s: %s (%s)\n", tx.from_key, tx.to_key, format_usd(tx.amount), tx.description)
          else
            lines[#lines + 1] = string.format("%s: %s -> %s: %s\n", tx.currency, tx.from_key, tx.to_key, format_crypto(tx.amount))
          end
        end
        if #all_tx > max_tx then
          lines[#lines + 1] = string.format("... and %d more\n", #all_tx - max_tx)
        end

        -- Refund errors (visible when refund fails or amount below minimum)
        local ok_err, err_content = pcall(fs.read, refund_errors_path)
        if ok_err and err_content and type(err_content) == "string" and err_content ~= "" then
          lines[#lines + 1] = "\n# Refund errors (failed or below minimum)\n"
          lines[#lines + 1] = err_content
        end

        -- Debug log (refund process lifecycle)
        local ok_dbg, dbg_content = pcall(fs.read, refund_debug_path)
        if ok_dbg and dbg_content and type(dbg_content) == "string" and dbg_content ~= "" then
          lines[#lines + 1] = "\n# Refund debug log\n"
          lines[#lines + 1] = dbg_content
        end

        return table.concat(lines, ""), 200
      end)

      if handler_ok and r1 then
        body = r1
        status = r2 or 200
        headers = r3
      else
        local err_msg = (type(r1) == "string" and r1 ~= "") and r1 or tostring(r1)
        if err_msg == "" or err_msg == "nil" then
          err_msg = "unknown error (no message)"
        end
        -- Log to process stdout so cluster/VM logs show the cause of 500
        print("[money_httpd] 500 error: " .. err_msg)
        body = "# Server error.\n\n" .. err_msg .. "\n"
        status = 500
      end
      if not headers then
        headers = { ["Content-Type"] = "text/plain; charset=utf-8" }
      end
    end

    local res = http.build_response(status, body or "", headers)
    net.send(pkt.src_ip, pkt.src_port, res)
  end
end
