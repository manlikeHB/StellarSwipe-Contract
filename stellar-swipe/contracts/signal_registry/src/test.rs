#![cfg(test)]

extern crate std;

use super::*;
use soroban_sdk::{testutils::Address as _, Env};

#[test]
fn create_and_read_signal() {
    let env = Env::default();
    env.mock_all_auths(); 

    #[allow(deprecated)]
    let contract_id = env.register_contract(None, SignalRegistry);
    let client = SignalRegistryClient::new(&env, &contract_id);

    let provider = Address::generate(&env);
    let expiry = env.ledger().timestamp() + 60;

    let signal_id = client.create_signal(
        &provider,
        &String::from_str(&env, "XLM/USDC"),
        &SignalAction::Buy,
        &100_000,
        &String::from_str(&env, "Breakout confirmed"),
        &expiry,
    );

    let signal = client.get_signal(&signal_id).unwrap();
    assert_eq!(signal.id, signal_id);
    assert_eq!(signal.status, SignalStatus::Active);
}

#[test]
fn provider_stats_initialized() {
    let env = Env::default();
    env.mock_all_auths(); 

    #[allow(deprecated)]
    let contract_id = env.register_contract(None, SignalRegistry);
    let client = SignalRegistryClient::new(&env, &contract_id);

    let provider = Address::generate(&env);
    let expiry = env.ledger().timestamp() + 120;

    client.create_signal(
        &provider,
        &String::from_str(&env, "XLM/BTC"),
        &SignalAction::Sell,
        &200_000,
        &String::from_str(&env, "Resistance hit"),
        &expiry,
    );

    let stats = client.get_provider_stats(&provider).unwrap();
    assert_eq!(stats.total_copies, 0);
}
