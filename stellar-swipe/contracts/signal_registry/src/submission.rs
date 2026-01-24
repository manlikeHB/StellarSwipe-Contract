#![allow(dead_code)]

use soroban_sdk::{contracttype, Address, Env, Map, Vec};

use crate::stake::{can_submit_signal, ContractError, StakeInfo, DEFAULT_MINIMUM_STAKE, UNSTAKE_LOCK_PERIOD};

/// Action enum for trading signals
#[contracttype]
#[derive(Clone)]
pub enum Action {
    Buy,
    Sell,
    Hold,
}

