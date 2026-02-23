-- money.null HTTP daemon: GET / returns balances, public keys, and recent transactions.
-- Requires: /etc/wallet/fkebank/token (USD), optional /etc/wallet/crypto_addresses (BTC=addr per line).

local token_path = "/etc/wallet/fkebank/token"
local crypto_addrs_path = "/etc/wallet/crypto_addresses"
local root = (os.get_args() and os.get_args()[1]) or "/var/www"

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

        local ok_usd, usd_balance = pcall(fkebank.balance, token_path)
        if ok_usd and usd_balance then
          lines[#lines + 1] = string.format("USD: %d cents\n", usd_balance)
        else
          lines[#lines + 1] = "USD: (no account or token)\n"
        end

        local addrs_lines = fs.read_lines(crypto_addrs_path)
        if addrs_lines and type(addrs_lines) == "table" then
          for i = 1, #addrs_lines do
            local line = addrs_lines[i]
            if type(line) == "string" and line ~= "" then
              local currency, addr = line:match("^(%w+)=(.+)$")
              if currency and addr and addr ~= "" then
                local ok, bal = pcall(crypto.balance, addr)
                lines[#lines + 1] = string.format("%s: %s (%s)\n", currency, addr, ok and bal and tostring(bal) or "?")
              end
            end
          end
        end

        lines[#lines + 1] = "\n# Recent USD transactions\n"
        local ok_h, usd_history = pcall(fkebank.history, token_path, "")
        if ok_h and usd_history and type(usd_history) == "table" then
          for i = 1, math.min(20, #usd_history) do
            local tx = usd_history[i]
            if type(tx) == "table" then
              lines[#lines + 1] = string.format("%s -> %s: %d (%s)\n", tostring(tx.from_key), tostring(tx.to_key), tx.amount or 0, tostring(tx.description or ""))
            end
          end
        end

        if addrs_lines and type(addrs_lines) == "table" then
          lines[#lines + 1] = "\n# Recent crypto transactions (first address)\n"
          for i = 1, #addrs_lines do
            local line = addrs_lines[i]
            if type(line) == "string" and line ~= "" then
              local currency, addr = line:match("^(%w+)=(.+)$")
              if currency and addr and addr ~= "" then
                local ok_h2, crypto_history = pcall(crypto.history, addr, "")
                if ok_h2 and crypto_history and type(crypto_history) == "table" and #crypto_history > 0 then
                  for j = 1, math.min(5, #crypto_history) do
                    local tx = crypto_history[j]
                    if type(tx) == "table" then
                      lines[#lines + 1] = string.format("%s: %s -> %s: %d\n", currency, tostring(tx.from_key), tostring(tx.to_key), tx.amount or 0)
                    end
                  end
                end
                break
              end
            end
          end
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
