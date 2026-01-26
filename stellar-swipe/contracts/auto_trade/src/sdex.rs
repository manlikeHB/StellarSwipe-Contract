use soroban_sdk::{Env, Address};

use crate::storage::Signal;
use crate::errors::AutoTradeError;

pub struct ExecutionResult {
    pub executed_amount: i128,
    pub executed_price: i128,
}

pub fn has_sufficient_balance(
    _env: &Env,
    _user: &Address,
    _asset: &u32,
    _amount: i128,
) -> bool {
    // TODO: query balance via auth framework / asset contract
    true
}

/// MARKET ORDER
pub fn execute_market_order(
    env: &Env,
    user: &Address,
    signal: &Signal,
    amount: i128,
) -> Result<ExecutionResult, AutoTradeError> {
    let now = env.ledger().timestamp();

    if now >= signal.expiry {
        return Err(AutoTradeError::SignalExpired);
    }

    // In real SDEX:
    // - manage_buy_offer / manage_sell_offer
    // - price set aggressively to cross spread
    // - expiration = signal.expiry

    Ok(ExecutionResult {
        executed_amount: amount,          // partial fills handled upstream
        executed_price: signal.price,     // approximated market price
    })
}

/// LIMIT ORDER
pub fn execute_limit_order(
    env: &Env,
    user: &Address,
    signal: &Signal,
    amount: i128,
) -> Result<ExecutionResult, AutoTradeError> {
    let now = env.ledger().timestamp();

    if now >= signal.expiry {
        return Err(AutoTradeError::SignalExpired);
    }

 
    Ok(ExecutionResult {
        executed_amount: amount,
        executed_price: signal.price,
    })
}
