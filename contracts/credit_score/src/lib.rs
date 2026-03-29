#![no_std]

use soroban_sdk::{
    contract, contractimpl, contracttype, symbol_short, Address, Env, String, Symbol, Vec,
};

const MIN_SCORE: u32 = 200;
const MAX_SCORE: u32 = 850;
const BASE_SCORE: u32 = 500;

const PTS_PAID_ON_TIME: u32 = 30;
const PTS_PAID_LATE: u32 = 15;
const PTS_DEFAULTED: i32 = -50;
const PTS_NEW_INVOICE: u32 = 5;

const LATE_PAYMENT_THRESHOLD_SECS: u64 = 7 * 24 * 60 * 60;

#[contracttype]
#[derive(Clone)]
pub struct PaymentRecord {
    pub invoice_id: u64,
    pub sme: Address,
    pub amount: i128,
    pub due_date: u64,
    pub paid_at: u64,
    pub status: PaymentStatus,
    pub days_late: i64,
}

#[contracttype]
#[derive(Clone, PartialEq, Debug)]
pub enum PaymentStatus {
    PaidOnTime,
    PaidLate,
    Defaulted,
}

#[contracttype]
#[derive(Clone)]
pub struct CreditScoreData {
    pub sme: Address,
    pub score: u32,
    pub total_invoices: u32,
    pub paid_on_time: u32,
    pub paid_late: u32,
    pub defaulted: u32,
    pub total_volume: i128,
    pub average_payment_days: i64,
    pub last_updated: u64,
    pub score_version: u32,
}

#[contracttype]
pub enum DataKey {
    CreditScore(Address),
    PaymentHistory(Address),
    PaymentRecordIdx(Address, u32),
    InvoiceProcessed(u64),
    Admin,
    InvoiceContract,
    PoolContract,
    Initialized,
    ScoreVersion,
}

const EVT: Symbol = symbol_short!("CREDIT");

#[contract]
pub struct CreditScoreContract;

fn calculate_score(
    total_invoices: u32,
    paid_on_time: u32,
    paid_late: u32,
    defaulted: u32,
    total_volume: i128,
    average_payment_days: i64,
) -> u32 {
    if total_invoices == 0 {
        return MIN_SCORE;
    }

    let mut score: i64 = BASE_SCORE as i64;

    score += (paid_on_time as i32 * PTS_PAID_ON_TIME as i32) as i64;
    score += (paid_late as i32 * PTS_PAID_LATE as i32) as i64;
    score += (defaulted as i32 * PTS_DEFAULTED as i32) as i64;

    if total_invoices >= 5 {
        score += PTS_NEW_INVOICE as i64;
    }
    if total_invoices >= 10 {
        score += PTS_NEW_INVOICE as i64;
    }
    if total_invoices >= 20 {
        score += PTS_NEW_INVOICE as i64;
    }

    if average_payment_days < 0 {
        score += 20;
    } else if average_payment_days < 3 {
        score += 15;
    } else if average_payment_days < 7 {
        score += 10;
    } else if average_payment_days > 30 {
        score -= 15;
    }

    if total_volume > 100_000_000_000 {
        score += 25;
    } else if total_volume > 10_000_000_000 {
        score += 15;
    } else if total_volume > 1_000_000_000 {
        score += 5;
    }

    if score < MIN_SCORE as i64 {
        MIN_SCORE
    } else if score > MAX_SCORE as i64 {
        MAX_SCORE
    } else {
        score as u32
    }
}

fn calculate_average_payment_days(paid_on_time: u32, paid_late: u32, total_late_days: i64) -> i64 {
    let total_paid = paid_on_time + paid_late;
    if total_paid == 0 {
        return 0;
    }
    total_late_days / total_paid as i64
}

#[contractimpl]
impl CreditScoreContract {
    pub fn initialize(env: Env, admin: Address, invoice_contract: Address, pool_contract: Address) {
        if env.storage().instance().has(&DataKey::Initialized) {
            panic!("already initialized");
        }
        env.storage().instance().set(&DataKey::Admin, &admin);
        env.storage()
            .instance()
            .set(&DataKey::InvoiceContract, &invoice_contract);
        env.storage()
            .instance()
            .set(&DataKey::PoolContract, &pool_contract);
        env.storage().instance().set(&DataKey::ScoreVersion, &1u32);
        env.storage().instance().set(&DataKey::Initialized, &true);
    }

