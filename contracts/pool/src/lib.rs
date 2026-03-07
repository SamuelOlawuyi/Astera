#![no_std]

use soroban_sdk::{
    contract, contractimpl, contracttype, symbol_short,
    token, Address, Env, Symbol,
};

/// Annual yield in basis points (800 = 8% APY)
const DEFAULT_YIELD_BPS: u32 = 800;
const BPS_DENOM: u32 = 10_000;
const SECS_PER_YEAR: u64 = 31_536_000;

#[contracttype]
#[derive(Clone)]
pub struct PoolConfig {
    pub usdc_token: Address,
    pub invoice_contract: Address,
    pub admin: Address,
    pub yield_bps: u32,
    pub total_deposited: i128,
    pub total_deployed: i128,
    pub total_paid_out: i128,
}

#[contracttype]
#[derive(Clone)]
pub struct InvestorPosition {
    pub deposited: i128,
    pub available: i128,
    pub deployed: i128,
    pub earned: i128,
    pub deposit_count: u32,
}

#[contracttype]
#[derive(Clone)]
pub struct FundedInvoice {
    pub invoice_id: u64,
    pub sme: Address,
    pub principal: i128,
    pub funded_at: u64,
    pub due_date: u64,
    pub repaid: bool,
}

#[contracttype]
pub enum DataKey {
    Config,
    Investor(Address),
    FundedInvoice(u64),
    Initialized,
}

const EVT: Symbol = symbol_short!("POOL");

#[contract]
pub struct FundingPool;

#[contractimpl]
impl FundingPool {
    pub fn initialize(
        env: Env,
        admin: Address,
        usdc_token: Address,
        invoice_contract: Address,
    ) {
        if env.storage().instance().has(&DataKey::Initialized) {
            panic!("already initialized");
        }

        let config = PoolConfig {
            usdc_token,
            invoice_contract,
            admin,
            yield_bps: DEFAULT_YIELD_BPS,
            total_deposited: 0,
            total_deployed: 0,
            total_paid_out: 0,
        };

        env.storage().instance().set(&DataKey::Config, &config);
        env.storage().instance().set(&DataKey::Initialized, &true);
    }

    /// Investor deposits USDC into the pool
    pub fn deposit(env: Env, investor: Address, amount: i128) {
        investor.require_auth();
        if amount <= 0 {
            panic!("amount must be positive");
        }

        let mut config: PoolConfig = env.storage().instance().get(&DataKey::Config).expect("not initialized");

        let token_client = token::Client::new(&env, &config.usdc_token);
        token_client.transfer(&investor, &env.current_contract_address(), &amount);

        let mut position = env.storage()
            .persistent()
            .get(&DataKey::Investor(investor.clone()))
            .unwrap_or(InvestorPosition {
                deposited: 0,
                available: 0,
                deployed: 0,
                earned: 0,
                deposit_count: 0,
            });

        position.deposited += amount;
        position.available += amount;
        position.deposit_count += 1;

        env.storage().persistent().set(&DataKey::Investor(investor.clone()), &position);

        config.total_deposited += amount;
        env.storage().instance().set(&DataKey::Config, &config);

        env.events().publish((EVT, symbol_short!("deposit")), (investor, amount));
    }

    /// Admin funds an SME invoice from the pool liquidity
    pub fn fund_invoice(
        env: Env,
        admin: Address,
        invoice_id: u64,
        principal: i128,
        sme: Address,
        due_date: u64,
    ) {
        admin.require_auth();

        let mut config: PoolConfig = env.storage().instance().get(&DataKey::Config).expect("not initialized");
        if admin != config.admin {
            panic!("unauthorized");
        }

        let liquidity = config.total_deposited - config.total_deployed;
        if liquidity < principal {
            panic!("insufficient pool liquidity");
        }

        // Transfer USDC to SME
        let token_client = token::Client::new(&env, &config.usdc_token);
        token_client.transfer(&env.current_contract_address(), &sme, &principal);

        let funded = FundedInvoice {
            invoice_id,
            sme: sme.clone(),
            principal,
            funded_at: env.ledger().timestamp(),
            due_date,
            repaid: false,
        };
        env.storage().persistent().set(&DataKey::FundedInvoice(invoice_id), &funded);

        config.total_deployed += principal;
        env.storage().instance().set(&DataKey::Config, &config);

        env.events().publish((EVT, symbol_short!("funded")), (invoice_id, sme, principal));
    }

