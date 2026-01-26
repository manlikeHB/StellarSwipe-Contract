use soroban_sdk::{contracttype, Address, Env};

use crate::errors::AutoTradeError;
use crate::storage::Signal;

/// Result returned by SDEX adapter
#[contracttype]
#[derive(Clone, Debug, PartialEq)]
pub struct ExecutionResult {
    pub executed_amount: i128,
    pub executed_price: i128,
}

/// Simulated on-chain balance check
/// In production: asset contract / trustline verification
pub fn has_sufficient_balance(
    env: &Env,
    user: &Address,
    _asset: &u32,
    amount: i128,
) -> bool {
    let key = (user.clone(), "balance");
    let balance: i128 = env
        .storage()
        .temporary()
        .get(&key)
        .unwrap_or(0);

    balance >= amount
}

/// Mock MARKET order execution
pub fn execute_market_order(
    env: &Env,
    _user: &Address,
    signal: &Signal,
    amount: i128,
) -> Result<ExecutionResult, AutoTradeError> {
    let now = env.ledger().timestamp();

    if now >= signal.expiry {
        return Err(AutoTradeError::SignalExpired);
    }

    // Simulated orderbook depth
    let available_liquidity: i128 = env
        .storage()
        .temporary()
        .get(&("liquidity", signal.signal_id))
        .unwrap_or(amount);

    if available_liquidity <= 0 {
        return Err(AutoTradeError::InsufficientLiquidity);
    }

    let executed_amount = core::cmp::min(amount, available_liquidity);

    Ok(ExecutionResult {
        executed_amount,
        executed_price: signal.price, // aggressive crossing price
    })
}

/// Mock LIMIT order execution
pub fn execute_limit_order(
    env: &Env,
    _user: &Address,
    signal: &Signal,
    amount: i128,
) -> Result<ExecutionResult, AutoTradeError> {
    let now = env.ledger().timestamp();

    if now >= signal.expiry {
        return Err(AutoTradeError::SignalExpired);
    }

    let market_price: i128 = env
        .storage()
        .temporary()
        .get(&("market_price", signal.signal_id))
        .unwrap_or(signal.price);

    // Limit condition not met
    if market_price > signal.price {
        return Ok(ExecutionResult {
            executed_amount: 0,
            executed_price: 0,
        });
    }

    Ok(ExecutionResult {
        executed_amount: amount,
        executed_price: signal.price,
    })
}
