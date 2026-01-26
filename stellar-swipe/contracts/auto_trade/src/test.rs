#![cfg(test)]

use super::*;
use crate::risk;
use crate::storage;
use soroban_sdk::{
    symbol_short,
    testutils::{Address as _, Ledger as _},
    Env,
};

fn setup_env() -> Env {
    let env = Env::default();
    env.mock_all_auths();
    env.ledger().set_timestamp(1000);
    env
}

fn setup_signal(_env: &Env, signal_id: u64, expiry: u64) -> storage::Signal {
    storage::Signal {
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
        let res =
            AutoTradeContract::execute_trade(env.clone(), user.clone(), 1, OrderType::Market, 0);

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
            999,
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
        storage::set_signal(&env, signal_id, &signal);
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
        storage::set_signal(&env, signal_id, &signal);
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
        storage::set_signal(&env, signal_id, &signal);
        storage::authorize_user(&env, &user);
        env.storage()
            .temporary()
            .set(&(user.clone(), symbol_short!("balance")), &50i128);

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
        storage::set_signal(&env, signal_id, &signal);
        storage::authorize_user(&env, &user);
        env.storage()
            .temporary()
            .set(&(user.clone(), symbol_short!("balance")), &500i128);
        env.storage()
            .temporary()
            .set(&(symbol_short!("liquidity"), signal_id), &500i128);

        let res = AutoTradeContract::execute_trade(
            env.clone(),
            user.clone(),
            signal_id,
            OrderType::Market,
            400,
        )
        .unwrap();

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
        storage::set_signal(&env, signal_id, &signal);
        storage::authorize_user(&env, &user);
        env.storage()
            .temporary()
            .set(&(user.clone(), symbol_short!("balance")), &500i128);
        env.storage()
            .temporary()
            .set(&(symbol_short!("liquidity"), signal_id), &100i128);

        let res = AutoTradeContract::execute_trade(
            env.clone(),
            user.clone(),
            signal_id,
            OrderType::Market,
            300,
        )
        .unwrap();

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
        storage::set_signal(&env, signal_id, &signal);
        storage::authorize_user(&env, &user);
        env.storage()
            .temporary()
            .set(&(user.clone(), symbol_short!("balance")), &500i128);
        env.storage()
            .temporary()
            .set(&(symbol_short!("price"), signal_id), &90i128);

        let res = AutoTradeContract::execute_trade(
            env.clone(),
            user.clone(),
            signal_id,
            OrderType::Limit,
            200,
        )
        .unwrap();

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
        storage::set_signal(&env, signal_id, &signal);
        storage::authorize_user(&env, &user);
        env.storage()
            .temporary()
            .set(&(user.clone(), symbol_short!("balance")), &500i128);
        env.storage()
            .temporary()
            .set(&(symbol_short!("price"), signal_id), &150i128);

        let res = AutoTradeContract::execute_trade(
            env.clone(),
            user.clone(),
            signal_id,
            OrderType::Limit,
            200,
        )
        .unwrap();

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
        storage::set_signal(&env, signal_id, &signal);
        storage::authorize_user(&env, &user);
        env.storage()
            .temporary()
            .set(&(user.clone(), symbol_short!("balance")), &500i128);
        env.storage()
            .temporary()
            .set(&(symbol_short!("liquidity"), signal_id), &500i128);
    });

    env.as_contract(&contract_id, || {
        let _ = AutoTradeContract::execute_trade(
            env.clone(),
            user.clone(),
            signal_id,
            OrderType::Market,
            400,
        )
        .unwrap();
    });

    env.as_contract(&contract_id, || {
        let trade = AutoTradeContract::get_trade(env.clone(), user.clone(), signal_id).unwrap();

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
        let trade = AutoTradeContract::get_trade(env.clone(), user.clone(), signal_id);

        assert!(trade.is_none());
    });
}

// ========================================
// Risk Management Tests
// ========================================

#[test]
fn test_get_default_risk_config() {
    let env = setup_env();
    let contract_id = env.register(AutoTradeContract, ());
    let user = Address::generate(&env);

    env.as_contract(&contract_id, || {
        let config = AutoTradeContract::get_risk_config(env.clone(), user.clone());

        assert_eq!(config.max_position_pct, 20);
        assert_eq!(config.daily_trade_limit, 10);
        assert_eq!(config.stop_loss_pct, 15);
    });
}