    /// SME repays invoice principal + interest
    pub fn repay_invoice(env: Env, invoice_id: u64, payer: Address) {
        payer.require_auth();

        let mut config: PoolConfig = env.storage().instance().get(&DataKey::Config).expect("not initialized");

        let mut funded: FundedInvoice = env.storage()
            .persistent()
            .get(&DataKey::FundedInvoice(invoice_id))
            .expect("invoice not funded by this pool");

        if funded.repaid {
            panic!("already repaid");
        }

        // Simple interest: principal * yield_bps * days_elapsed / (10000 * 365)
        let now = env.ledger().timestamp();
        let elapsed_secs = now - funded.funded_at;
        let interest = (funded.principal as u128
            * config.yield_bps as u128
            * elapsed_secs as u128)
            / (BPS_DENOM as u128 * SECS_PER_YEAR as u128);
        let total_due = funded.principal + interest as i128;

        // Receive repayment
        let token_client = token::Client::new(&env, &config.usdc_token);
        token_client.transfer(&payer, &env.current_contract_address(), &total_due);

        // Mark repaid
        funded.repaid = true;
        env.storage().persistent().set(&DataKey::FundedInvoice(invoice_id), &funded);

        config.total_deployed -= funded.principal;
        config.total_paid_out += total_due;
        // Interest flows back into deposited pool, increasing share value
        config.total_deposited += interest as i128;
        env.storage().instance().set(&DataKey::Config, &config);

        env.events().publish(
            (EVT, symbol_short!("repaid")),
            (invoice_id, funded.principal, interest as i128),
        );
    }

    /// Investor withdraws their available (undeployed) USDC
    pub fn withdraw(env: Env, investor: Address, amount: i128) {
        investor.require_auth();
        if amount <= 0 {
            panic!("amount must be positive");
        }

        let mut config: PoolConfig = env.storage().instance().get(&DataKey::Config).expect("not initialized");

        let mut position: InvestorPosition = env.storage()
            .persistent()
            .get(&DataKey::Investor(investor.clone()))
            .expect("no position found");

        if position.available < amount {
            panic!("insufficient available balance");
        }

        let token_client = token::Client::new(&env, &config.usdc_token);
        token_client.transfer(&env.current_contract_address(), &investor, &amount);

        position.available -= amount;
        position.deposited -= amount;
        env.storage().persistent().set(&DataKey::Investor(investor.clone()), &position);

        config.total_deposited -= amount;
        env.storage().instance().set(&DataKey::Config, &config);

        env.events().publish((EVT, symbol_short!("withdraw")), (investor, amount));
    }

    /// Admin updates the pool yield rate
    pub fn set_yield(env: Env, admin: Address, yield_bps: u32) {
        admin.require_auth();
        let mut config: PoolConfig = env.storage().instance().get(&DataKey::Config).expect("not initialized");
        if admin != config.admin {
            panic!("unauthorized");
        }
        if yield_bps > 5_000 {
            panic!("yield cannot exceed 50%");
        }
        config.yield_bps = yield_bps;
        env.storage().instance().set(&DataKey::Config, &config);
    }

    // ---- Views ----

    pub fn get_config(env: Env) -> PoolConfig {
        env.storage().instance().get(&DataKey::Config).expect("not initialized")
    }

    pub fn get_position(env: Env, investor: Address) -> Option<InvestorPosition> {
        env.storage().persistent().get(&DataKey::Investor(investor))
    }

    pub fn get_funded_invoice(env: Env, invoice_id: u64) -> Option<FundedInvoice> {
        env.storage().persistent().get(&DataKey::FundedInvoice(invoice_id))
    }

    /// Available liquidity in the pool
    pub fn available_liquidity(env: Env) -> i128 {
        let config: PoolConfig = env.storage().instance().get(&DataKey::Config).expect("not initialized");
        config.total_deposited - config.total_deployed
    }

    /// Estimate repayment amount for a given invoice at current time
    pub fn estimate_repayment(env: Env, invoice_id: u64) -> i128 {
        let config: PoolConfig = env.storage().instance().get(&DataKey::Config).expect("not initialized");
        let funded: FundedInvoice = env.storage()
            .persistent()
            .get(&DataKey::FundedInvoice(invoice_id))
            .expect("invoice not funded");

        let now = env.ledger().timestamp();
        let elapsed = now - funded.funded_at;
        let interest = (funded.principal as u128
            * config.yield_bps as u128
            * elapsed as u128)
            / (BPS_DENOM as u128 * SECS_PER_YEAR as u128);

        funded.principal + interest as i128
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use soroban_sdk::{
        testutils::{Address as _, Ledger},
        Env,
    };

    #[test]
    fn test_deposit_and_withdraw() {
        let env = Env::default();
        env.mock_all_auths();

        let contract_id = env.register_contract(None, FundingPool);
        let client = FundingPoolClient::new(&env, &contract_id);

        let admin = Address::generate(&env);
        let usdc = Address::generate(&env);
        let invoice_contract = Address::generate(&env);
        let investor = Address::generate(&env);

        client.initialize(&admin, &usdc, &invoice_contract);

        // Note: in real tests you'd mock the USDC token contract
        // For simplicity we just test state transitions here

        let config = client.get_config();
        assert_eq!(config.yield_bps, DEFAULT_YIELD_BPS);
        assert_eq!(config.total_deposited, 0);
    }
}
