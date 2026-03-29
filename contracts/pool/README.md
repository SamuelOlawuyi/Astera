# Funding Pool Contract — API Documentation

Soroban smart contract for pooled invoice funding on Stellar. Investors deposit accepted stablecoins, co-fund invoices, and earn time-based yield on repayments.

---

## Overview

The Funding Pool enables:

- **Multi-token support** — admin can whitelist multiple stablecoins (USDC, EURC, etc.)
- **Co-funding** — multiple investors combine capital to fund a single invoice
- **Pro-rata yield** — interest distributed proportionally to each co-funder's share
- **Position tracking** — per-investor, per-token balances and earnings

---

## Constants

| Constant | Value | Description |
| --- | --- | --- |
| `DEFAULT_YIELD_BPS` | `800` | Default annual yield (8% APY) |
| `BPS_DENOM` | `10_000` | Basis points denominator |
| `SECS_PER_YEAR` | `31_536_000` | Seconds per year (365 days) |

---

## Data Structures

### PoolConfig

```rust
pub struct PoolConfig {
    pub invoice_contract: Address,  // Invoice contract address
    pub admin: Address,             // Pool administrator
    pub yield_bps: u32,             // Annual yield in basis points (800 = 8%)
}
```

### PoolTokenTotals

Per-token aggregate statistics.

```rust
pub struct PoolTokenTotals {
    pub total_deposited: i128,  // Total deposited (including earned interest)
    pub total_deployed: i128,   // Currently locked in active invoices
    pub total_paid_out: i128,   // Total paid out (principal + interest)
}
```

### InvestorPosition

Per-investor, per-token position tracking.

```rust
pub struct InvestorPosition {
    pub deposited: i128,     // Total amount deposited (net of withdrawals)
    pub available: i128,     // Undeployed balance available for withdrawal/funding
    pub deployed: i128,      // Amount locked in active invoices
    pub earned: i128,        // Total interest earned
    pub deposit_count: u32,  // Number of deposits made
}
```

### FundedInvoice

Tracks a single invoice's funding state.

```rust
pub struct FundedInvoice {
    pub invoice_id: u64,   // Invoice ID from invoice contract
    pub sme: Address,      // SME receiving the funds
    pub token: Address,    // Stablecoin used for this invoice
    pub principal: i128,   // Total funding target
    pub committed: i128,   // Amount committed so far
    pub funded_at: u64,    // Timestamp when fully funded (0 while open)
    pub due_date: u64,     // Invoice due date
    pub repaid: bool,      // Whether invoice has been repaid
}
```

### CoFundKey

Composite key for per-investor, per-invoice share records.

```rust
pub struct CoFundKey {
    pub invoice_id: u64,
    pub investor: Address,
}
```

### InvestorTokenKey

Composite key for per-investor, per-token position records.

```rust
pub struct InvestorTokenKey {
    pub investor: Address,
    pub token: Address,
}
```

### DataKey (Storage Keys)

```rust
pub enum DataKey {
    Config,                             // Instance: PoolConfig
    InvestorPosition(InvestorTokenKey),  // Persistent: per-investor, per-token position
    FundedInvoice(u64),                 // Persistent: per-invoice funding record
    CoFunders(u64),                     // Persistent: Vec<Address> of co-funders
    CoFundShare(CoFundKey),             // Persistent: i128 share per investor per invoice
    AcceptedTokens,                     // Instance: Vec<Address> of whitelisted tokens
    TokenTotals(Address),               // Instance: per-token PoolTokenTotals
    Initialized,                        // Instance: initialization flag
}
```

---

## Events

All events use topic prefix `POOL` (via `symbol_short!("POOL")`).

| Event | Topic | Data | Emitted When |
| --- | --- | --- | --- |
| `deposit` | `(POOL, "deposit")` | `(investor: Address, amount: i128)` | Investor deposits stablecoin |
| `funded` | `(POOL, "funded")` | `(invoice_id: u64, sme: Address, principal: i128)` | Invoice fully funded; stablecoin disbursed to SME |
| `repaid` | `(POOL, "repaid")` | `(invoice_id: u64, principal: i128, interest: i128)` | Invoice repaid with interest |
| `withdraw` | `(POOL, "withdraw")` | `(investor: Address, amount: i128)` | Investor withdraws stablecoin |