    pub fn record_payment(
        env: Env,
        caller: Address,
        invoice_id: u64,
        sme: Address,
        amount: i128,
        due_date: u64,
        paid_at: u64,
    ) {
        let pool: Address = env
            .storage()
            .instance()
            .get(&DataKey::PoolContract)
            .expect("not initialized");

        if caller != pool {
            pool.require_auth();
        }

        if env
            .storage()
            .persistent()
            .has(&DataKey::InvoiceProcessed(invoice_id))
        {
            panic!("invoice already processed");
        }

        let status = if paid_at <= due_date {
            PaymentStatus::PaidOnTime
        } else if paid_at <= due_date + LATE_PAYMENT_THRESHOLD_SECS {
            PaymentStatus::PaidLate
        } else {
            PaymentStatus::Defaulted
        };

        let days_late: i64 = if paid_at > due_date {
            (paid_at - due_date) as i64 / (24 * 60 * 60)
        } else {
            -((due_date - paid_at) as i64 / (24 * 60 * 60))
        };

        let record = PaymentRecord {
            invoice_id,
            sme: sme.clone(),
            amount,
            due_date,
            paid_at,
            status: status.clone(),
            days_late,
        };

        let mut credit_data = Self::get_or_create_credit_data(&env, &sme);

        let history_len: u32 = env
            .storage()
            .instance()
            .get(&DataKey::PaymentHistory(sme.clone()))
            .unwrap_or(0);

        env.storage().persistent().set(
            &DataKey::PaymentRecordIdx(sme.clone(), history_len),
            &record,
        );
        env.storage()
            .instance()
            .set(&DataKey::PaymentHistory(sme.clone()), &(history_len + 1));

        match status {
            PaymentStatus::PaidOnTime => {
                credit_data.paid_on_time += 1;
            }
            PaymentStatus::PaidLate => {
                credit_data.paid_late += 1;
            }
            PaymentStatus::Defaulted => {
                credit_data.defaulted += 1;
            }
        }

        credit_data.total_invoices += 1;
        credit_data.total_volume += amount;
        credit_data.average_payment_days = calculate_average_payment_days(
            credit_data.paid_on_time,
            credit_data.paid_late,
            credit_data.average_payment_days * (credit_data.total_invoices - 1) as i64 + days_late,
        );
        credit_data.score = calculate_score(
            credit_data.total_invoices,
            credit_data.paid_on_time,
            credit_data.paid_late,
            credit_data.defaulted,
            credit_data.total_volume,
            credit_data.average_payment_days,
        );
        credit_data.last_updated = env.ledger().timestamp();

        env.storage()
            .persistent()
            .set(&DataKey::CreditScore(sme.clone()), &credit_data);
        env.storage()
            .persistent()
            .set(&DataKey::InvoiceProcessed(invoice_id), &true);

        env.events().publish(
            (EVT, symbol_short!("payment")),
            (sme, invoice_id, status, credit_data.score),
        );
    }

    pub fn record_default(
        env: Env,
        caller: Address,
        invoice_id: u64,
        sme: Address,
        amount: i128,
        due_date: u64,
    ) {
        let pool: Address = env
            .storage()
            .instance()
            .get(&DataKey::PoolContract)
            .expect("not initialized");

        if caller != pool {
            pool.require_auth();
        }

        if env
            .storage()
            .persistent()
            .has(&DataKey::InvoiceProcessed(invoice_id))
        {
            panic!("invoice already processed");
        }

        let defaulted_at = env.ledger().timestamp();
        let days_late = if defaulted_at > due_date {
            (defaulted_at - due_date) as i64 / (24 * 60 * 60)
        } else {
            0
        };

        let record = PaymentRecord {
            invoice_id,
            sme: sme.clone(),
            amount,
            due_date,
            paid_at: defaulted_at,
            status: PaymentStatus::Defaulted,
            days_late,
        };

        let mut credit_data = Self::get_or_create_credit_data(&env, &sme);

        let history_len: u32 = env
            .storage()
            .instance()
            .get(&DataKey::PaymentHistory(sme.clone()))
            .unwrap_or(0);

        env.storage().persistent().set(
            &DataKey::PaymentRecordIdx(sme.clone(), history_len),
            &record,
        );
        env.storage()
            .instance()
            .set(&DataKey::PaymentHistory(sme.clone()), &(history_len + 1));

        credit_data.defaulted += 1;
        credit_data.total_invoices += 1;
        credit_data.total_volume += amount;
        credit_data.average_payment_days = calculate_average_payment_days(
            credit_data.paid_on_time,
            credit_data.paid_late,
            credit_data.average_payment_days * (credit_data.total_invoices - 1) as i64 + days_late,
        );
        credit_data.score = calculate_score(
            credit_data.total_invoices,
            credit_data.paid_on_time,
            credit_data.paid_late,
            credit_data.defaulted,
            credit_data.total_volume,
            credit_data.average_payment_days,
        );
        credit_data.last_updated = env.ledger().timestamp();

        env.storage()
            .persistent()
            .set(&DataKey::CreditScore(sme.clone()), &credit_data);
        env.storage()
            .persistent()
            .set(&DataKey::InvoiceProcessed(invoice_id), &true);

        env.events().publish(
            (EVT, symbol_short!("default")),
            (sme, invoice_id, credit_data.score),
        );
    }

