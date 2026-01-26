#![no_std]

use soroban_sdk::{
    contract, contractimpl, contracttype, Address, Env, Symbol,
};

mod sdex;
mod storage;
mod errors;

// use sdex::*;
use errors::AutoTradeError;

/// ==========================
/// Types
/// ==========================

#[contracttype]
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum OrderType {
    Market,
    Limit,
}

#[contracttype]
#[derive(Clone)]
pub enum TradeStatus {
    Pending,
    PartiallyFilled,
    Filled,
    Failed,
}

#[contracttype]
#[derive(Clone)]
pub struct Trade {
    pub signal_id: u64,
    pub user: Address,
    pub requested_amount: i128,
    pub executed_amount: i128,
    pub executed_price: i128,
    pub timestamp: u64,
    pub status: TradeStatus,
}

#[contracttype]
pub struct TradeResult {
    pub trade: Trade,
}

/// ==========================
/// Storage Keys
/// ==========================

#[contracttype]
enum DataKey {
    Trades(Address, u64), // (user, signal_id)
}

/// ==========================
/// Contract
/// ==========================

#[contract]
pub struct AutoTradeContract;

/// ==========================
/// Implementation
/// ==========================

#[contractimpl]
impl AutoTradeContract {
    /// Execute a trade on behalf of a user based on a signal
    pub fn execute_trade(
        env: Env,
        user: Address,
        signal_id: u64,
        order_type: OrderType,
        amount: i128,
    ) -> Result<TradeResult, AutoTradeError> {
        // ----------------------
        // Basic validation
        // ----------------------
        if amount <= 0 {
            return Err(AutoTradeError::InvalidAmount);
        }

        user.require_auth();

        // ----------------------
        // Validate signal
        // ----------------------
        let signal = storage::get_signal(&env, signal_id)
            .ok_or(AutoTradeError::SignalNotFound)?;

        if env.ledger().timestamp() > signal.expiry {
            return Err(AutoTradeError::SignalExpired);
        }

        // ----------------------
        // Authorization check
        // ----------------------
        if !storage::is_authorized(&env, &user) {
            return Err(AutoTradeError::Unauthorized);
        }

        // ----------------------
        // Balance check
        // ----------------------
        let balance_ok = sdex::has_sufficient_balance(
            &env,
            &user,
            &signal.base_asset,
            amount,
        );

        if !balance_ok {
            return Err(AutoTradeError::InsufficientBalance);
        }

        // ----------------------
        // Execute order on SDEX
        // ----------------------
        let execution = match order_type {
            OrderType::Market => {
                sdex::execute_market_order(&env, &user, &signal, amount)?
            }
            OrderType::Limit => {
                sdex::execute_limit_order(&env, &user, &signal, amount)?
            }
        };

        // ----------------------
        // Determine trade status
        // ----------------------
        let status = if execution.executed_amount == 0 {
            TradeStatus::Failed
        } else if execution.executed_amount < amount {
            TradeStatus::PartiallyFilled
        } else {
            TradeStatus::Filled
        };

        // ----------------------
        // Persist trade
        // ----------------------
        let trade = Trade {
            signal_id,
            user: user.clone(),
            requested_amount: amount,
            executed_amount: execution.executed_amount,
            executed_price: execution.executed_price,
            timestamp: env.ledger().timestamp(),
            status,
        };

        env.storage().persistent().set(
            &DataKey::Trades(user.clone(), signal_id),
            &trade,
        );

        // ----------------------
        // Emit event
        // ----------------------
        #[allow(deprecated)]
        env.events().publish(
            (Symbol::new(&env, "trade_executed"), user, signal_id),
            trade.clone(),
        );

        Ok(TradeResult { trade })
    }

    /// Fetch executed trade by user + signal
    pub fn get_trade(env: Env, user: Address, signal_id: u64) -> Option<Trade> {
        env.storage()
            .persistent()
            .get(&DataKey::Trades(user, signal_id))
    }
}
