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
    true
}

pub fn execute_market_order(
    _env: &Env,
    _user: &Address,
    signal: &Signal,
    amount: i128,
) -> Result<ExecutionResult, AutoTradeError> {
    Ok(ExecutionResult {
        executed_amount: amount,
        executed_price: signal.price,
    })
}

pub fn execute_limit_order(
    _env: &Env,
    _user: &Address,
    signal: &Signal,
    amount: i128,
) -> Result<ExecutionResult, AutoTradeError> {
    Ok(ExecutionResult {
        executed_amount: amount,
        executed_price: signal.price,
    })
}
