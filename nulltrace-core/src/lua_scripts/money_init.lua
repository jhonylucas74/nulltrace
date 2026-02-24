-- One-time init: create crypto wallets (BTC, ETH, SOL), write /etc/wallet/crypto_addresses.
-- Idempotent: if crypto_addresses exists, skip. Run before money_httpd and money_refund.
-- Seed is done from Rust in main.rs when creating money.null VM.

local addrs_path = "/etc/wallet/crypto_addresses"
local keys_dir = "/etc/wallet/keys"

if not fs.stat(addrs_path) then
  local lines = {}
  for _, currency in ipairs({ "BTC", "ETH", "SOL" }) do
    local addr = crypto.create_wallet(currency, keys_dir)
    if addr and addr ~= "" then
      lines[#lines + 1] = currency .. "=" .. addr
    end
  end
  if #lines > 0 then
    fs.write(addrs_path, table.concat(lines, "\n"), "text/plain")
  end
end