---

## Public Functions

### initialize

```rust
pub fn initialize(env: Env, admin: Address, initial_token: Address, invoice_contract: Address)
```

Initializes the pool with an admin, the first accepted stablecoin, and the invoice contract. Can only be called once.

| Parameter | Type | Description |
| --- | --- | --- |
| `admin` | `Address` | Protocol administrator |
| `initial_token` | `Address` | First accepted stablecoin (e.g. USDC) |
| `invoice_contract` | `Address` | Invoice contract address |

**Auth:** None (single-use)

**Panics:**
- `"already initialized"` — contract already initialized

**Example:**
```bash
stellar contract invoke --id <POOL_ID> --source <ADMIN> \
  -- initialize \
  --admin <ADMIN_ADDR> \
  --initial_token <USDC_ADDR> \
  --invoice_contract <INVOICE_ADDR>
```

---

### add_token

```rust
pub fn add_token(env: Env, admin: Address, token: Address)
```

Admin whitelists a new stablecoin for the pool.

| Parameter | Type | Description |
| --- | --- | --- |
| `admin` | `Address` | Must be pool admin (must sign) |
| `token` | `Address` | Stablecoin contract to add |

**Auth:** `admin.require_auth()`

**Panics:**
- `"unauthorized"` — caller is not admin
- `"token already accepted"` — token is already whitelisted

---

### remove_token

```rust
pub fn remove_token(env: Env, admin: Address, token: Address)
```

Admin removes a stablecoin from the whitelist. Only allowed if the token has zero balances.

| Parameter | Type | Description |
| --- | --- | --- |
| `admin` | `Address` | Must be pool admin (must sign) |
| `token` | `Address` | Stablecoin contract to remove |

**Auth:** `admin.require_auth()`

**Panics:**
- `"unauthorized"` — caller is not admin
- `"token not in whitelist"` — token not found
- `"token has non-zero pool balances"` — token has active deposits/deployments

---

### deposit

```rust
pub fn deposit(env: Env, investor: Address, token: Address, amount: i128)
```

Investor deposits an accepted stablecoin into the pool. Token is transferred from investor to the pool contract.

| Parameter | Type | Description |
| --- | --- | --- |
| `investor` | `Address` | Investor wallet (must sign) |
| `token` | `Address` | Stablecoin to deposit |
| `amount` | `i128` | Amount in smallest unit (e.g. `1_000_000_000` = 100 USDC) |

**Auth:** `investor.require_auth()`

**Panics:**
- `"amount must be positive"` — amount ≤ 0
- `"token not accepted"` — token not in whitelist

**Events:** `deposit` — `(investor, amount)`

**Example:**
```bash
stellar contract invoke --id <POOL_ID> --source <INVESTOR> \
  -- deposit \
  --investor <INVESTOR_ADDR> \
  --token <USDC_ADDR> \
  --amount 1000000000
```

---

### init_co_funding

```rust
pub fn init_co_funding(
    env: Env,
    admin: Address,
    invoice_id: u64,
    principal: i128,
    sme: Address,
    due_date: u64,
    token: Address,
)
```

Admin registers an invoice for co-funding in a specific stablecoin. Investors then commit capital via `commit_to_invoice`.

| Parameter | Type | Description |
| --- | --- | --- |
| `admin` | `Address` | Pool admin (must sign) |
| `invoice_id` | `u64` | Invoice ID from the invoice contract |
| `principal` | `i128` | Total funding target |
| `sme` | `Address` | SME wallet to receive funds |
| `due_date` | `u64` | Invoice due date (unix timestamp) |
| `token` | `Address` | Stablecoin to use for this invoice |

**Auth:** `admin.require_auth()`

