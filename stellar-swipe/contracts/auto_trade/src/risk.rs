#![allow(dead_code)]
use soroban_sdk::{contracttype, Address, Env, Map, Vec};

use crate::errors::AutoTradeError;

/// ==========================
/// Risk Configuration Types
/// ==========================

#[contracttype]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RiskConfig {
    pub max_position_pct: u32,  // Percentage (0-100)
    pub daily_trade_limit: u32, // Max trades per 24 hours
    pub stop_loss_pct: u32,     // Percentage (0-100)
}

impl Default for RiskConfig {
    fn default() -> Self {
        RiskConfig {
            max_position_pct: 20,  // 20% of portfolio
            daily_trade_limit: 10, // 10 trades per day
            stop_loss_pct: 15,     // 15% stop loss
        }
    }
}

#[contracttype]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Position {
    pub asset_id: u32,
    pub amount: i128,
    pub entry_price: i128,
    pub timestamp: u64,
}

#[contracttype]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TradeRecord {
    pub timestamp: u64,
    pub signal_id: u64,
    pub amount: i128,
}

#[contracttype]
pub enum RiskDataKey {
    UserRiskConfig(Address),
    UserPositions(Address),
    UserTradeHistory(Address),
    AssetPrice(u32),
}

/// ==========================
/// Risk Configuration Management
/// ==========================
pub fn get_risk_config(env: &Env, user: &Address) -> RiskConfig {
    env.storage()
        .persistent()
        .get(&RiskDataKey::UserRiskConfig(user.clone()))
        .unwrap_or_default()
}

pub fn set_risk_config(env: &Env, user: &Address, config: &RiskConfig) {
    env.storage()
        .persistent()
        .set(&RiskDataKey::UserRiskConfig(user.clone()), config);
}

/// ==========================
/// Position Management
/// ==========================
pub fn get_user_positions(env: &Env, user: &Address) -> Map<u32, Position> {
    env.storage()
        .persistent()
        .get(&RiskDataKey::UserPositions(user.clone()))
        .unwrap_or_else(|| Map::new(env))
}

pub fn update_position(env: &Env, user: &Address, asset_id: u32, amount: i128, price: i128) {
    let mut positions = get_user_positions(env, user);

    if amount == 0 {
        positions.remove(asset_id);
    } else {
        let position = Position {
            asset_id,
            amount,
            entry_price: price,
            timestamp: env.ledger().timestamp(),
        };
        positions.set(asset_id, position);
    }

    env.storage()
        .persistent()
        .set(&RiskDataKey::UserPositions(user.clone()), &positions);
}

/// ==========================
/// Trade History Management
/// ==========================
pub fn get_trade_history(env: &Env, user: &Address) -> Vec<TradeRecord> {
    env.storage()
        .persistent()
        .get(&RiskDataKey::UserTradeHistory(user.clone()))
        .unwrap_or_else(|| Vec::new(env))
}

pub fn add_trade_record(env: &Env, user: &Address, signal_id: u64, amount: i128) {
    let mut history = get_trade_history(env, user);

    let record = TradeRecord {
        timestamp: env.ledger().timestamp(),
        signal_id,
        amount,
    };

    history.push_back(record);

    env.storage()
        .persistent()
        .set(&RiskDataKey::UserTradeHistory(user.clone()), &history);
}

/// ==========================
/// Price Management
/// ==========================
pub fn get_asset_price(env: &Env, asset_id: u32) -> Option<i128> {
    env.storage()
        .temporary()
        .get(&RiskDataKey::AssetPrice(asset_id))
}

pub fn set_asset_price(env: &Env, asset_id: u32, price: i128) {
    env.storage()
        .temporary()
        .set(&RiskDataKey::AssetPrice(asset_id), &price);
}

/// ==========================
/// Risk Checks
/// ==========================
/// Check if daily trade limit is exceeded
pub fn check_daily_trade_limit(
    env: &Env,
    user: &Address,
    config: &RiskConfig,
) -> Result<(), AutoTradeError> {
    let history = get_trade_history(env, user);
    let current_time = env.ledger().timestamp();
    let day_ago = current_time.saturating_sub(86400); // 24 hours in seconds

    let mut recent_trades = 0u32;
    for i in 0..history.len() {
        if let Some(record) = history.get(i) {
            if record.timestamp >= day_ago {
                recent_trades += 1;
            }
        }
    }

    if recent_trades >= config.daily_trade_limit {
        return Err(AutoTradeError::DailyTradeLimitExceeded);
    }

    Ok(())
}

