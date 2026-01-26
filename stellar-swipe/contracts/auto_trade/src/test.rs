#![cfg(test)]

use soroban_sdk::{testutils::Address as _, Address, Env};

use crate::sdex::*;
use crate::storage::Signal;

fn setup_signal(env: &Env, id: u64) -> Signal {
    let signal = Signal {
        signal_id: id,
        price: 100,
        expiry: env.ledger().timestamp() + 1000,
        base_asset: 1,
    };

    env.storage().persistent().set(&("signal", id), &signal);
    signal
}

#[test]
fn market_order_full_fill() {
    let env = Env::default();
    let user = Address::generate(&env);

    env.storage()
        .temporary()
        .set(&(user.clone(), "balance"), &1_000);

    env.storage()
        .temporary()
        .set(&("liquidity", 1u64), &500);

    let signal = setup_signal(&env, 1);

    let res = execute_market_order(&env, &user, &signal, 400).unwrap();

    assert_eq!(res.executed_amount, 400);
}

#[test]
fn market_order_partial_fill() {
    let env = Env::default();
    let user = Address::generate(&env);

    env.storage()
        .temporary()
        .set(&(user.clone(), "balance"), &1_000);

    env.storage()
        .temporary()
        .set(&("liquidity", 2u64), &100);

    let signal = setup_signal(&env, 2);

    let res = execute_market_order(&env, &user, &signal, 300).unwrap();

    assert_eq!(res.executed_amount, 100);
}

#[test]
fn limit_order_not_filled() {
    let env = Env::default();
    let user = Address::generate(&env);

    env.storage()
        .temporary()
        .set(&("market_price", 3u64), &150);

    let signal = setup_signal(&env, 3);

    let res = execute_limit_order(&env, &user, &signal, 200).unwrap();

    assert_eq!(res.executed_amount, 0);
}

#[test]
fn expired_signal_rejected() {
    let env = Env::default();
    let user = Address::generate(&env);

    let signal = Signal {
        signal_id: 4,
        price: 100,
        expiry: env.ledger().timestamp() - 1,
        base_asset: 1,
    };

    let err = execute_market_order(&env, &user, &signal, 100).unwrap_err();
    assert_eq!(err, crate::errors::AutoTradeError::SignalExpired);
}