**Panics:**
- `"unauthorized"` — caller is not admin
- `"token not accepted"` — token not in whitelist
- `"principal must be positive"` — principal ≤ 0
- `"invoice already registered for funding"` — duplicate invoice_id

**Example:**
```bash
stellar contract invoke --id <POOL_ID> --source <ADMIN> \
  -- init_co_funding \
  --admin <ADMIN_ADDR> \
  --invoice_id 1 \
  --principal 3000000000 \
  --sme <SME_ADDR> \
  --due_date 1735689600 \
  --token <USDC_ADDR>
```

---

### commit_to_invoice

```rust
pub fn commit_to_invoice(env: Env, investor: Address, invoice_id: u64, amount: i128)
```

Investor commits available balance toward funding an invoice. When total committed reaches the principal target, the stablecoin is disbursed to the SME automatically.

| Parameter | Type | Description |
| --- | --- | --- |
| `investor` | `Address` | Investor wallet (must sign) |
| `invoice_id` | `u64` | Invoice to contribute to |
| `amount` | `i128` | Amount to commit |

**Auth:** `investor.require_auth()`

**Panics:**
- `"amount must be positive"` — amount ≤ 0
- `"invoice not registered for co-funding"` — invalid invoice_id
- `"invoice already fully funded"` — invoice is fully funded
- `"invoice already repaid"` — invoice has been repaid
- `"amount exceeds remaining funding gap"` — amount > remaining needed
- `"investor has no position in this invoice token"` — investor has no deposits in the invoice's token
- `"insufficient available balance"` — not enough available balance

**Events:** `funded` — `(invoice_id, sme, principal)` when invoice becomes fully funded

---

### repay_invoice

```rust
pub fn repay_invoice(env: Env, invoice_id: u64, payer: Address)
```

SME repays the invoice. The payer transfers `principal + accrued interest` to the pool. Each co-funder's position is credited with their proportional share of principal + interest.

**Interest formula:**
```
interest = (principal × yield_bps × elapsed_seconds) / (BPS_DENOM × SECS_PER_YEAR)
```

| Parameter | Type | Description |
| --- | --- | --- |
| `invoice_id` | `u64` | Invoice to repay |
| `payer` | `Address` | Wallet paying the repayment (must sign) |

**Auth:** `payer.require_auth()`

**Panics:**
- `"invoice not found"` — invalid invoice_id
- `"invoice not fully funded yet"` — invoice hasn't been fully funded
- `"already repaid"` — invoice already repaid

**Events:** `repaid` — `(invoice_id, principal, interest)`

---

### withdraw

```rust
pub fn withdraw(env: Env, investor: Address, token: Address, amount: i128)
```

Investor withdraws available (undeployed) balance from the pool.

| Parameter | Type | Description |
| --- | --- | --- |
| `investor` | `Address` | Investor wallet (must sign) |
| `token` | `Address` | Stablecoin to withdraw |
| `amount` | `i128` | Amount to withdraw |

**Auth:** `investor.require_auth()`

**Panics:**
- `"amount must be positive"` — amount ≤ 0
- `"token not accepted"` — token not in whitelist
- `"no position found"` — investor has no position
- `"insufficient available balance"` — not enough available balance

**Events:** `withdraw` — `(investor, amount)`

---

### set_yield

```rust
pub fn set_yield(env: Env, admin: Address, yield_bps: u32)
```

Admin updates the pool's annual yield rate.

| Parameter | Type | Description |
| --- | --- | --- |
| `admin` | `Address` | Pool admin (must sign) |
| `yield_bps` | `u32` | New yield in basis points (e.g. `1000` = 10%) |

**Auth:** `admin.require_auth()`

**Panics:**
- `"unauthorized"` — caller is not admin
- `"yield cannot exceed 50%"` — yield_bps > 5000

---

## Read-Only Functions

### get_config

```rust
pub fn get_config(env: Env) -> PoolConfig
```

Returns the pool configuration.

**Panics:** `"not initialized"`

---

### accepted_tokens

```rust
pub fn accepted_tokens(env: Env) -> Vec<Address>
```

Returns the list of whitelisted stablecoin addresses.

