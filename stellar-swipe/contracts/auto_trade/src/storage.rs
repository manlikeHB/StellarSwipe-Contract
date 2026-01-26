#![no_std]

use soroban_sdk::{contracttype, Address, Env};

#[contracttype]
#[derive(Clone)]
pub struct Signal {
    pub id: u64,
    pub expiry: u64,
    pub base_asset: u32,   // placeholder
    pub quote_asset: u32,  // placeholder
    pub price: i128,
}

/// Fetch signal (mock for now)
pub fn get_signal(_env: &Env, _signal_id: u64) -> Option<Signal> {
    None
}

/// Authorization check (mock)
pub fn is_authorized(_env: &Env, _user: &Address) -> bool {
    true
}
