#![cfg(test)]

use super::*;
use soroban_sdk::{testutils::{Address as _, Ledger as _}, Env, symbol_short};
use crate::storage::Signal;

#[contracttype]
enum MockDataKey {
    Signal(u64),
    Authorized(Address),
}

fn setup_env() -> Env {
    let env = Env::default();
    env.mock_all_auths();
    env.ledger().set_timestamp(1000);
    env
}

fn setup_signal(_env: &Env, signal_id: u64, expiry: u64) -> Signal {
    Signal {
        signal_id,
        price: 100,
        expiry,
        base_asset: 1,
    }
}

#[test]
fn test_execute_trade_invalid_amount() {
    let env = setup_env();
    let contract_id = env.register(AutoTradeContract, ());
    let user = Address::generate(&env);

    env.as_contract(&contract_id, || {
        let res = AutoTradeContract::execute_trade(
            env.clone(),
            user.clone(),
            1,
            OrderType::Market,
            0,
        );

        assert_eq!(res, Err(AutoTradeError::InvalidAmount));
    });
}

#[test]
fn test_execute_trade_signal_not_found() {
    let env = setup_env();
    let contract_id = env.register(AutoTradeContract, ());
    let user = Address::generate(&env);

    env.as_contract(&contract_id, || {
        let res = AutoTradeContract::execute_trade(
            env.clone(),
            user.clone(),
            1,
            OrderType::Market,
            100,
        );

        assert_eq!(res, Err(AutoTradeError::SignalNotFound));
    });
}

#[test]
fn test_execute_trade_signal_expired() {
    let env = setup_env();
    let contract_id = env.register(AutoTradeContract, ());
    let user = Address::generate(&env);
    let signal_id = 1;
    let signal = setup_signal(&env, signal_id, env.ledger().timestamp() - 1);

    env.as_contract(&contract_id, || {
        env.storage().persistent().set(&MockDataKey::Signal(signal_id), &signal);
        let res = AutoTradeContract::execute_trade(
            env.clone(),
            user.clone(),
            signal_id,
            OrderType::Market,
            100,
        );

        assert_eq!(res, Err(AutoTradeError::SignalExpired));
    });
}

#[test]
fn test_execute_trade_unauthorized() {
    let env = setup_env();
    let contract_id = env.register(AutoTradeContract, ());
    let user = Address::generate(&env);
    let signal_id = 1;
    let signal = setup_signal(&env, signal_id, env.ledger().timestamp() + 1000);

    env.as_contract(&contract_id, || {
        env.storage().persistent().set(&MockDataKey::Signal(signal_id), &signal);
        // Not setting authorized to true
        let res = AutoTradeContract::execute_trade(
            env.clone(),
            user.clone(),
            signal_id,
            OrderType::Market,
            100,
        );

        assert_eq!(res, Err(AutoTradeError::Unauthorized));
    });
}

#[test]
fn test_execute_trade_insufficient_balance() {
    let env = setup_env();
    let contract_id = env.register(AutoTradeContract, ());
    let user = Address::generate(&env);
    let signal_id = 1;
    let signal = setup_signal(&env, signal_id, env.ledger().timestamp() + 1000);

    env.as_contract(&contract_id, || {
        env.storage().persistent().set(&MockDataKey::Signal(signal_id), &signal);
        env.storage().persistent().set(&MockDataKey::Authorized(user.clone()), &true);
        let balance_key = (user.clone(), symbol_short!("balance"));
        env.storage().temporary().set(&balance_key, &50i128); // insufficient
        let res = AutoTradeContract::execute_trade(
            env.clone(),
            user.clone(),
            signal_id,
            OrderType::Market,
            100,
        );

        assert_eq!(res, Err(AutoTradeError::InsufficientBalance));
    });
}

#[test]
fn test_execute_trade_market_full_fill() {
    let env = setup_env();
    let contract_id = env.register(AutoTradeContract, ());
    let user = Address::generate(&env);
    let signal_id = 1;
    let signal = setup_signal(&env, signal_id, env.ledger().timestamp() + 1000);

    env.as_contract(&contract_id, || {
        env.storage().persistent().set(&MockDataKey::Signal(signal_id), &signal);
        env.storage().persistent().set(&MockDataKey::Authorized(user.clone()), &true);
        let balance_key = (user.clone(), symbol_short!("balance"));
        env.storage().temporary().set(&balance_key, &500i128);
        let liquidity_key = (symbol_short!("liquidity"), signal_id);
        env.storage().temporary().set(&liquidity_key, &500i128);

        let res = AutoTradeContract::execute_trade(
            env.clone(),
            user.clone(),
            signal_id,
            OrderType::Market,
            400,
        ).unwrap();

        assert_eq!(res.trade.executed_amount, 400);
        assert_eq!(res.trade.executed_price, 100);
        assert_eq!(res.trade.status, TradeStatus::Filled);
    });
}