#[test]
fn test_set_custom_risk_config() {
    let env = setup_env();
    let contract_id = env.register(AutoTradeContract, ());
    let user = Address::generate(&env);

    env.as_contract(&contract_id, || {
        let custom_config = risk::RiskConfig {
            max_position_pct: 30,
            daily_trade_limit: 15,
            stop_loss_pct: 10,
        };

        AutoTradeContract::set_risk_config(env.clone(), user.clone(), custom_config.clone());

        let retrieved = AutoTradeContract::get_risk_config(env.clone(), user.clone());
        assert_eq!(retrieved, custom_config);
    });
}

#[test]
fn test_position_limit_allows_first_trade() {
    let env = setup_env();
    let contract_id = env.register(AutoTradeContract, ());
    let user = Address::generate(&env);
    let signal_id = 1;
    let signal = setup_signal(&env, signal_id, env.ledger().timestamp() + 1000);

    env.as_contract(&contract_id, || {
        storage::set_signal(&env, signal_id, &signal);
        storage::authorize_user(&env, &user);
        env.storage()
            .temporary()
            .set(&(user.clone(), symbol_short!("balance")), &1000i128);
        env.storage()
            .temporary()
            .set(&(symbol_short!("liquidity"), signal_id), &1000i128);

        // First trade should be allowed
        let res = AutoTradeContract::execute_trade(
            env.clone(),
            user.clone(),
            signal_id,
            OrderType::Market,
            1000,
        );

        assert!(res.is_ok());
    });
}

#[test]
fn test_get_user_positions() {
    let env = setup_env();
    let contract_id = env.register(AutoTradeContract, ());
    let user = Address::generate(&env);
    let signal_id = 1;
    let signal = setup_signal(&env, signal_id, env.ledger().timestamp() + 1000);

    env.as_contract(&contract_id, || {
        storage::set_signal(&env, signal_id, &signal);
        storage::authorize_user(&env, &user);
        env.storage()
            .temporary()
            .set(&(user.clone(), symbol_short!("balance")), &1000i128);
        env.storage()
            .temporary()
            .set(&(symbol_short!("liquidity"), signal_id), &500i128);

        // Execute a trade
        let _ = AutoTradeContract::execute_trade(
            env.clone(),
            user.clone(),
            signal_id,
            OrderType::Market,
            400,
        )
        .unwrap();

        // Check positions
        let positions = AutoTradeContract::get_user_positions(env.clone(), user.clone());
        assert!(positions.contains_key(1));

        let position = positions.get(1).unwrap();
        assert_eq!(position.amount, 400);
        assert_eq!(position.entry_price, 100);
    });
}

#[test]
fn test_stop_loss_check() {
    let env = setup_env();
    let contract_id = env.register(AutoTradeContract, ());
    let user = Address::generate(&env);

    env.as_contract(&contract_id, || {
        // Setup a position with entry price 100
        risk::update_position(&env, &user, 1, 1000, 100);

        let config = risk::RiskConfig::default(); // 15% stop loss

        // Price at 90 (10% drop) - should NOT trigger
        let triggered = risk::check_stop_loss(&env, &user, 1, 90, &config);
        assert!(!triggered);

        // Price at 80 (20% drop) - should trigger
        let triggered = risk::check_stop_loss(&env, &user, 1, 80, &config);
        assert!(triggered);
    });
}

#[test]
fn test_portfolio_value_calculation() {
    let env = setup_env();
    let contract_id = env.register(AutoTradeContract, ());
    let user = Address::generate(&env);

    env.as_contract(&contract_id, || {
        // Set up positions and prices
        risk::set_asset_price(&env, 1, 100);
        risk::set_asset_price(&env, 2, 200);

        risk::update_position(&env, &user, 1, 1000, 100);
        risk::update_position(&env, &user, 2, 500, 200);

        let total_value = risk::calculate_portfolio_value(&env, &user);
        // (1000 * 100 / 100) + (500 * 200 / 100) = 1000 + 1000 = 2000
        assert_eq!(total_value, 2000);
    });
}