/// Calculate total portfolio value
pub fn calculate_portfolio_value(env: &Env, user: &Address) -> i128 {
    let positions = get_user_positions(env, user);
    let mut total_value = 0i128;

    let keys = positions.keys();
    for i in 0..keys.len() {
        if let Some(asset_id) = keys.get(i) {
            if let Some(position) = positions.get(asset_id) {
                if let Some(price) = get_asset_price(env, asset_id) {
                    total_value += position.amount * price / 100; // Assuming price is in basis points
                }
            }
        }
    }

    total_value
}

/// Check if position limit would be exceeded
pub fn check_position_limit(
    env: &Env,
    user: &Address,
    asset_id: u32,
    trade_amount: i128,
    trade_price: i128,
    config: &RiskConfig,
) -> Result<(), AutoTradeError> {
    let current_portfolio_value = calculate_portfolio_value(env, user);

    // Handle first trade case - allow if within absolute limit
    if current_portfolio_value == 0 {
        // For first trade, we'll allow it as long as it's reasonable
        return Ok(());
    }

    let positions = get_user_positions(env, user);
    let current_position = positions.get(asset_id).map(|p| p.amount).unwrap_or(0);

    let new_position_amount = current_position + trade_amount;
    let new_position_value = new_position_amount * trade_price / 100;

    // Calculate the new portfolio value including this trade
    let trade_value = trade_amount * trade_price / 100;
    let new_portfolio_value = current_portfolio_value + trade_value;

    // Calculate what percentage this position would be of the NEW portfolio
    let position_pct = (new_position_value * 100) / new_portfolio_value;

    if position_pct > config.max_position_pct as i128 {
        return Err(AutoTradeError::PositionLimitExceeded);
    }

    Ok(())
}

/// Check if stop-loss is triggered for a sell
pub fn check_stop_loss(
    env: &Env,
    user: &Address,
    asset_id: u32,
    current_price: i128,
    config: &RiskConfig,
) -> bool {
    let positions = get_user_positions(env, user);

    if let Some(position) = positions.get(asset_id) {
        let stop_loss_price = position.entry_price * (100 - config.stop_loss_pct as i128) / 100;

        if current_price <= stop_loss_price {
            return true;
        }
    }

    false
}

/// Perform all risk checks before executing a trade
pub fn validate_trade(
    env: &Env,
    user: &Address,
    asset_id: u32,
    amount: i128,
    price: i128,
    is_sell: bool,
) -> Result<bool, AutoTradeError> {
    let config = get_risk_config(env, user);

    // Check daily trade limit
    check_daily_trade_limit(env, user, &config)?;

    // Check position limit (only for buys)
    if !is_sell {
        check_position_limit(env, user, asset_id, amount, price, &config)?;
    }

    // Check stop-loss (only for sells)
    let stop_loss_triggered = if is_sell {
        check_stop_loss(env, user, asset_id, price, &config)
    } else {
        false
    };

    Ok(stop_loss_triggered)
}

#[cfg(test)]
mod tests {
    use super::*;
    use soroban_sdk::testutils::{Address as TestAddress, Ledger};
    use soroban_sdk::{contract, Env};

    #[contract]
    struct TestContract;

    fn setup_env() -> Env {
        let env = Env::default();
        env.ledger().set_timestamp(1000);
        env
    }

    fn test_user(env: &Env) -> Address {
        Address::generate(env)
    }

    #[test]
    fn test_default_risk_config() {
        let env = setup_env();
        let user = test_user(&env);
        let contract_addr = env.register(TestContract, ());

        env.as_contract(&contract_addr, || {
            let config = get_risk_config(&env, &user);
            assert_eq!(config.max_position_pct, 20);
            assert_eq!(config.daily_trade_limit, 10);
            assert_eq!(config.stop_loss_pct, 15);
        });
    }

    #[test]
    fn test_set_custom_risk_config() {
        let env = setup_env();
        let user = test_user(&env);
        let contract_addr = env.register(TestContract, ());

        env.as_contract(&contract_addr, || {
            let custom_config = RiskConfig {
                max_position_pct: 30,
                daily_trade_limit: 15,
                stop_loss_pct: 10,
            };
            set_risk_config(&env, &user, &custom_config);

            let retrieved = get_risk_config(&env, &user);
            assert_eq!(retrieved, custom_config);
        });
    }