    pub fn get_credit_score(env: Env, sme: Address) -> CreditScoreData {
        Self::get_or_create_credit_data(&env, &sme)
    }

    pub fn get_payment_history(env: Env, sme: Address) -> Vec<PaymentRecord> {
        let history_len: u32 = env
            .storage()
            .instance()
            .get(&DataKey::PaymentHistory(sme.clone()))
            .unwrap_or(0);

        let mut records = Vec::new(&env);
        for i in 0..history_len {
            if let Some(record) = env
                .storage()
                .persistent()
                .get(&DataKey::PaymentRecordIdx(sme.clone(), i))
            {
                records.push_back(record);
            }
        }
        records
    }

    pub fn get_payment_record(env: Env, sme: Address, index: u32) -> Option<PaymentRecord> {
        env.storage()
            .persistent()
            .get(&DataKey::PaymentRecordIdx(sme, index))
    }

    pub fn get_payment_history_length(env: Env, sme: Address) -> u32 {
        env.storage()
            .instance()
            .get(&DataKey::PaymentHistory(sme))
            .unwrap_or(0)
    }

    pub fn get_score_band(env: Env, score: u32) -> String {
        if score >= 800 {
            String::from_str(&env, "Excellent")
        } else if score >= 740 {
            String::from_str(&env, "Very Good")
        } else if score >= 670 {
            String::from_str(&env, "Good")
        } else if score >= 580 {
            String::from_str(&env, "Fair")
        } else if score >= 500 {
            String::from_str(&env, "Poor")
        } else {
            String::from_str(&env, "Very Poor")
        }
    }

    pub fn is_invoice_processed(env: Env, invoice_id: u64) -> bool {
        env.storage()
            .persistent()
            .has(&DataKey::InvoiceProcessed(invoice_id))
    }

    pub fn get_config(env: Env) -> (Address, Address, Address) {
        let admin: Address = env
            .storage()
            .instance()
            .get(&DataKey::Admin)
            .expect("not initialized");
        let invoice_contract: Address = env
            .storage()
            .instance()
            .get(&DataKey::InvoiceContract)
            .expect("not initialized");
        let pool_contract: Address = env
            .storage()
            .instance()
            .get(&DataKey::PoolContract)
            .expect("not initialized");
        (admin, invoice_contract, pool_contract)
    }

    pub fn set_invoice_contract(env: Env, admin: Address, invoice_contract: Address) {
        admin.require_auth();
        Self::require_admin(&env, &admin);
        env.storage()
            .instance()
            .set(&DataKey::InvoiceContract, &invoice_contract);
    }

    pub fn set_pool_contract(env: Env, admin: Address, pool_contract: Address) {
        admin.require_auth();
        Self::require_admin(&env, &admin);
        env.storage()
            .instance()
            .set(&DataKey::PoolContract, &pool_contract);
    }

    fn get_or_create_credit_data(env: &Env, sme: &Address) -> CreditScoreData {
        if let Some(data) = env
            .storage()
            .persistent()
            .get(&DataKey::CreditScore(sme.clone()))
        {
            data
        } else {
            CreditScoreData {
                sme: sme.clone(),
                score: MIN_SCORE,
                total_invoices: 0,
                paid_on_time: 0,
                paid_late: 0,
                defaulted: 0,
                total_volume: 0,
                average_payment_days: 0,
                last_updated: env.ledger().timestamp(),
                score_version: 1,
            }
        }
    }

