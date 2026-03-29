# Invoice Contract — API Documentation

Soroban smart contract for tokenizing receivables on Stellar. Each invoice is stored as an [`Invoice`](src/lib.rs) record with a full lifecycle from `Pending` → `Funded` → `Paid` (or `Defaulted`).

---

## Data Structures

### InvoiceStatus

```rust
pub enum InvoiceStatus {
    Pending,    // Created by SME, awaiting funding
    Funded,     // Funded by the pool contract
    Paid,       // Repayment confirmed
    Defaulted,  // Missed due date, no repayment
}
```

### Invoice

```rust
pub struct Invoice {
    pub id: u64,              // Unique invoice identifier (auto-incremented)
    pub owner: Address,       // SME who created the invoice
    pub debtor: String,       // Counterparty / debtor label
    pub amount: i128,         // Face value in smallest unit (7 decimals for USDC)
    pub due_date: u64,        // Unix timestamp when payment is due
    pub description: String,  // Human-readable description
    pub status: InvoiceStatus,// Current lifecycle status
    pub created_at: u64,      // Ledger timestamp at creation
    pub funded_at: u64,       // Ledger timestamp when funded (0 if not funded)
    pub paid_at: u64,         // Ledger timestamp when paid (0 if not paid)
    pub pool_contract: Address, // Pool contract that funded this invoice
}
```

### InvoiceMetadata

Wallet- and explorer-friendly view derived from `Invoice` (no extra storage).

```rust
pub struct InvoiceMetadata {
    pub name: String,         // e.g. "Astera Invoice #1"
    pub description: String,  // Same as on-chain invoice description
    pub image: String,        // Placeholder artwork URI
    pub amount: i128,         // Face value in smallest unit
    pub debtor: String,       // Counterparty label
    pub due_date: u64,        // Unix timestamp
    pub status: InvoiceStatus,// Current status
    pub symbol: String,       // Short ticker e.g. "INV-1" (SEP-0041 style)
    pub decimals: u32,        // 7 (USDC on Stellar)
}
```

### DataKey (Storage Keys)

```rust
pub enum DataKey {
    Invoice(u64),    // Persistent: individual invoice by ID
    InvoiceCount,    // Instance: total number of invoices created
    Admin,           // Instance: admin address
    Pool,            // Instance: authorized pool contract address
    Initialized,     // Instance: initialization flag
}
```

---

## Events

All events use topic prefix `INVOICE` (via `symbol_short!("INVOICE")`).

| Event | Topic | Data | Emitted When |
| --- | --- | --- | --- |
| `created` | `(INVOICE, "created")` | `(id: u64, owner: Address, amount: i128)` | New invoice created |
| `funded` | `(INVOICE, "funded")` | `id: u64` | Invoice marked as funded |
| `paid` | `(INVOICE, "paid")` | `id: u64` | Invoice marked as paid |
| `default` | `(INVOICE, "default")` | `id: u64` | Invoice marked as defaulted |

---

## Public Functions

### initialize

```rust
pub fn initialize(env: Env, admin: Address, pool: Address)
```

Initializes the contract with admin and authorized pool addresses. Can only be called once.

| Parameter | Type | Description |
| --- | --- | --- |
| `admin` | `Address` | Protocol administrator |
| `pool` | `Address` | Authorized pool contract address |

**Auth:** None (but can only be called once)

**Panics:**
- `"already initialized"` — contract already initialized

**Example:**
```bash
stellar contract invoke --id <CONTRACT_ID> --source <ADMIN> \
  -- initialize --admin <ADMIN_ADDR> --pool <POOL_ADDR>
```

---

### create_invoice

```rust
pub fn create_invoice(
    env: Env,
    owner: Address,
    debtor: String,
    amount: i128,
    due_date: u64,
    description: String,
) -> u64
```

SME creates a new invoice token on-chain. Returns the auto-incremented invoice ID.

| Parameter | Type | Description |
| --- | --- | --- |
| `owner` | `Address` | SME wallet address (must sign) |
| `debtor` | `String` | Name of the debtor / counterparty |
| `amount` | `i128` | Face value in smallest unit (e.g. `1_000_000_000` = 100 USDC) |
| `due_date` | `u64` | Unix timestamp; must be in the future |
| `description` | `String` | Human-readable description of the invoice |

**Returns:** `u64` — the new invoice ID

**Auth:** `owner.require_auth()`

**Panics:**
- `"amount must be positive"` — amount ≤ 0
- `"due date must be in the future"` — due_date ≤ current ledger timestamp

**Events:** `created` — `(id, owner, amount)`

**Example:**
```bash
stellar contract invoke --id <CONTRACT_ID> --source <SME> \
  -- create_invoice \
  --owner <SME_ADDR> \
  --debtor "ACME Corp" \
  --amount 1000000000 \
  --due_date 1735689600 \
  --description "Invoice #001 - Goods delivery"
```

---

### mark_funded

```rust
pub fn mark_funded(env: Env, id: u64, pool: Address)
```

Called by the authorized pool contract when it funds an invoice. Transitions status from `Pending` → `Funded`.

| Parameter | Type | Description |
| --- | --- | --- |
| `id` | `u64` | Invoice ID to mark as funded |
| `pool` | `Address` | Pool contract address (must match stored pool; must sign) |

