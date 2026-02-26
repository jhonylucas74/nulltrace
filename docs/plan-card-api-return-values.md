# Plan: card_api return values instead of errors

## Goal

Change `card_api` (create_invoice, pay_invoice, total_collected) to return values instead of raising errors. This prevents mlua from exposing internal Rust paths (e.g. `src/cluster/process.rs:67:31`) in error messages to users.

## Current behavior

- All three functions return `Err(mlua::Error::runtime(...))` on failure.
- mlua wraps this in `CallbackError` with a full traceback and pushes it to Lua.
- Lua's `pcall` catches it; `tostring(pay_err)` includes the traceback with `.rs` paths.
- User sees: `# Payment failed: runtime error: Card credit limit exceeded\nstack traceback:\n\t...\n\tsrc/cluster/process.rs:67:31: in ?`

## New behavior

- Functions return `Ok(...)` on both success and failure.
- Success: `Ok((true, result))` or `Ok((invoice_id,))`
- Failure: `Ok((false, "message"))` or `Ok((nil, "message"))`
- No mlua error path → no traceback → user sees only the clean message.

---

## API contract

### 1. card.create_invoice(destination_key, amount_cents)

| Result | Return value | Lua usage |
|--------|--------------|-----------|
| Success | `(invoice_id)` | `local id, err = card.create_invoice(...); if id then ... end` |
| Failure | `(nil, "message")` | `if not id then body = "# Error: " .. tostring(err) end` |

### 2. card.pay_invoice(invoice_id, card_number, cvv, expiry_month, expiry_year, holder_name)

| Result | Return value | Lua usage |
|--------|--------------|-----------|
| Success | `(true)` | `local ok, err = card.pay_invoice(...); if ok then ... end` |
| Failure | `(false, "message")` | `if not ok then body = "# Payment failed: " .. tostring(err) end` |

### 3. card.total_collected(destination_key)

| Result | Return value | Lua usage |
|--------|--------------|-----------|
| Success | `(total_cents)` | `local total, err = card.total_collected(...); if total ~= nil then ... end` |
| Failure | `(nil, "message")` | `if total == nil then ... end` |

---

## Files to change

### 1. nulltrace-core/src/cluster/lua_api/card_api.rs

**create_invoice**

- Replace `map_err(...)?` and `Ok(Value::String(...))` with `match`:
  - `Ok(invoice)` → `Ok((Value::String(_lua.create_string(&invoice.id.to_string())?),))` (multi-value return)
  - `Err(e)` → `Ok((Value::Nil, Value::String(_lua.create_string(&e.to_string())?)))`

**pay_invoice**

- Replace `map_err(...)?` and `Ok(Value::Boolean(true))` with `match`:
  - `Ok(())` → `Ok((Value::Boolean(true),))`
  - `Err(e)` → `Ok((Value::Boolean(false), Value::String(_lua.create_string(&e.to_string())?)))`
- For invalid UUID: return `Ok((false, "Invalid invoice id"))` instead of `Err(...)`

**total_collected**

- Replace `map_err(...)?` and `Ok(Value::Integer(total))` with `match`:
  - `Ok(total)` → `Ok((Value::Integer(total),))`
  - `Err(e)` → `Ok((Value::Nil, Value::String(_lua.create_string(&e.to_string())?)))`

**mlua return type**

- `create_function` accepts callbacks that return `Result<R>` where `R: IntoLuaMulti`.
- Tuples implement `IntoLuaMulti`: `(true,)` → single value; `(false, "msg")` → two values.
- Use `mlua::Nil` for nil: `Ok((Nil, err_string))` for failure.
- Single value: `Ok(result)` or `Ok((result,))`; two values: `Ok((nil, err))`.

### 2. nulltrace-core/src/lua_scripts/card_httpd.lua

**create_invoice (lines 71–74)**

```lua
-- Before:
local inv_ok, invoice_id = pcall(card.create_invoice, dest_key, AMOUNT_CENTS)
if not inv_ok or not invoice_id then
  body = "# Error creating invoice: " .. tostring(invoice_id or "unknown") .. "\n"
  status = 500
else
  ...
end

-- After:
local invoice_id, inv_err = card.create_invoice(dest_key, AMOUNT_CENTS)
if not invoice_id or invoice_id == "" then
  body = "# Error creating invoice: " .. tostring(inv_err or "unknown") .. "\n"
  status = 500
else
  ...
end
```

**pay_invoice (lines 76–85)**

```lua
-- Before:
local pay_ok, pay_err = pcall(card.pay_invoice, invoice_id, card_number, cvv, expiry_month, expiry_year, holder_name)
if pay_ok then
  ...
else
  body = "# Payment failed: " .. tostring(pay_err or "unknown") .. "\n"
  status = 400
end

-- After:
local pay_ok, pay_err = card.pay_invoice(invoice_id, card_number, cvv, expiry_month, expiry_year, holder_name)
if pay_ok then
  ...
else
  body = "# Payment failed: " .. tostring(pay_err or "unknown") .. "\n"
  status = 400
end
```

**total_collected (lines 79, 95)**

```lua
-- Before:
local ok_t, t = pcall(card.total_collected, dest_key)
if ok_t and type(t) == "number" then new_total = t end

-- After:
local t, t_err = card.total_collected(dest_key)
if t ~= nil and type(t) == "number" then new_total = t end
```

Same pattern for the GET / handler (lines 94–95).

---

## Tests

### 1. card_invoice_service tests

- No changes to `card_invoice_service.rs` tests.
- `CardInvoiceService` is unchanged; only the Lua API layer changes.

### 2. Integration / manual

- Run `card.null` VM and:
  - POST /pay with valid card → success.
  - POST /pay with card over limit → failure.
  - Confirm the error message shows only the user-facing text (e.g. "Card credit limit exceeded") and no `.rs` paths.

---

## Rollout / risk

- **Scope:** Only `card_api.rs` and `card_httpd.lua`.
- **Other scripts:** `card_scripts_card.lua` uses `http.post` to the card.null server; it does not call `card.*` directly. No changes there.
- **Risk:** Low; behavior is localized to the card API and its HTTP handler.

---

## Checklist

- [x] Update `card_api.rs`: create_invoice return values
- [x] Update `card_api.rs`: pay_invoice return values
- [x] Update `card_api.rs`: total_collected return values
- [x] Update `card_httpd.lua`: create_invoice usage
- [x] Update `card_httpd.lua`: pay_invoice usage
- [x] Update `card_httpd.lua`: total_collected usage (2 places)
- [x] Verify mlua `MultiValue` / `IntoLuaMulti` for multi-value returns
- [ ] Manual test: card.null payment success and failure