    fn require_admin(env: &Env, admin: &Address) {
        let stored_admin: Address = env
            .storage()
            .instance()
            .get(&DataKey::Admin)
            .expect("not initialized");
        if admin != &stored_admin {
            panic!("unauthorized");
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use soroban_sdk::{testutils::Address as _, testutils::Ledger, Env};

    fn setup(env: &Env) -> (CreditScoreContractClient<'_>, Address, Address, Address) {
        let contract_id = env.register(CreditScoreContract, ());
        let client = CreditScoreContractClient::new(env, &contract_id);
        let admin = Address::generate(env);
        let invoice_contract = Address::generate(env);
        let pool_contract = Address::generate(env);
        client.initialize(&admin, &invoice_contract, &pool_contract);
        (client, admin, invoice_contract, pool_contract)
    }

    #[test]
    fn test_initial_score() {
        let env = Env::default();
        env.mock_all_auths();

        let (client, _admin, _invoice, _pool) = setup(&env);
        let sme = Address::generate(&env);

        let score_data = client.get_credit_score(&sme);
        assert_eq!(score_data.score, MIN_SCORE);
        assert_eq!(score_data.total_invoices, 0);
    }

    #[test]
    fn test_record_payment_on_time() {
        let env = Env::default();
        env.mock_all_auths();
        env.ledger().with_mut(|l| l.timestamp = 100_000);

        let (client, _admin, _invoice, pool) = setup(&env);
        let sme = Address::generate(&env);

        let due_date = 200_000u64;
        let paid_at = 150_000u64;

        client.record_payment(&pool, &1, &sme, &1_000_000_000i128, &due_date, &paid_at);

        let score_data = client.get_credit_score(&sme);
        assert_eq!(score_data.total_invoices, 1);
        assert_eq!(score_data.paid_on_time, 1);
        assert_eq!(score_data.paid_late, 0);
        assert_eq!(score_data.defaulted, 0);
        assert!(score_data.score > MIN_SCORE);
    }

    #[test]
    fn test_record_payment_late() {
        let env = Env::default();
        env.mock_all_auths();
        env.ledger().with_mut(|l| l.timestamp = 100_000);

        let (client, _admin, _invoice, pool) = setup(&env);
        let sme = Address::generate(&env);

        let due_date = 100_000u64;
        let paid_at = 150_000u64;

        client.record_payment(&pool, &1, &sme, &1_000_000_000i128, &due_date, &paid_at);

        let score_data = client.get_credit_score(&sme);
        assert_eq!(score_data.total_invoices, 1);
        assert_eq!(score_data.paid_on_time, 0);
        assert_eq!(score_data.paid_late, 1);
        assert!(score_data.score > MIN_SCORE);
    }

    #[test]
    fn test_record_default() {
        let env = Env::default();
        env.mock_all_auths();
        env.ledger().with_mut(|l| l.timestamp = 200_000);

        let (client, _admin, _invoice, pool) = setup(&env);
        let sme = Address::generate(&env);

        let due_date = 100_000u64;

        client.record_default(&pool, &1, &sme, &1_000_000_000i128, &due_date);

        let score_data = client.get_credit_score(&sme);
        assert_eq!(score_data.total_invoices, 1);
        assert_eq!(score_data.defaulted, 1);
        assert!(score_data.score < BASE_SCORE);
    }

    #[test]
    fn test_multiple_payments_improve_score() {
        let env = Env::default();
        env.mock_all_auths();
        env.ledger().with_mut(|l| l.timestamp = 100_000);

        let (client, _admin, _invoice, pool) = setup(&env);
        let sme = Address::generate(&env);

        let due_date = 200_000u64;

        for i in 1..=10 {
            client.record_payment(
                &pool,
                &i,
                &sme,
                &1_000_000_000i128,
                &due_date,
                &(due_date - 1000),
            );
        }

        let score_data = client.get_credit_score(&sme);
        assert_eq!(score_data.total_invoices, 10);
        assert_eq!(score_data.paid_on_time, 10);
        assert!(score_data.score > BASE_SCORE);
    }

    #[test]
    fn test_defaults_decrease_score() {
        let env = Env::default();
        env.mock_all_auths();
        env.ledger().with_mut(|l| l.timestamp = 300_000);

        let (client, _admin, _invoice, pool) = setup(&env);
        let sme = Address::generate(&env);

        let due_date = 100_000u64;

        client.record_payment(
            &pool,
            &1,
            &sme,
            &1_000_000_000i128,
            &due_date,
            &(due_date - 1000),
        );
        client.record_default(&pool, &2, &sme, &1_000_000_000i128, &due_date);
        client.record_default(&pool, &3, &sme, &1_000_000_000i128, &due_date);

        let score_data = client.get_credit_score(&sme);
        assert_eq!(score_data.total_invoices, 3);
        assert_eq!(score_data.paid_on_time, 1);
        assert_eq!(score_data.defaulted, 2);
        assert!(score_data.score < BASE_SCORE);
    }

    #[test]
    fn test_payment_history() {
        let env = Env::default();
        env.mock_all_auths();
        env.ledger().with_mut(|l| l.timestamp = 100_000);

        let (client, _admin, _invoice, pool) = setup(&env);
        let sme = Address::generate(&env);

        let due_date = 200_000u64;

        client.record_payment(
            &pool,
            &1,
            &sme,
            &1_000_000_000i128,
            &due_date,
            &(due_date - 1000),
        );
        client.record_payment(&pool, &2, &sme, &2_000_000_000i128, &due_date, &due_date);
        client.record_default(&pool, &3, &sme, &500_000_000i128, &due_date);

        let history = client.get_payment_history(&sme);
        assert_eq!(history.len(), 3);

        let record1 = client.get_payment_record(&sme, &0).unwrap();
        assert_eq!(record1.invoice_id, 1);
        assert!(matches!(record1.status, PaymentStatus::PaidOnTime));

        let record2 = client.get_payment_record(&sme, &1).unwrap();
        assert_eq!(record2.invoice_id, 2);
        assert!(matches!(record2.status, PaymentStatus::PaidOnTime));

        let record3 = client.get_payment_record(&sme, &2).unwrap();
        assert_eq!(record3.invoice_id, 3);
        assert!(matches!(record3.status, PaymentStatus::Defaulted));
    }

    #[test]
    #[should_panic(expected = "invoice already processed")]
    fn test_cannot_process_same_invoice_twice() {
        let env = Env::default();
        env.mock_all_auths();
        env.ledger().with_mut(|l| l.timestamp = 100_000);

        let (client, _admin, _invoice, pool) = setup(&env);
        let sme = Address::generate(&env);

        let due_date = 200_000u64;

        client.record_payment(
            &pool,
            &1,
            &sme,
            &1_000_000_000i128,
            &due_date,
            &(due_date - 1000),
        );

        client.record_payment(
            &pool,
            &1,
            &sme,
            &1_000_000_000i128,
            &due_date,
            &(due_date - 1000),
        );
    }

    #[test]
    fn test_score_bands() {
        let env = Env::default();
        env.mock_all_auths();

        let (client, _admin, _invoice, _pool) = setup(&env);

        assert_eq!(
            client.get_score_band(&850),
            String::from_str(&env, "Excellent")
        );
        assert_eq!(
            client.get_score_band(&800),
            String::from_str(&env, "Excellent")
        );
        assert_eq!(
            client.get_score_band(&750),
            String::from_str(&env, "Very Good")
        );
        assert_eq!(client.get_score_band(&700), String::from_str(&env, "Good"));
        assert_eq!(client.get_score_band(&650), String::from_str(&env, "Fair"));
        assert_eq!(client.get_score_band(&600), String::from_str(&env, "Fair"));
        assert_eq!(client.get_score_band(&550), String::from_str(&env, "Poor"));
        assert_eq!(
            client.get_score_band(&400),
            String::from_str(&env, "Very Poor")
        );
    }

    #[test]
    fn test_invoice_processed_check() {
        let env = Env::default();
        env.mock_all_auths();
        env.ledger().with_mut(|l| l.timestamp = 100_000);

        let (client, _admin, _invoice, pool) = setup(&env);
        let sme = Address::generate(&env);

        assert!(!client.is_invoice_processed(&1));

        let due_date = 200_000u64;
        client.record_payment(
            &pool,
            &1,
            &sme,
            &1_000_000_000i128,
            &due_date,
            &(due_date - 1000),
        );

        assert!(client.is_invoice_processed(&1));
    }

    #[test]
    fn test_total_volume_tracking() {
        let env = Env::default();
        env.mock_all_auths();
        env.ledger().with_mut(|l| l.timestamp = 100_000);

        let (client, _admin, _invoice, pool) = setup(&env);
        let sme = Address::generate(&env);

        let due_date = 200_000u64;

        client.record_payment(
            &pool,
            &1,
            &sme,
            &1_000_000_000i128,
            &due_date,
            &(due_date - 1000),
        );
        client.record_payment(
            &pool,
            &2,
            &sme,
            &2_000_000_000i128,
            &due_date,
            &(due_date - 1000),
        );
        client.record_payment(
            &pool,
            &3,
            &sme,
            &3_000_000_000i128,
            &due_date,
            &(due_date - 1000),
        );

        let score_data = client.get_credit_score(&sme);
        assert_eq!(score_data.total_volume, 6_000_000_000i128);
    }
}
