#![no_std]

mod types;

use soroban_sdk::{
    contract, contractimpl, contracttype, Address, Bytes, Env, Map, String,
};

use types::{
    Signal,
    SignalAction,
    SignalStats,
    SignalStatus,
};

const MAX_EXPIRY_SECONDS: u64 = 30 * 24 * 60 * 60; // 30 days

#[contract]
pub struct SignalRegistry;

#[contracttype]
#[derive(Clone)]
enum StorageKey {
    SignalCounter,
    Signals,
    ProviderStats,
}

#[contractimpl]
impl SignalRegistry {
    /* =========================
       INTERNAL HELPERS
    ========================== */

    fn next_signal_id(env: &Env) -> u64 {
        let mut counter: u64 = env
            .storage()
            .instance()
            .get(&StorageKey::SignalCounter)
            .unwrap_or(0);

        counter = counter.checked_add(1).expect("signal id overflow");

        env.storage()
            .instance()
            .set(&StorageKey::SignalCounter, &counter);

        counter
    }

    fn get_signals_map(env: &Env) -> Map<u64, Signal> {
        env.storage()
            .instance()
            .get(&StorageKey::Signals)
            .unwrap_or(Map::new(env))
    }

    fn save_signals_map(env: &Env, map: &Map<u64, Signal>) {
        env.storage()
            .instance()
            .set(&StorageKey::Signals, map);
    }

    fn get_provider_stats_map(env: &Env) -> Map<Address, SignalStats> {
        env.storage()
            .instance()
            .get(&StorageKey::ProviderStats)
            .unwrap_or(Map::new(env))
    }

    fn save_provider_stats_map(env: &Env, map: &Map<Address, SignalStats>) {
        env.storage()
            .instance()
            .set(&StorageKey::ProviderStats, map);
    }

    fn validate_asset_pair(env: &Env, asset_pair: &String) {
        let bytes: Bytes = asset_pair.clone().into_bytes();

        let mut has_slash = false;
        for i in 0..bytes.len() {
            if bytes.get(i).unwrap() == b'/' {
                has_slash = true;
                break;
            }
        }

        if !has_slash {
            panic!("invalid asset pair");
        }
    }

    /* =========================
       PUBLIC API
    ========================== */

    pub fn create_signal(
        env: Env,
        provider: Address,
        asset_pair: String,
        action: SignalAction,
        price: i128,
        rationale: String,
        expiry: u64,
    ) -> u64 {
        provider.require_auth();

        Self::validate_asset_pair(&env, &asset_pair);

        let now = env.ledger().timestamp();

        if expiry <= now {
            panic!("expiry must be in the future");
        }

        if expiry > now + MAX_EXPIRY_SECONDS {
            panic!("expiry exceeds max 30 days");
        }

        let id = Self::next_signal_id(&env);

        let signal = Signal {
            id,
            provider: provider.clone(),
            asset_pair,
            action,
            price,
            rationale,
            timestamp: now,
            expiry,
            status: SignalStatus::Active,
        };

        // Store signal
        let mut signals = Self::get_signals_map(&env);
        signals.set(id, signal);
        Self::save_signals_map(&env, &signals);

        // Initialize provider stats on first submission
        let mut stats = Self::get_provider_stats_map(&env);
        if !stats.contains_key(provider.clone()) {
            stats.set(provider, SignalStats::default());
            Self::save_provider_stats_map(&env, &stats);
        }

        id
    }

    pub fn get_signal(env: Env, signal_id: u64) -> Option<Signal> {
        let signals = Self::get_signals_map(&env);
        signals.get(signal_id)
    }

    pub fn get_provider_stats(env: Env, provider: Address) -> Option<SignalStats> {
        let stats = Self::get_provider_stats_map(&env);
        stats.get(provider)
    }
}
