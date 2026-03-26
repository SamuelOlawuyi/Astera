# Testnet Deployment Walkthrough

This guide walks you through deploying the Astera invoice financing platform to Stellar testnet from scratch. Follow each step carefully and verify the output matches the expected results.

---

## Prerequisites

Before starting, ensure you have the following tools installed:

### 1. Install Rust and Cargo

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source $HOME/.cargo/env
```

Verify installation:

```bash
rustc --version
cargo --version
```

Expected output:

```
rustc 1.75.0 (or later)
cargo 1.75.0 (or later)
```

### 2. Add wasm32-unknown-unknown target

```bash
rustup target add wasm32-unknown-unknown
```

Expected output:

```
info: downloading component 'rust-std' for 'wasm32-unknown-unknown'
info: installing component 'rust-std' for 'wasm32-unknown-unknown'
```

### 3. Install Stellar CLI

```bash
cargo install --locked stellar-cli --features opt
```

Verify installation:

```bash
stellar --version
```

Expected output:

```
stellar 21.0.0 (or later)
```

### 4. Install Node.js 20+

Download from [nodejs.org](https://nodejs.org/) or use a version manager:

```bash
# Using nvm
nvm install 20
nvm use 20
```

Verify installation:

```bash
node --version
npm --version
```

Expected output:

```
v20.x.x (or later)
10.x.x (or later)
```

### 5. Install Freighter Wallet

Install the [Freighter browser extension](https://www.freighter.app/) and create a wallet. Make sure to:

- Save your recovery phrase securely
- Switch to Testnet in Freighter settings

---

## Step 1: Generate and Fund Testnet Keypair

### Generate a new keypair

```bash
stellar keys generate deployer --network testnet
```

Expected output:

```
Secret key: S...
Public key: G...
```

The keypair is saved globally in your Stellar CLI configuration.

### View your public address

```bash
stellar keys address deployer
```

Expected output:

```
GXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXX
```

### Fund your testnet account

```bash
stellar keys fund deployer --network testnet
```

Expected output:

```
Funded deployer with 10000 XLM
```

This gives you 10,000 testnet XLM to cover deployment and transaction fees.

### Verify account balance

```bash
stellar keys balance deployer --network testnet
```

Expected output:

```
10000.0000000 XLM
```

---

## Step 2: Build Contracts for WASM Release

Navigate to the project root and build both contracts:

```bash
cd astera
cargo build --target wasm32-unknown-unknown --release
```

Expected output:

```
   Compiling soroban-sdk v21.x.x
   Compiling invoice v0.1.0 (/path/to/astera/contracts/invoice)
   Compiling pool v0.1.0 (/path/to/astera/contracts/pool)
    Finished release [optimized] target(s) in X.XXs
