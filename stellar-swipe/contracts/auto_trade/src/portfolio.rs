#![allow(dead_code)]
//! Portfolio calculation and P&L tracking.
//!
//! Builds portfolio from positions with current values and unrealized P&L.

use soroban_sdk::{contracttype, Address, Env, Vec};

use crate::risk;

#[contracttype]
#[derive(Clone, Debug)]
pub struct AssetHolding {
    pub asset_id: u32,
    pub amount: i128,
    pub current_value_xlm: i128,
    pub avg_entry_price: i128,
    pub unrealized_pnl: i128,
}

#[contracttype]
#[derive(Clone, Debug)]
pub struct Portfolio {
    pub assets: Vec<AssetHolding>,
    pub total_value_xlm: i128,
    pub total_pnl: i128,
}

/// Get portfolio for user. Uses risk::get_user_positions and risk::get_asset_price.
pub fn get_portfolio(env: &Env, user: &Address) -> Portfolio {
    let positions = risk::get_user_positions(env, user);
    let mut assets = Vec::new(env);
    let mut total_value_xlm = 0i128;
    let mut total_pnl = 0i128;

    let keys = positions.keys();
    for i in 0..keys.len() {
        if let Some(asset_id) = keys.get(i) {
            if let Some(position) = positions.get(asset_id) {
                let current_price =
                    risk::get_asset_price(env, asset_id).unwrap_or(position.entry_price);
                let current_value_xlm = position.amount * current_price;
                let unrealized_pnl = (current_price - position.entry_price) * position.amount;

                total_value_xlm = total_value_xlm + current_value_xlm;
                total_pnl = total_pnl + unrealized_pnl;

                assets.push_back(AssetHolding {
                    asset_id,
                    amount: position.amount,
                    current_value_xlm,
                    avg_entry_price: position.entry_price,
                    unrealized_pnl,
                });
            }
        }
    }

    Portfolio {
        assets,
        total_value_xlm,
        total_pnl,
    }
}
