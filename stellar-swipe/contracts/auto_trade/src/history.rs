#![allow(dead_code)]
//! Trade history storage and query.
//!
//! Stores all executed trades per user with full details.
//! Gas: ~O(limit) per get_trade_history query.

use soroban_sdk::{contracttype, Address, Env, Vec};
/// Default page size for trade history
pub const DEFAULT_HISTORY_LIMIT: u32 = 20;

/// Maximum trades per page
pub const MAX_HISTORY_LIMIT: u32 = 100;

#[contracttype]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HistoryTradeStatus {
    Pending,
    Executed,
    Failed,
    Cancelled,
}

#[contracttype]
#[derive(Clone, Debug)]
pub struct HistoryTrade {
    pub id: u64,
    pub signal_id: u64,
    pub base_asset: u32,
    pub amount: i128,
    pub price: i128,
    pub fee: i128,
    pub timestamp: u64,
    pub status: HistoryTradeStatus,
}

#[contracttype]
pub enum HistoryDataKey {
    UserTradeCount(Address),
    Trade(Address, u64),
}

/// Get number of trades for a user
pub fn get_user_trade_count(env: &Env, user: &Address) -> u64 {
    env.storage()
        .persistent()
        .get(&HistoryDataKey::UserTradeCount(user.clone()))
        .unwrap_or(0)
}

/// Record a trade to history. Called after successful execution.
pub fn record_trade(
    env: &Env,
    user: &Address,
    signal_id: u64,
    base_asset: u32,
    amount: i128,
    price: i128,
    fee: i128,
    status: HistoryTradeStatus,
) -> u64 {
    let count = get_user_trade_count(env, user);
    let id = count;

    let trade = HistoryTrade {
        id,
        signal_id,
        base_asset,
        amount,
        price,
        fee,
        timestamp: env.ledger().timestamp(),
        status,
    };

    env.storage()
        .persistent()
        .set(&HistoryDataKey::Trade(user.clone(), id), &trade);

    let new_count = count + 1;
    env.storage()
        .persistent()
        .set(&HistoryDataKey::UserTradeCount(user.clone()), &new_count);

    id
}

/// Get trade by user and index
fn get_trade_by_index(env: &Env, user: &Address, index: u64) -> Option<HistoryTrade> {
    env.storage()
        .persistent()
        .get(&HistoryDataKey::Trade(user.clone(), index))
}

/// Get trade history for user, newest first, with pagination.
pub fn get_trade_history(env: &Env, user: &Address, offset: u32, limit: u32) -> Vec<HistoryTrade> {
    let count = get_user_trade_count(env, user);
    if count == 0 {
        return Vec::new(env);
    }

    let limit = if limit == 0 {
        DEFAULT_HISTORY_LIMIT
    } else if limit > MAX_HISTORY_LIMIT {
        MAX_HISTORY_LIMIT
    } else {
        limit
    };

    let mut result = Vec::new(env);
    let total = count as u64;
    let mut taken = 0u32;
    let mut skipped = 0u32;

    for i in (0..total).rev() {
        if skipped < offset {
            skipped += 1;
            continue;
        }
        if taken >= limit {
            break;
        }
        if let Some(trade) = get_trade_by_index(env, user, i) {
            result.push_back(trade);
            taken += 1;
        }
    }

    result
}
