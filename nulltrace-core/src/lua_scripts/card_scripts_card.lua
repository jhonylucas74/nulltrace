-- card.null NTML script: request_card, on_card_received, payWithCard

local function url_encode(s)
  if not s then return "" end
  s = tostring(s)
  s = s:gsub("&", "%%26")
  s = s:gsub("=", "%%3D")
  s = s:gsub(" ", "+")
  return s
end

function requestCard(ctx)
  browser.request_card("card.null", "on_card_received")
  ui.set_visible("resultArea", true)
  ui.set_text("resultMsg", "Waiting for card selection...")
end

function on_card_received(ctx)
  local card_number = ctx.eventData["card_number"] or ""
  local cvv = ctx.eventData["cvv"] or ""
  local expiry_month = ctx.eventData["expiry_month"] or ""
  local expiry_year = ctx.eventData["expiry_year"] or ""
  local holder_name = ctx.eventData["holder_name"] or ""
  local last4 = ctx.eventData["last4"] or ""
  storage.set("last_card_number", card_number)
  storage.set("last_cvv", cvv)
  storage.set("last_expiry_month", expiry_month)
  storage.set("last_expiry_year", expiry_year)
  storage.set("last_holder_name", holder_name)
  ui.set_text("resultMsg", "Card ****" .. last4 .. " selected. Click Pay $100 to charge.")
  ui.set_visible("resultArea", true)
end

function payWithCard(ctx)
  print("[payWithCard] START")
  local card_number = storage.get("last_card_number") or ""
  local cvv = storage.get("last_cvv") or ""
  local expiry_month = storage.get("last_expiry_month") or ""
  local expiry_year = storage.get("last_expiry_year") or ""
  local holder_name = storage.get("last_holder_name") or ""
  print("[payWithCard] card_number len=" .. #card_number .. " cvv len=" .. #cvv)
  if card_number == "" or cvv == "" then
    print("[payWithCard] no card - early return")
    ui.set_text("resultMsg", "No card selected. Click Collect card first.")
    return
  end
  local body = "card_number=" .. url_encode(card_number) ..
    "&cvv=" .. url_encode(cvv) ..
    "&expiry_month=" .. url_encode(expiry_month) ..
    "&expiry_year=" .. url_encode(expiry_year) ..
    "&holder_name=" .. url_encode(holder_name)
  print("[payWithCard] calling http.post card.null/pay (blocking until curl returns)")
  local ok, resp = pcall(http.post, "card.null/pay", body)
  print("[payWithCard] http.post returned ok=" .. tostring(ok) .. " resp type=" .. type(resp))
  if not ok then
    print("[payWithCard] http.post FAILED: " .. tostring(resp))
    ui.set_visible("resultArea", true)
    ui.set_text("resultMsg", "Payment request failed: " .. tostring(resp))
    ui.set_class("resultMsg", "text-red-400 block")
    return
  end
  local status = resp and resp.status or 0
  local resp_body = resp and resp.body or ""
  print("[payWithCard] status=" .. tostring(status) .. " body_len=" .. #resp_body)
  print("[payWithCard] /pay response body:\n" .. resp_body)
  print("[payWithCard] parsing body with str.parse_table")
  local t = str.parse_table(resp_body or "")
  print("[payWithCard] parse done, t.success=" .. tostring(t.success) .. " t.message=" .. tostring(t.message))
  local success = t.success == "true"
  local message = t.message or ""
  if success then
    print("[payWithCard] SUCCESS branch - applying patches")
    local new_total = t.new_total or "0.00"
    ui.set_visible("resultArea", true)
    ui.set_text("total", "Total collected: $" .. new_total)
    ui.set_text("resultMsg", message ~= "" and message or "Payment successful. $100 charged.")
    ui.set_class("resultMsg", "text-green-400 font-medium block")
    ui.set_text("payBtnLabel", "Pay again")
    print("[payWithCard] SUCCESS patches applied")
  else
    print("[payWithCard] FAILURE branch - applying patches")
    ui.set_visible("resultArea", true)
    ui.set_text("resultMsg", message ~= "" and message or "Payment failed.")
    ui.set_class("resultMsg", "text-red-400 block")
    print("[payWithCard] FAILURE patches applied message=" .. tostring(message))
  end
  print("[payWithCard] END")
end