#[test]
fn test_execute_trade_market_partial_fill() {
    let env = setup_env();
    let contract_id = env.register(AutoTradeContract, ());
    let user = Address::generate(&env);
    let signal_id = 2;
    let signal = setup_signal(&env, signal_id, env.ledger().timestamp() + 1000);

    env.as_contract(&contract_id, || {
        env.storage().persistent().set(&MockDataKey::Signal(signal_id), &signal);
        env.storage().persistent().set(&MockDataKey::Authorized(user.clone()), &true);
        let balance_key = (user.clone(), symbol_short!("balance"));
        env.storage().temporary().set(&balance_key, &500i128);
        let liquidity_key = (symbol_short!("liquidity"), signal_id);
        env.storage().temporary().set(&liquidity_key, &100i128);

        let res = AutoTradeContract::execute_trade(
            env.clone(),
            user.clone(),
            signal_id,
            OrderType::Market,
            300,
        ).unwrap();

        assert_eq!(res.trade.executed_amount, 100);
        assert_eq!(res.trade.executed_price, 100);
        assert_eq!(res.trade.status, TradeStatus::PartiallyFilled);
    });
}

#[test]
fn test_execute_trade_limit_filled() {
    let env = setup_env();
    let contract_id = env.register(AutoTradeContract, ());
    let user = Address::generate(&env);
    let signal_id = 3;
    let signal = setup_signal(&env, signal_id, env.ledger().timestamp() + 1000);

    env.as_contract(&contract_id, || {
        env.storage().persistent().set(&MockDataKey::Signal(signal_id), &signal);
        env.storage().persistent().set(&MockDataKey::Authorized(user.clone()), &true);
        let balance_key = (user.clone(), symbol_short!("balance"));
        env.storage().temporary().set(&balance_key, &500i128);
        let price_key = (symbol_short!("price"), signal_id);
        env.storage().temporary().set(&price_key, &90i128); // market_price < signal.price

        let res = AutoTradeContract::execute_trade(
            env.clone(),
            user.clone(),
            signal_id,
            OrderType::Limit,
            200,
        ).unwrap();

        assert_eq!(res.trade.executed_amount, 200);
        assert_eq!(res.trade.executed_price, 100);
        assert_eq!(res.trade.status, TradeStatus::Filled);
    });
}

#[test]
fn test_execute_trade_limit_not_filled() {
    let env = setup_env();
    let contract_id = env.register(AutoTradeContract, ());
    let user = Address::generate(&env);
    let signal_id = 4;
    let signal = setup_signal(&env, signal_id, env.ledger().timestamp() + 1000);

    env.as_contract(&contract_id, || {
        env.storage().persistent().set(&MockDataKey::Signal(signal_id), &signal);
        env.storage().persistent().set(&MockDataKey::Authorized(user.clone()), &true);
        let balance_key = (user.clone(), symbol_short!("balance"));
        env.storage().temporary().set(&balance_key, &500i128);
        let price_key = (symbol_short!("price"), signal_id);
        env.storage().temporary().set(&price_key, &150i128); // market_price > signal.price

        let res = AutoTradeContract::execute_trade(
            env.clone(),
            user.clone(),
            signal_id,
            OrderType::Limit,
            200,
        ).unwrap();

        assert_eq!(res.trade.executed_amount, 0);
        assert_eq!(res.trade.executed_price, 0);
        assert_eq!(res.trade.status, TradeStatus::Failed);
    });
}

#[test]
fn test_get_trade_existing() {
    let env = setup_env();
    let contract_id = env.register(AutoTradeContract, ());
    let user = Address::generate(&env);
    let signal_id = 1;
    let signal = setup_signal(&env, signal_id, env.ledger().timestamp() + 1000);

    env.as_contract(&contract_id, || {
        env.storage().persistent().set(&MockDataKey::Signal(signal_id), &signal);
        env.storage().persistent().set(&MockDataKey::Authorized(user.clone()), &true);
        let balance_key = (user.clone(), symbol_short!("balance"));
        env.storage().temporary().set(&balance_key, &500i128);
        let liquidity_key = (symbol_short!("liquidity"), signal_id);
        env.storage().temporary().set(&liquidity_key, &500i128);
    });

    env.as_contract(&contract_id, || {
        let _ = AutoTradeContract::execute_trade(
            env.clone(),
            user.clone(),
            signal_id,
            OrderType::Market,
            400,
        ).unwrap();
    });

    env.as_contract(&contract_id, || {
        let trade = AutoTradeContract::get_trade(
            env.clone(),
            user.clone(),
            signal_id,
        ).unwrap();

        assert_eq!(trade.executed_amount, 400);
    });
}

#[test]
fn test_get_trade_non_existing() {
    let env = setup_env();
    let contract_id = env.register(AutoTradeContract, ());
    let user = Address::generate(&env);
    let signal_id = 999;

    env.as_contract(&contract_id, || {
        let trade = AutoTradeContract::get_trade(
            env.clone(),
            user.clone(),
            signal_id,
        );

        assert!(trade.is_none());
    });
}