**Auth:** `pool.require_auth()`

**Panics:**
- `"not initialized"` — contract not initialized
- `"unauthorized pool"` — caller is not the authorized pool
- `"invoice not found"` — invalid invoice ID
- `"invoice is not pending"` — invoice status is not `Pending`

**Events:** `funded` — `id`

---

### mark_paid

```rust
pub fn mark_paid(env: Env, id: u64, caller: Address)
```

Marks a funded invoice as paid. Can be called by the invoice owner, pool, or admin.

| Parameter | Type | Description |
| --- | --- | --- |
| `id` | `u64` | Invoice ID to mark as paid |
| `caller` | `Address` | Must be one of: invoice owner, pool, or admin |

**Auth:** `caller.require_auth()`

**Panics:**
- `"not initialized"` — contract not initialized
- `"invoice not found"` — invalid invoice ID
- `"unauthorized"` — caller is not owner, pool, or admin
- `"invoice is not funded"` — invoice status is not `Funded`

**Events:** `paid` — `id`

---

### mark_defaulted

```rust
pub fn mark_defaulted(env: Env, id: u64, pool: Address)
```

Marks a funded invoice as defaulted. Only callable by the authorized pool contract.

| Parameter | Type | Description |
| --- | --- | --- |
| `id` | `u64` | Invoice ID to mark as defaulted |
| `pool` | `Address` | Authorized pool contract address (must sign) |

**Auth:** `pool.require_auth()`

**Panics:**
- `"not initialized"` — contract not initialized
- `"unauthorized pool"` — caller is not the authorized pool
- `"invoice not found"` — invalid invoice ID
- `"invoice is not funded"` — invoice status is not `Funded`

**Events:** `default` — `id`

---

### get_invoice

```rust
pub fn get_invoice(env: Env, id: u64) -> Invoice
```

Returns the full `Invoice` record for the given ID.

| Parameter | Type | Description |
| --- | --- | --- |
| `id` | `u64` | Invoice ID |

**Returns:** `Invoice`

**Auth:** None (read-only)

**Panics:**
- `"invoice not found"` — invalid invoice ID

---

### get_metadata

```rust
pub fn get_metadata(env: Env, id: u64) -> InvoiceMetadata
```

Returns SEP-oriented metadata for wallet/explorer display. Derives `name`, `symbol`, and other display fields from the stored invoice.

| Parameter | Type | Description |
| --- | --- | --- |
| `id` | `u64` | Invoice ID |

**Returns:** `InvoiceMetadata`

**Auth:** None (read-only)

**Panics:**
- `"invoice not found"` — invalid invoice ID

**JSON example (off-chain projection):**
```json
{
  "name": "Astera Invoice #1",
  "description": "Invoice #001 - Goods delivery",
  "image": "https://astera.io/metadata/invoice/placeholder.svg",
  "amount": "1000000000",
  "debtor": "ACME Corp",
  "due_date": 1735689600,
  "status": "Pending",
  "symbol": "INV-1",
  "decimals": 7
}
```

---

### get_invoice_count

```rust
pub fn get_invoice_count(env: Env) -> u64
```

Returns the total number of invoices created.

**Returns:** `u64`

**Auth:** None (read-only)

---

### set_pool

```rust
pub fn set_pool(env: Env, admin: Address, pool: Address)
```

Updates the authorized pool contract address (admin only).

| Parameter | Type | Description |
| --- | --- | --- |
| `admin` | `Address` | Must be the stored admin (must sign) |
| `pool` | `Address` | New pool contract address |

**Auth:** `admin.require_auth()`

**Panics:**
- `"not initialized"` — contract not initialized
- `"unauthorized"` — caller is not admin

---

## Error Codes Summary

| Error Message | Cause |
| --- | --- |
| `"already initialized"` | Contract already initialized |
| `"not initialized"` | Functions called before initialization |
| `"amount must be positive"` | Invoice amount ≤ 0 |
| `"due date must be in the future"` | Due date ≤ current ledger timestamp |
| `"unauthorized pool"` | Caller is not the authorized pool contract |
| `"unauthorized"` | Caller lacks permission for the operation |
| `"invoice not found"` | Invalid invoice ID |
| `"invoice is not pending"` | Expected `Pending` status but got something else |
| `"invoice is not funded"` | Expected `Funded` status but got something else |

---

## SEP Alignment

- **[SEP-0041](https://github.com/stellar/stellar-protocol/blob/master/ecosystem/sep-0041.md)** — `name`, `symbol`, and `decimals` on `InvoiceMetadata` map to the Soroban token interface. Invoice face value uses `amount` + `decimals` for consistent formatting.
- **NFT metadata conventions** — `name`, `description`, and `image` fields match common NFT/token metadata shapes for wallet display.

---

## Invoice Lifecycle

```
┌─────────┐     mark_funded()     ┌────────┐     mark_paid()      ┌──────┐
│ Pending │ ──────────────────► │ Funded │ ──────────────────► │ Paid │
└─────────┘                      └────────┘                      └──────┘
                                      │
                                      │ mark_defaulted()
                                      ▼
                                 ┌───────────┐
                                 │ Defaulted │
                                 └───────────┘
```
