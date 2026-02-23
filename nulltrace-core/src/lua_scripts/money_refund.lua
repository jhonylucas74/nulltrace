-- money.null refund script: when a transfer arrives at our USD key, send half back to the sender.
-- Reads token from /etc/wallet/fkebank/token; tracks refunded tx ids in /etc/wallet/refunded_ids.

local token_path = "/etc/wallet/fkebank/token"
local refunded_path = "/etc/wallet/refunded_ids"

local function read_refunded_ids()
  local content = fs.read(refunded_path)
  if not content or content == "" then return {} end
  local set = {}
  for id in (content .. "\n"):gmatch("([^\n]+)") do
    set[id] = true
  end
  return set
end

local function append_refunded_id(id)
  local content = fs.read(refunded_path) or ""
  local new_line = (content == "" or content:match("\n$")) and "" or "\n"
  fs.write(refunded_path, content .. new_line .. tostring(id), "text/plain")
end

while true do
  local ok, history = pcall(fkebank.history, token_path, "")
  if ok and history and type(history) == "table" then
    local refunded = read_refunded_ids()
    for _, tx in ipairs(history) do
      if type(tx) == "table" and tx.id and tx.to_key and tx.from_key and tx.from_key ~= "system" then
        local tx_id = tostring(tx.id)
        if not refunded[tx_id] then
          local amount = math.floor((tx.amount or 0) / 2)
          if amount > 0 then
            local transfer_ok = pcall(fkebank.transfer, token_path, tx.from_key, amount, "Refund half")
            if transfer_ok then
              append_refunded_id(tx_id)
            end
          end
        end
      end
    end
  end
end