```

### Verify WASM files were created

```bash
ls -lh target/wasm32-unknown-unknown/release/*.wasm
```

Expected output:

```
-rwxr-xr-x  invoice.wasm
-rwxr-xr-x  pool.wasm
```

---

## Step 3: Deploy Invoice Contract

Deploy the invoice contract to testnet:

```bash
stellar contract deploy \
  --wasm target/wasm32-unknown-unknown/release/invoice.wasm \
  --source deployer \
  --network testnet
```

Expected output:

```
CXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXX
```

**IMPORTANT:** Save this contract ID! You'll need it for initialization and frontend configuration.

Set it as an environment variable for convenience:

```bash
export INVOICE_CONTRACT_ID=CXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXX
echo "Invoice Contract ID: $INVOICE_CONTRACT_ID"
```

---

## Step 4: Deploy Pool Contract

Deploy the pool contract to testnet:

```bash
stellar contract deploy \
  --wasm target/wasm32-unknown-unknown/release/pool.wasm \
  --source deployer \
  --network testnet
```

Expected output:

```
CXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXX
```

**IMPORTANT:** Save this contract ID as well!

Set it as an environment variable:

```bash
export POOL_CONTRACT_ID=CXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXX
echo "Pool Contract ID: $POOL_CONTRACT_ID"
```

---

## Step 5: Get Testnet USDC Token Address

Stellar testnet has a native USDC token. Get the token address:

```bash
stellar contract id asset \
  --asset USDC:GBBD47IF6LWK7P7MDEVSCWR7DPUWV3NY3DTQEVFL4NAT4AQH3ZLLFLA5 \
  --network testnet
```

Expected output:

```
CBIELTK6YBZJU5UP2WWQEUCYKLPU6AUNZ2BQ4WWFEIE3USCIHMXQDAMA
```

Set it as an environment variable:

```bash
export USDC_TOKEN_ID=CBIELTK6YBZJU5UP2WWQEUCYKLPU6AUNZ2BQ4WWFEIE3USCIHMXQDAMA
echo "USDC Token ID: $USDC_TOKEN_ID"
```

---

## Step 6: Initialize Invoice Contract

Get your deployer address:

```bash
export DEPLOYER_ADDRESS=$(stellar keys address deployer)
echo "Deployer Address: $DEPLOYER_ADDRESS"
```

Initialize the invoice contract with admin and pool addresses:

```bash
stellar contract invoke \
  --id $INVOICE_CONTRACT_ID \
  --source deployer \
  --network testnet \
  -- initialize \
  --admin $DEPLOYER_ADDRESS \
  --pool $POOL_CONTRACT_ID
```

Expected output:

```
null
```

A `null` response indicates successful initialization with no return value.

### Verify initialization

Check the invoice count (should be 0):

```bash
stellar contract invoke \
  --id $INVOICE_CONTRACT_ID \
  --network testnet \
  -- get_invoice_count
```

Expected output:

```
0
```

---

## Step 7: Initialize Pool Contract

Initialize the pool contract with admin, USDC token, and invoice contract addresses:

```bash
stellar contract invoke \
  --id $POOL_CONTRACT_ID \
  --source deployer \
  --network testnet \
  -- initialize \
  --admin $DEPLOYER_ADDRESS \
  --usdc_token $USDC_TOKEN_ID \
  --invoice_contract $INVOICE_CONTRACT_ID
```

Expected output:

```
null
```

### Verify initialization

Check the pool configuration:

```bash
stellar contract invoke \
  --id $POOL_CONTRACT_ID \
  --network testnet \
  -- get_config
```

Expected output:

```json
{
  "admin": "GXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXX",
  "invoice_contract": "CXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXX",
  "total_deployed": "0",
  "total_deposited": "0",
  "total_paid_out": "0",
  "usdc_token": "CBIELTK6YBZJU5UP2WWQEUCYKLPU6AUNZ2BQ4WWFEIE3USCIHMXQDAMA",
  "yield_bps": 800
}
```

---

## Step 8: Mint Testnet USDC for Testing

To test the full flow, you'll need testnet USDC. First, establish a trustline to USDC:

```bash
stellar contract invoke \
  --id $USDC_TOKEN_ID \
  --source deployer \
  --network testnet \
  -- mint \
  --to $DEPLOYER_ADDRESS \
  --amount 10000000000
```

Expected output:

```
null
```

This mints 1,000 USDC (10000000000 stroops, USDC has 7 decimals) to your deployer address.

### Verify USDC balance

```bash
stellar contract invoke \
  --id $USDC_TOKEN_ID \
  --network testnet \
  -- balance \
  --id $DEPLOYER_ADDRESS
```

Expected output:

```
"10000000000"
```

---

## Step 9: Configure Frontend Environment

Navigate to the frontend directory:

```bash
cd frontend
```

Copy the example environment file:

```bash
cp .env.example .env.local
```

Edit `.env.local` and add your deployed contract IDs:

```bash
# Use your actual contract IDs from the deployment steps
cat > .env.local << EOF
NEXT_PUBLIC_INVOICE_CONTRACT_ID=$INVOICE_CONTRACT_ID
NEXT_PUBLIC_POOL_CONTRACT_ID=$POOL_CONTRACT_ID
NEXT_PUBLIC_USDC_TOKEN_ID=$USDC_TOKEN_ID
EOF
```

Verify the configuration:

```bash
cat .env.local
```

Expected output:

```
NEXT_PUBLIC_INVOICE_CONTRACT_ID=CXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXX
NEXT_PUBLIC_POOL_CONTRACT_ID=CXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXX
NEXT_PUBLIC_USDC_TOKEN_ID=CBIELTK6YBZJU5UP2WWQEUCYKLPU6AUNZ2BQ4WWFEIE3USCIHMXQDAMA
```

---

## Step 10: Install Frontend Dependencies

Install npm packages:

```bash
npm install
```

Expected output:

```
added XXX packages, and audited XXX packages in Xs

XXX packages are looking for funding
  run `npm fund` for details

found 0 vulnerabilities
```

---

## Step 11: Run the Frontend

Start the development server:

```bash
npm run dev
```

Expected output:

```
   ▲ Next.js 14.x.x
   - Local:        http://localhost:3000
   - Environments: .env.local

 ✓ Ready in X.Xs
```

---

## Step 12: Verify End-to-End Flow

### 1. Open the application

Navigate to [http://localhost:3000](http://localhost:3000) in your browser.

### 2. Connect Freighter wallet

- Click "Connect Wallet" in the navbar
- Approve the connection in Freighter
- Ensure you're on Testnet in Freighter settings

### 3. Test SME Flow (Create Invoice)

- Go to "New Invoice" page
- Fill in the form:
  - Debtor: "ACME Corp"
  - Amount: 1000 (USDC)
  - Due Date: Select a future date
  - Description: "Test Invoice #1"
- Click "Create Invoice"
- Approve the transaction in Freighter
- You should see a success message with the invoice ID

### 4. Test Investor Flow (Deposit to Pool)

- Go to "Invest" page
- Enter deposit amount: 5000 (USDC)
- Click "Deposit"
- Approve the transaction in Freighter
- Your pool balance should update

### 5. Test Admin Flow (Fund Invoice)

As the deployer/admin, you can fund invoices from the CLI:

```bash
# First, initialize co-funding for the invoice
stellar contract invoke \
  --id $POOL_CONTRACT_ID \
  --source deployer \
  --network testnet \
  -- init_co_funding \
  --admin $DEPLOYER_ADDRESS \
  --invoice_id 1 \
  --principal 10000000000 \
  --sme $DEPLOYER_ADDRESS \
  --due_date 1735689600

# Then commit funds to the invoice
stellar contract invoke \
  --id $POOL_CONTRACT_ID \
  --source deployer \
  --network testnet \
  -- commit_to_invoice \
  --investor $DEPLOYER_ADDRESS \
  --invoice_id 1 \
  --amount 10000000000
```

### 6. Test Repayment Flow

```bash
stellar contract invoke \
  --id $POOL_CONTRACT_ID \
  --source deployer \
  --network testnet \
  -- repay_invoice \
  --invoice_id 1 \
  --payer $DEPLOYER_ADDRESS
```

### 7. Verify on Dashboard

- Go to "Dashboard" page
- You should see your created invoice with status "Funded" or "Paid"
- Check your investor position showing deposits, deployed capital, and earned interest

---

## Troubleshooting

### Contract deployment fails

**Error:** `error: account not found`

**Solution:** Make sure your deployer account is funded:

```bash
stellar keys fund deployer --network testnet
```

### Contract initialization fails

**Error:** `already initialized`

**Solution:** The contract has already been initialized. You can either:

- Use the existing deployment
- Deploy a new contract instance

### USDC mint fails

**Error:** `unauthorized`

**Solution:** On testnet, you may need to use the official testnet USDC faucet or wrap native XLM. Alternatively, deploy your own test token.

### Frontend can't connect to contracts

**Error:** Contract calls fail or return errors

**Solution:**

- Verify contract IDs in `.env.local` are correct
- Ensure Freighter is connected and on Testnet
- Check browser console for detailed error messages
- Verify contracts are initialized properly

### Transaction fails with "insufficient balance"

**Solution:** Ensure your wallet has:

- Enough XLM for transaction fees (minimum 1 XLM)
- Enough USDC for the operation you're performing

---

## Summary

You've successfully:

1. ✅ Installed all prerequisites (Rust, Stellar CLI, Node.js)
2. ✅ Generated and funded a testnet keypair
3. ✅ Built both contracts for WASM release
4. ✅ Deployed invoice contract to testnet
5. ✅ Deployed pool contract to testnet
6. ✅ Initialized both contracts with correct parameters
7. ✅ Minted testnet USDC for testing
8. ✅ Configured frontend with deployed contract IDs
9. ✅ Ran the frontend and verified end-to-end flow

Your Astera platform is now running on Stellar testnet!

## Next Steps

- Explore the full invoice lifecycle (create → fund → repay)
- Test multi-investor co-funding scenarios
- Monitor contract events on [Stellar Expert](https://stellar.expert/explorer/testnet)
- Review contract code in `contracts/invoice/src/lib.rs` and `contracts/pool/src/lib.rs`
- Customize yield rates and pool parameters

## Useful Resources

- [Stellar Documentation](https://developers.stellar.org/)
- [Soroban Smart Contracts](https://soroban.stellar.org/)
- [Stellar CLI Reference](https://developers.stellar.org/docs/tools/developer-tools/stellar-cli)
- [Freighter Wallet](https://www.freighter.app/)
- [Stellar Expert Explorer](https://stellar.expert/explorer/testnet)