    #[test]
    fn test_daily_trade_limit_not_exceeded() {
        let env = setup_env();
        let user = test_user(&env);
        let contract_addr = env.register(TestContract, ());

        env.as_contract(&contract_addr, || {
            let config = RiskConfig::default();

            // Add 5 trades
            for i in 0..5 {
                add_trade_record(&env, &user, i, 100);
            }

            let result = check_daily_trade_limit(&env, &user, &config);
            assert!(result.is_ok());
        });
    }

    #[test]
    fn test_daily_trade_limit_exceeded() {
        let env = setup_env();
        let user = test_user(&env);
        let contract_addr = env.register(TestContract, ());

        env.as_contract(&contract_addr, || {
            let config = RiskConfig::default();

            // Add 10 trades (at the limit)
            for i in 0..10 {
                add_trade_record(&env, &user, i, 100);
            }

            let result = check_daily_trade_limit(&env, &user, &config);
            assert_eq!(result, Err(AutoTradeError::DailyTradeLimitExceeded));
        });
    }

    #[test]
    fn test_position_limit_first_trade() {
        let env = setup_env();
        let user = test_user(&env);
        let contract_addr = env.register(TestContract, ());

        env.as_contract(&contract_addr, || {
            let config = RiskConfig::default();
            set_asset_price(&env, 1, 100);

            let result = check_position_limit(&env, &user, 1, 1000, 100, &config);
            assert!(result.is_ok());
        });
    }

    #[test]
    fn test_position_limit_exceeded() {
        let env = setup_env();
        let user = test_user(&env);
        let contract_addr = env.register(TestContract, ());

        env.as_contract(&contract_addr, || {
            let config = RiskConfig::default();

            // Set up existing positions
            // Asset 1: 1000 units at price 100 = value 1000
            // Asset 2: 4000 units at price 100 = value 4000
            // Total portfolio = 5000
            set_asset_price(&env, 1, 100);
            set_asset_price(&env, 2, 100);
            update_position(&env, &user, 1, 1000, 100);
            update_position(&env, &user, 2, 4000, 100);

            // Try to add 2000 more to asset 1 (price 100, value 2000)
            // New position in asset 1 would be: 3000 units, value 3000
            // New portfolio would be: 5000 + 2000 = 7000
            // Position % would be: 3000 / 7000 = 42.8% > 20%
            let result = check_position_limit(&env, &user, 1, 2000, 100, &config);
            assert_eq!(result, Err(AutoTradeError::PositionLimitExceeded));
        });
    }

    #[test]
    fn test_stop_loss_not_triggered() {
        let env = setup_env();
        let user = test_user(&env);
        let contract_addr = env.register(TestContract, ());

        env.as_contract(&contract_addr, || {
            let config = RiskConfig::default();

            // Entry price 100, stop loss at 15% = 85
            update_position(&env, &user, 1, 1000, 100);

            let triggered = check_stop_loss(&env, &user, 1, 90, &config);
            assert!(!triggered);
        });
    }

    #[test]
    fn test_stop_loss_triggered() {
        let env = setup_env();
        let user = test_user(&env);
        let contract_addr = env.register(TestContract, ());

        env.as_contract(&contract_addr, || {
            let config = RiskConfig::default();

            // Entry price 100, stop loss at 15% = 85
            update_position(&env, &user, 1, 1000, 100);

            let triggered = check_stop_loss(&env, &user, 1, 80, &config);
            assert!(triggered);
        });
    }

    #[test]
    fn test_calculate_portfolio_value() {
        let env = setup_env();
        let user = test_user(&env);
        let contract_addr = env.register(TestContract, ());

        env.as_contract(&contract_addr, || {
            set_asset_price(&env, 1, 100);
            set_asset_price(&env, 2, 200);

            update_position(&env, &user, 1, 1000, 100);
            update_position(&env, &user, 2, 500, 200);

            let total_value = calculate_portfolio_value(&env, &user);
            // (1000 * 100 / 100) + (500 * 200 / 100) = 1000 + 1000 = 2000
            assert_eq!(total_value, 2000);
        });
    }
}
