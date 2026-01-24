#![no_std]

use soroban_sdk::{contract, contractimpl, Address, Env, Symbol, Map, events::Event};

/// Information about a provider's stake
#[derive(Clone)]
pub struct StakeInfo {
    pub amount: i128,           // Amount staked (XLM stroops)
    pub last_signal_time: u64,  // Timestamp of last signal submission
}

// Storage key for all stakes
const STAKES_KEY: Symbol = Symbol::short("stakes");

// Default minimum stake: 100 XLM (expressed in stroops, 1 XLM = 1_000_000 stroops)
pub const DEFAULT_MINIMUM_STAKE: i128 = 100_000_000;

// Lock period after last signal: 7 days
pub const UNSTAKE_LOCK_PERIOD: u64 = 7 * 24 * 60 * 60;

#[contract]
pub struct StakeRegistry;

#[contractimpl]
impl StakeRegistry {
    /// Stake XLM into the contract
    /// The amount must be positive
    pub fn stake(env: Env, provider: Address, amount: i128) {
        if amount <= 0 {
            panic!("Stake amount must be positive");
        }

        // Load stakes map
        let mut stakes: Map<Address, StakeInfo> = env.storage().get(&STAKES_KEY).unwrap_or_default();

        let mut info = stakes.get(&provider).unwrap_or(StakeInfo {
            amount: 0,
            last_signal_time: 0,
        });

        info.amount += amount;

        stakes.set(provider.clone(), info.clone());
        env.storage().set(&STAKES_KEY, &stakes);

        // Emit stake event for frontend tracking
        env.events().publish(
            (Symbol::short("stake"), provider.clone()),
            info.amount,
        );
    }

    /// Unstake XLM after lock period
    pub fn unstake(env: Env, provider: Address) -> i128 {
        let mut stakes: Map<Address, StakeInfo> = env.storage().get(&STAKES_KEY).unwrap_or_default();
        let mut info = stakes.get(&provider).expect("No stake found for provider");

        let now = env.ledger().timestamp();
        if now < info.last_signal_time + UNSTAKE_LOCK_PERIOD {
            panic!("Stake is locked until 7 days after last signal");
        }

        if info.amount <= 0 {
            panic!("No staked funds to withdraw");
        }

        let amount = info.amount;
        info.amount = 0;

        stakes.set(provider.clone(), info);
        env.storage().set(&STAKES_KEY, &stakes);

        // Emit unstake event
        env.events().publish(
            (Symbol::short("unstake"), provider.clone()),
            amount,
        );

        amount
    }

    /// Verify minimum stake before submitting a signal
    pub fn verify_stake(env: Env, provider: Address, minimum_stake: i128) {
        let stakes: Map<Address, StakeInfo> = env.storage().get(&STAKES_KEY).unwrap_or_default();
        let info = stakes.get(&provider).expect("No stake found");

        if info.amount < minimum_stake {
            panic!("Insufficient stake to submit signal");
        }
    }

    /// Update last signal time (call after successful signal submission)
    pub fn update_last_signal_time(env: Env, provider: Address) {
        let mut stakes: Map<Address, StakeInfo> = env.storage().get(&STAKES_KEY).unwrap_or_default();
        let mut info = stakes.get(&provider).expect("No stake found");
        info.last_signal_time = env.ledger().timestamp();
        stakes.set(provider.clone(), info);
        env.storage().set(&STAKES_KEY, &stakes);
    }

    /// Get stake info for a provider
    pub fn get_stake(env: Env, provider: Address) -> StakeInfo {
        let stakes: Map<Address, StakeInfo> = env.storage().get(&STAKES_KEY).unwrap_or_default();
        stakes.get(&provider).unwrap_or(StakeInfo {
            amount: 0,
            last_signal_time: 0,
        })
    }
}

#![cfg(test)]
#[test]
fn test_stake_and_unstake_flow() {
    let env = Env::default();
    env.mock_all_auths();

    let provider = Address::generate(&env);
    let contract_id = env.register_contract(None, StakeRegistry);
    let client = StakeRegistryClient::new(&env, &contract_id);

    // Stake 100 XLM
    client.stake(&provider, 100_000_000);

    // Check stake info
    let info = client.get_stake(&provider).unwrap();
    assert_eq!(info.amount, 100_000_000);

    // Try unstake before lock period -> should fail
    let res = std::panic::catch_unwind(|| client.unstake(&provider));
    assert!(res.is_err());

    // Simulate 7 days passing
    env.ledger().set_timestamp(env.ledger().timestamp() + 7 * 24 * 60 * 60);

    // Unstake succeeds
    let withdrawn = client.unstake(&provider);
    assert_eq!(withdrawn, 100_000_000);
}
