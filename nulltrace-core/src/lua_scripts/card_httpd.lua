-- card.null HTTP daemon: GET / shows total collected and form to pay $100.
-- POST /pay accepts card data, creates invoice, pays via card.pay_invoice.
-- Requires: /etc/wallet/fkebank/token (destination key for card.null).

local token_path = "/etc/wallet/fkebank/token"
local AMOUNT_CENTS = 10000  -- $100 per invoice

local function format_usd(cents)
  if type(cents) ~= "number" then return "0.00" end
  return string.format("%.2f", cents / 100)
end

-- Parse application/x-www-form-urlencoded body into table
local function parse_form_body(body)
  local t = {}
  if not body or body == "" then return t end
  for part in (body or ""):gmatch("([^&]+)") do
    local k, v = part:match("^([^=]+)=(.+)$")
    if k and v then
      t[k] = v
    end
  end
  return t
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
      local path_only = http.normalize_request_path(req.path or "/")
      local method = (req.method and req.method:upper()) or "GET"

      if path_only == "/pay" and (method == "POST" or method == "GET") then
        local params
        if method == "POST" and req.body and req.body ~= "" then
          params = parse_form_body(req.body)
        else
          local raw_path = req.path or ""
          local q = raw_path:match("%?(.+)$")
          params = parse_form_body(q or "")
        end
        local card_number = params.card_number or ""
        local cvv = params.cvv or ""
        local expiry_month = tonumber(params.expiry_month) or 0
        local expiry_year = tonumber(params.expiry_year) or 0
        local holder_name = params.holder_name or ""

        local dest_ok, dest_key = pcall(fkebank.key, token_path)
        if not dest_ok or not dest_key or dest_key == "" then
          body = "# Error: no destination key (token file missing or invalid).\n"
          status = 500
        else
          local inv_ok, invoice_id = pcall(card.create_invoice, dest_key, AMOUNT_CENTS)
          if not inv_ok or not invoice_id then
            body = "# Error creating invoice: " .. tostring(invoice_id or "unknown") .. "\n"
            status = 500
          else
            local pay_ok, pay_err = pcall(card.pay_invoice, invoice_id, card_number, cvv, expiry_month, expiry_year, holder_name)
            if pay_ok then
              body = "# Payment successful. $100 charged.\n"
              status = 200
            else
              body = "# Payment failed: " .. tostring(pay_err or "unknown") .. "\n"
              status = 400
            end
          end
        end
        headers = { ["Content-Type"] = "text/plain; charset=utf-8" }
      else
        -- GET /
        local dest_ok, dest_key = pcall(fkebank.key, token_path)
        local total = 0
        if dest_ok and dest_key and dest_key ~= "" then
          local ok, t = pcall(card.total_collected, dest_key)
          if ok and type(t) == "number" then total = t end
        end

        local lines = {
          "# card.null - Pay $100 per invoice\n",
          "\n",
          "Total collected: $" .. format_usd(total) .. "\n",
          "\n",
          "## Pay $100\n",
          "POST /pay with form fields: card_number, cvv, expiry_month, expiry_year, holder_name\n",
        }
        body = table.concat(lines, "")
        status = 200
        headers = { ["Content-Type"] = "text/plain; charset=utf-8" }
      end
    end

    local res = http.build_response(status, body or "", headers)
    net.send(pkt.src_ip, pkt.src_port, res)
  end
end