**Panics:** `"not initialized"`

---

### get_token_totals

```rust
pub fn get_token_totals(env: Env, token: Address) -> PoolTokenTotals
```

Returns aggregate deposit/deployment/payout totals for a specific token.

---

### get_position

```rust
pub fn get_position(env: Env, investor: Address, token: Address) -> Option<InvestorPosition>
```

Returns the investor's position for a specific token. Returns `None` if no position exists.

---

### get_funded_invoice

```rust
pub fn get_funded_invoice(env: Env, invoice_id: u64) -> Option<FundedInvoice>
```

Returns funding details for an invoice. Returns `None` if not registered.

---

### get_co_fund_share

```rust
pub fn get_co_fund_share(env: Env, invoice_id: u64, investor: Address) -> i128
```

Returns the amount an investor has committed to a specific invoice. Returns `0` if no commitment.

---

### available_liquidity

```rust
pub fn available_liquidity(env: Env, token: Address) -> i128
```

Returns undeployed liquidity for a specific token: `total_deposited - total_deployed`.

---

### estimate_repayment

```rust
pub fn estimate_repayment(env: Env, invoice_id: u64) -> i128
```

Estimates total repayment amount at the current time (principal + accrued interest).

**Panics:**
- `"not initialized"` — contract not initialized
- `"invoice not funded"` — invoice doesn't exist

---

## Error Codes Summary

| Error Message | Cause |
| --- | --- |
| `"already initialized"` | Contract already initialized |
| `"not initialized"` | Functions called before initialization |
| `"unauthorized"` | Caller is not admin |
| `"amount must be positive"` | Amount ≤ 0 |
| `"principal must be positive"` | Principal ≤ 0 |
| `"token already accepted"` | Duplicate token in whitelist |
| `"token not in whitelist"` | Removing non-existent token |
| `"token has non-zero pool balances"` | Removing token with active balances |
| `"token not accepted"` | Using non-whitelisted token |
| `"invoice already registered for funding"` | Duplicate invoice_id |
| `"invoice not registered for co-funding"` | Invalid invoice_id |
| `"invoice already fully funded"` | Committing to fully funded invoice |
| `"invoice already repaid"` | Committing to repaid invoice |
| `"amount exceeds remaining funding gap"` | Commitment exceeds remaining needed |
| `"investor has no position in this invoice token"` | No deposits in the invoice's token |
| `"insufficient available balance"` | Available balance too low |
| `"invoice not found"` | Invalid invoice_id in repay |
| `"invoice not fully funded yet"` | Repaying before fully funded |
| `"already repaid"` | Duplicate repayment |
| `"no position found"` | No position for withdrawal |
| `"yield cannot exceed 50%"` | yield_bps > 5000 |

---

## Interest Calculation

```
interest = (principal × yield_bps × elapsed_seconds) / (10_000 × 31_536_000)
```

- `principal` — invoice face value
- `yield_bps` — annual yield in basis points (e.g. 800 = 8%)
- `elapsed_seconds` — time between `funded_at` and repayment
- Interest is distributed proportionally to co-funders based on their share of the principal

---

## Co-Funding Workflow

```
1. Admin:    init_co_funding(invoice_id, principal, sme, due_date, token)
2. Investor: commit_to_invoice(investor, invoice_id, amount)
   ... repeat until committed == principal ...
3. Auto:     Stablecoin disbursed to SME; "funded" event emitted
4. SME:      repay_invoice(invoice_id, payer)
5. Auto:     Principal + interest credited to each co-funder's available balance
```

```
┌──────────┐     commit_to_invoice()    ┌──────────────┐     repay_invoice()     ┌──────────┐
│  Open    │ ────────────────────────► │ Fully Funded │ ────────────────────► │  Repaid  │
│ (co-fund)│  (repeatable by any       └──────────────┘  (principal + yield    └──────────┘
└──────────┘   investor until full)         │              returned to pool)
                                            │
                                            │ Stablecoin auto-transferred
                                            │ to SME on full commitment
                                            ▼
                                       SME receives funds
```
