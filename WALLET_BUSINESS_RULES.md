# NullTrace Wallet — Business Rules & Security Analysis

> Internal document. Last updated: 2026-02-24.

---

## Table of Contents

1. [System Overview](#1-system-overview)
2. [Currencies & Accounts](#2-currencies--accounts)
3. [Business Rules](#3-business-rules)
   - [3.1 Account Creation](#31-account-creation)
   - [3.2 Transfers](#32-transfers)
   - [3.3 Credit / Debit (System Operations)](#33-credit--debit-system-operations)
   - [3.4 Currency Conversion](#34-currency-conversion)
   - [3.5 Credit Cards](#35-credit-cards)
   - [3.6 Incoming Money Listener](#36-incoming-money-listener)
4. [What Is Allowed](#4-what-is-allowed)
5. [What Is Not Allowed](#5-what-is-not-allowed)
6. [Security Vulnerabilities & Attack Vectors](#6-security-vulnerabilities--attack-vectors)

---

## 1. System Overview

The NullTrace wallet is a fictional multi-currency financial system embedded in a game world. It exposes functionality through three layers:

| Layer | Technology | Who uses it |
|---|---|---|
| Lua API (in-VM) | Lua sandbox via mlua | Lua game scripts running inside VMs |
| gRPC Service | Protobuf over gRPC | Game client (Tauri/React frontend) |
| DB Services | PostgreSQL via sqlx | Internal Rust backend only |

There are three distinct account types: **player**, **vm**, and **npc**. Each gets one Fkebank (USD) account. Crypto wallets are identity-less — ownership is proven solely by possession of the private key file stored in the VM's filesystem.

---

## 2. Currencies & Accounts

### 2.1 Currencies

| Currency | Type | Unit | Exchange rate to USD |
|---|---|---|---|
| **USD** | Fkebank (centralized) | cents | 1:1 |
| **BTC** | Crypto (simulated) | cents | 1 BTC cent = 250 USD cents |
| **ETH** | Crypto (simulated) | cents | 1 ETH cent = 20 USD cents |
| **SOL** | Crypto (simulated) | cents | 1 SOL cent = 1 USD cent |

All amounts are stored as `BIGINT` representing **cents** (integer arithmetic, no floating point in DB).

### 2.2 Account Identity

- **Fkebank (USD):** identified by a `fkebank-{32 hex}` key. One account per (owner_type, owner_id) pair. There can only be one account per player, one per VM, one per NPC.
- **Crypto:** identified by address (`bc1q...`, `0x...`, base58). No player_id link in DB — ownership is filesystem-based.
- **Player crypto vault:** auto-created key is `player-vault-{player_uuid}-{currency}`. This is a synthetic address used to track the player's crypto holdings server-side.

### 2.3 NPC Accounts

NPC accounts (e.g. `money.null`) have deterministic UUIDs derived from the account name:

```
owner_id = UUID_v5(npc_namespace, "nulltrace.npc.{account_id}")
```

This ensures stable identity across server restarts.

### 2.4 Authorization

- **USD operations via Lua:** require a **64-char hex token** stored in the VM's filesystem (`/etc/wallet/token`). The token is validated against `fkebank_tokens` table before any write.
- **USD operations via gRPC:** authorized by `player_id` from the authenticated session — no token needed.
- **Crypto operations via Lua:** require the private key file to be present on the VM's filesystem. The path is passed to `crypto.transfer()`.
- **Crypto operations via gRPC:** authorized by `player_id` only; no private key involved.

---

## 3. Business Rules

### 3.1 Account Creation

- Player wallet (USD + crypto vaults) is created **once** at account registration. Idempotent.
- Crypto vaults are **auto-created** on first balance read (`get_balances`).
- NPC accounts are created at server startup and are idempotent by `account_id`.
- Cards are created on demand by the player; no maximum number is enforced per player.
- A Fkebank token is regenerated on demand (overwriting the previous one via `ON CONFLICT`).

### 3.2 Transfers

#### USD (Fkebank)

- Sender must have sufficient balance (`balance >= amount`).
- Recipient key **must exist** in `fkebank_accounts`. Transfer fails with `RecipientNotFound` otherwise — money is never lost in transit.
- Self-transfer (`from_key == to_key`) is rejected.
- `amount <= 0` is rejected.
- The entire operation runs in a single DB transaction; on any error, both sides roll back atomically.
- A single transaction record is written (not a debit + credit pair).

#### Crypto (BTC / ETH / SOL)

- Sender vault must have sufficient balance.
- Recipient wallet is **auto-created** if it does not exist (idempotent via `ON CONFLICT`).
- Self-transfer is rejected.
- `amount <= 0` is rejected.
- Entire operation is atomic (`transfer_atomic`): debit → ensure recipient exists → credit → single record.

#### Via Lua API

- USD: `fkebank.transfer(token_path, to_key, amount, description)` — token validated before any state change.
- Crypto: `crypto.transfer(currency, from_address, priv_key_path, to_address, amount, description)` — reads private key from VM filesystem.

### 3.3 Credit / Debit (System Operations)

These are server-side operations, not player-accessible:

- **Credit:** adds funds from `'system'` to a player's balance. Only USD is supported directly. `from_key = 'system'` in the transaction record.
- **Debit:** removes funds from a player's balance to `'system'`. Fails with `InsufficientBalance` if balance is insufficient. `to_key = 'system'` in the transaction record.

### 3.4 Currency Conversion

Conversion formula:

```
out_amount = floor(in_amount × in_rate / out_rate)
```

Where rates (USD cents per unit cent):

```
USD = 1.0    BTC = 250.0    ETH = 20.0    SOL = 1.0
```

Rules:
- Same-currency conversion returns the amount unchanged.
- If `floor(result) == 0`, the operation fails with `ConvertedAmountTooSmall`.
- Conversion is atomic: debit from-currency, credit to-currency, both in a single DB transaction.
- Minimum meaningful conversions: at least 250 USD to get 1 BTC cent; at least 20 USD to get 1 ETH cent.

### 3.5 Credit Cards

Cards are **virtual Fkebank cards** with weekly billing cycles.

- **Card number:** Visa format (16 digits, starts with `4`), randomly generated.
- **CVV:** 3 digits, randomly generated.
- **Expiry:** 3 years from creation date.
- **Credit limit:** configurable at creation; default $1,000 (100,000 cents).
- **Billing day:** Monday at 12:00 UTC.

#### Purchase

- `current_debt + purchase_amount <= credit_limit` — enforced atomically in a single `UPDATE ... WHERE` clause.
- If the constraint fails, returns `CardLimitExceeded`.
- Purchase adds to the current **open statement** totals.

#### Statement Lifecycle

1. On card creation, an open statement is created: `period_start = now`, `period_end = next Monday 12:00 UTC`.
2. When `get_or_create_open_statement()` is called and `period_end < now`, the old statement is marked **closed** and a new one is created. This is **lazy rollover** — no background job required.
3. When the bill is paid, the statement is marked **paid**.

#### Bill Payment

1. Acquires a `SELECT FOR UPDATE` lock on the card row (prevents race conditions).
2. Reads the player's Fkebank USD account.
3. Atomically debits USD equal to `current_debt`.
4. Zeroes `current_debt` on the card.
5. Marks the current statement as **paid**.
6. If USD balance < debt, the entire transaction is rolled back — debt remains, nothing is charged.

#### Deletion

Soft delete only (`is_active = FALSE`). Historical transactions are preserved.

### 3.6 Incoming Money Listener

- Lua VMs can register a key to listen for incoming transfers.
- A background Rust task polls `wallet_transactions` every N milliseconds.
- State is tracked in **Redis** with a 300-second TTL: `last_seen_tx_id` per key.
- On first registration (no Redis entry), only transactions since server startup are delivered.
- Two receive modes: `recv()` (blocking) and `try_recv()` (non-blocking).

---

## 4. What Is Allowed

| Operation | Who | Notes |
|---|---|---|
| Send USD to any valid Fkebank key | Player (gRPC), Lua VM (with token) | Recipient must exist |
| Send crypto to any address | Player (gRPC), Lua VM (with priv key) | Recipient wallet auto-created |
| Receive USD / crypto | Any key/address | No permission needed to receive |
| Convert between currencies | Player (gRPC) | Subject to rate and minimum |
| Create virtual credit cards | Player | No limit on number of cards |
| Make purchases via card | Player, Lua | Up to credit limit |
| Pay credit card bill | Player | Must have USD balance >= debt |
| Delete a card (soft) | Player | Historical records preserved |
| View full transaction history | Player (own), Lua VM (with token) | Filters: today / 7d / 30d / all |
| View crypto history | Anyone with the address | No auth required |
| Listen for incoming transfers | Lua VM | Registered by key/address |

---

## 5. What Is Not Allowed

| Operation | Reason |
|---|---|
| Transfer to self (same key/address) | Explicitly rejected at service layer |
| Transfer with `amount <= 0` | Rejected at service layer |
| Transfer to non-existent USD key | `RecipientNotFound` — no money lost |
| Credit / debit without server authority | Not exposed in player gRPC or Lua API |
| Exceed credit card limit | Atomic DB constraint prevents it |
| Pay card bill with insufficient USD | DB transaction rolls back; debt unchanged |
| Create a crypto vault for another player | Vault key includes player UUID |
| Convert tiny amounts that round to 0 | `ConvertedAmountTooSmall` error |
| Access another player's USD history without their token | Token validates against account |

---

## 6. Security Vulnerabilities & Attack Vectors

The following are identified logic flaws and security gaps in the current implementation, ordered by severity.

---

### CRITICAL

#### VULN-01 — money.null Refund Daemon: Infinite Money Exploit

**File:** `nulltrace-core/src/lua_scripts/money_refund.lua`

**Description:** The refund daemon (`money.null`) automatically sends back **2× the received amount** to any sender.

```lua
-- Simplified logic
if currency == "USD" and amount >= 1 then
    fkebank.transfer(token_path, tx.from_key, amount * 2, "Double back")
end
```

**Attack:**
1. Player sends $1 (100 cents) to `money.null`.
2. `money.null` sends back $2 (200 cents). Net gain: $1.
3. Player repeats in a loop → exponential money generation until `money.null` balance is exhausted.

**Impact:** Any player can drain the `money.null` balance to zero and generate unlimited in-game currency. This breaks the entire in-game economy.

**Fix options:**
- Remove the 2× multiplier (refund exactly the amount received).
- Add a cooldown / rate limit per sender key.
- Cap the maximum refund amount.
- Require `money.null` to have sufficient balance before refunding.

---

#### VULN-02 — Crypto Private Key Not Verified in Lua Transfer Path

**File:** `nulltrace-core/src/cluster/db/crypto_wallet_service.rs` (`transfer()` function)

**Description:** The Lua API's `crypto.transfer()` reads the private key file and passes the content to `CryptoWalletService::transfer()`. However, the service has a `TODO` and **does not verify** that the private key cryptographically corresponds to `from_address`:

```rust
// TODO: verify private_key_content derives to from_address (secp256k1)
// Currently: only checks existence and balance
```

**Attack:**
- If an attacker can read *any* private key file (e.g., from a shared directory, a path traversal bug, or a misconfigured VM), they can call `crypto.transfer()` with:
  - `from_address` = victim's address
  - `priv_key_path` = path to any file (content doesn't matter — it's not checked)
- The transfer will succeed as long as the attacker's Lua script runs with correct parameters.

**Impact:** Theft of all crypto funds from any wallet whose address is known (all vault addresses are predictable: `player-vault-{uuid}-{currency}`).

**Fix:** Implement the secp256k1 (or equivalent) signature verification before applying the balance change.

---

### HIGH

#### VULN-03 — Credit Card Debt Without Repayment Capability (Intentional Fraud Vector)

**File:** `nulltrace-core/src/cluster/db/wallet_card_service.rs` (`pay_card_bill`)

**Description:** A player can:
1. Max out their credit card (up to `credit_limit`).
2. Transfer all USD to another account.
3. Attempt to pay the bill → fails with insufficient balance.
4. Retains goods/services purchased with the card.

The bill payment transaction rolls back entirely if USD is insufficient — the debt persists but nothing is collected.

**Impact:** Players can effectively get free in-game goods by abusing the card+transfer combo. While this may be intentional game design ("robbing the bank"), it could unbalance the economy if not designed consciously.

**Note:** If this is intentional, it should be documented as a game mechanic, not left as an implicit gap.

---

#### VULN-04 — Crypto Vault Addresses Are Predictable

**File:** `nulltrace-core/src/cluster/db/wallet_service.rs` (`get_balances`)

**Description:** Crypto vault keys follow the pattern: `player-vault-{player_uuid}-{currency}`.

Player UUIDs are likely visible in other parts of the game (friend lists, leaderboards, etc.). Any player who knows another player's UUID can:
- Derive their exact crypto vault addresses for all currencies.
- Send funds to those addresses (fine).
- Monitor those addresses' transaction history (crypto history has **no auth**).

**Impact:** Full financial surveillance of any player's crypto activity without their knowledge or consent.

**Fix:** Use a hashed or randomized vault address, or add authentication to `history_by_address()`.

---

#### VULN-05 — Crypto Transaction History Requires No Authentication

**File:** `nulltrace-core/src/cluster/db/crypto_wallet_service.rs` (`history_by_address`)

**Description:** Unlike USD (which requires a token), `history_by_address()` for crypto queries the DB and returns all transactions with no authorization check.

**Impact:** Anyone who knows a crypto address can see the full financial history of that wallet — sender, recipient, amount, timestamp.

**Note:** This mirrors how real blockchains work (public ledger). If this is intentional game design, it should be documented.

---

#### VULN-06 — Incoming Money Listener: Duplicate Transaction Delivery on Redis Expiry

**File:** `nulltrace-core/src/cluster/incoming_money_listener.rs`

**Description:** The listener uses Redis to track `last_seen_tx_id` with a **300-second TTL**. If a Lua VM is idle for more than 5 minutes:
1. Redis key expires.
2. On next poll, the listener falls back to `startup_time` as the cutoff.
3. All transactions since server startup (that weren't delivered before TTL) are re-delivered.

**Impact:**
- Duplicate transaction processing in the Lua VM.
- The `money_refund.lua` daemon could send double refunds for the same incoming transaction.
- Combined with VULN-01, this multiplies the exploit potential.

**Fix:** Use a persistent store (DB) for `last_seen_tx_id`, or significantly extend the TTL and handle duplicates idempotently in Lua (e.g., track processed `tx.id`).

---

### MEDIUM

#### VULN-07 — Credit Card CVV Stored in Plaintext

**File:** `nulltrace-core/migrations/020_create_wallet_cards.sql`

```sql
cvv VARCHAR(4)  -- stored as-is
```

**Description:** CVV values are stored in plaintext in the database. In a real payment system this would be a PCI DSS violation. In the game context, any DB breach exposes all card details.

**Fix:** Hash CVVs (BCrypt/Argon2) or, since these are virtual game cards, accept the risk and document it.

---

#### VULN-08 — No Maximum Transfer Amount

**Description:** There is no cap on the `amount` field for any transfer (USD or crypto). A player can transfer their entire balance in a single operation, which may not be intended behavior and makes certain economy exploits easier (e.g., VULN-03).

**Fix:** Consider defining per-transaction limits or daily transfer limits for balance.

---

#### VULN-09 — No Rate Limiting on Any Wallet Operation

**Description:** Transfer, conversion, and card purchase endpoints have no rate limiting at the service or gRPC layer. A player can:
- Spam conversions to probe rounding behavior.
- Repeatedly send $0.01 to `money.null` to drain it (VULN-01 amplified).
- Flood the transaction table with tiny-amount transfers.

**Fix:** Add per-player rate limits on transfer/conversion endpoints.

---

#### VULN-10 — `transfer()` vs `transfer_atomic()` Inconsistency in Crypto Service

**File:** `nulltrace-core/src/cluster/db/crypto_wallet_service.rs`

**Description:** Two different transfer functions exist for crypto:
- `transfer_atomic()`: Used for player-to-player transfers via the Rust wallet service. Properly handles the "create wallet if not exists" case atomically.
- `transfer()`: Used by the Lua API path. Has a `TODO` for private key verification and may update recipient balance **without first ensuring the wallet exists in DB**.

This inconsistency means the Lua path and the gRPC path have different atomicity guarantees and different security properties.

**Fix:** Consolidate to a single transfer path after implementing private key verification (VULN-02).

---

#### VULN-11 — money.null HTTP Server Leaks All Financial Data

**File:** `nulltrace-core/src/lua_scripts/money_httpd.lua`

**Description:** The `money.null` HTTP server on port 80 returns:
- Current USD balance and Fkebank key.
- All crypto addresses and balances.
- Last 100 transactions (sender, recipient, amount, timestamp).
- Refund errors and debug logs.

If this server is reachable by other players in the game world (as implied by its role), it exposes `money.null`'s complete financial state and operational details (including error messages that may reveal internal implementation).

**Fix:** Restrict to aggregate/anonymized data, or require authentication to view the full history.

---

### LOW / INFORMATIONAL

#### VULN-12 — Crypto Addresses Are Not Cryptographically Valid

**File:** `nulltrace-core/src/cluster/db/wallet_common.rs`

**Description:** Generated crypto addresses (`bc1q...`, `0x...`, base58) are randomly generated strings that mimic the format of real addresses but have no cryptographic validity (no keypair, no checksum, no derivation). If any code path ever tries to validate these against real blockchain rules, it will fail.

**Impact:** Low in a game context; only a concern if the game ever integrates real blockchain functionality.

---

#### VULN-13 — Statement Rollover Is Lazy (No Background Job)

**File:** `nulltrace-core/src/cluster/db/wallet_card_service.rs` (`get_or_create_open_statement`)

**Description:** Billing statement rollover only happens when `get_or_create_open_statement()` is explicitly called. If a player never accesses their card after the billing period ends, the statement remains "open" indefinitely.

**Impact:** The `due_date` shown to the player may be stale if they never interact with the card. Low impact but may cause confusion.

---

#### VULN-14 — No Audit Log for Failed Operations

**Description:** Failed transfers, failed bill payments, and failed conversions are not recorded anywhere. Only successful operations produce a `wallet_transactions` record.

**Impact:** No way to investigate suspicious activity (e.g., repeated failed transfers probing another account's balance, or brute-forcing token values).

**Fix:** Consider a `wallet_events` table for auditing failed operations.

---

## Summary Table

| ID | Severity | Title |
|---|---|---|
| VULN-01 | CRITICAL | money.null Refund Daemon — 2× infinite money |
| VULN-02 | CRITICAL | Crypto private key not verified in Lua path |
| VULN-03 | HIGH | Credit card debt without repayment capability |
| VULN-04 | HIGH | Crypto vault addresses are predictable |
| VULN-05 | HIGH | Crypto history requires no authentication |
| VULN-06 | HIGH | Duplicate transaction delivery on Redis TTL expiry |
| VULN-07 | MEDIUM | Credit card CVV stored in plaintext |
| VULN-08 | MEDIUM | No maximum transfer amount |
| VULN-09 | MEDIUM | No rate limiting on wallet operations |
| VULN-10 | MEDIUM | `transfer()` vs `transfer_atomic()` inconsistency |
| VULN-11 | MEDIUM | money.null HTTP server leaks all financial data |
| VULN-12 | LOW | Crypto addresses not cryptographically valid |
| VULN-13 | LOW | Statement rollover is lazy (no background job) |
| VULN-14 | LOW | No audit log for failed operations |
