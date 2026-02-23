#![no_std]

mod errors;
mod events;
mod reputation;
mod types;

use errors::OracleError;
use reputation::{
    adjust_oracle_weight, calculate_reputation, get_oracle_stats, should_remove_oracle,
    slash_oracle, track_oracle_accuracy, SlashReason,
};
use soroban_sdk::{contract, contractimpl, Address, Env, Vec};
use types::{ConsensusPriceData, OracleReputation, PriceSubmission, StorageKey};

#[contract]
pub struct OracleContract;

#[contractimpl]
impl OracleContract {
    /// Initialize the contract with an admin
    pub fn initialize(env: Env, admin: Address) {
        if env.storage().instance().has(&StorageKey::Admin) {
            panic!("already initialized");
        }
        env.storage().instance().set(&StorageKey::Admin, &admin);
    }

    /// Register a new oracle
    pub fn register_oracle(env: Env, admin: Address, oracle: Address) -> Result<(), OracleError> {
        admin.require_auth();
        Self::require_admin(&env, &admin)?;

        let mut oracles = Self::get_oracles(&env);
        if oracles.contains(&oracle) {
            return Err(OracleError::OracleAlreadyExists);
        }

        oracles.push_back(oracle.clone());
        env.storage()
            .persistent()
            .set(&StorageKey::Oracles, &oracles);

        // Initialize with default reputation
        let stats = OracleReputation {
            total_submissions: 0,
            accurate_submissions: 0,
            avg_deviation: 0,
            reputation_score: 50,
            weight: 1,
            last_slash: 0,
        };
        reputation::save_oracle_stats(&env, &oracle, &stats);

        Ok(())
    }

    /// Submit a price from an oracle
    pub fn submit_price(env: Env, oracle: Address, price: i128) -> Result<(), OracleError> {
        oracle.require_auth();

        if price <= 0 {
            return Err(OracleError::InvalidPrice);
        }

        let oracles = Self::get_oracles(&env);
        if !oracles.contains(&oracle) {
            return Err(OracleError::OracleNotFound);
        }

        // Check reputation
        let stats = get_oracle_stats(&env, &oracle);
        if stats.weight == 0 {
            return Err(OracleError::LowReputation);
        }

        let submission = PriceSubmission {
            oracle: oracle.clone(),
            price,
            timestamp: env.ledger().timestamp(),
        };

        let mut submissions = Self::get_price_submissions(&env);
        submissions.push_back(submission);
        env.storage()
            .instance()
            .set(&StorageKey::PriceSubmissions, &submissions);

        events::emit_price_submitted(&env, oracle, price);

        Ok(())
    }

    /// Calculate consensus price and update oracle reputations
    pub fn calculate_consensus(env: Env) -> Result<i128, OracleError> {
        let submissions = Self::get_price_submissions(&env);
        let oracles = Self::get_oracles(&env);

        if submissions.is_empty() {
            return Err(OracleError::InsufficientOracles);
        }

        // Calculate weighted median
        let consensus_price = Self::weighted_median(&env, &submissions);

        // Track accuracy for each oracle
        for i in 0..submissions.len() {
            let submission = submissions.get(i).unwrap();
            track_oracle_accuracy(&env, &submission.oracle, submission.price, consensus_price);

            // Check for major deviation and slash if needed
            let deviation = ((submission.price - consensus_price).abs() * 10000) / consensus_price;
            if deviation > 2000 {
                // 20%
                slash_oracle(&env, &submission.oracle, SlashReason::MajorDeviation);
                events::emit_oracle_slashed(&env, submission.oracle.clone(), "major_deviation", 20);
            }
        }

        // Adjust weights for all oracles
        let mut removed_oracles = Vec::new(&env);
        for i in 0..oracles.len() {
            let oracle = oracles.get(i).unwrap();
            let old_stats = get_oracle_stats(&env, &oracle);
            let old_weight = old_stats.weight;

            let new_weight = adjust_oracle_weight(&env, &oracle);

            if new_weight != old_weight {
                let reputation = calculate_reputation(&env, &oracle);
                events::emit_weight_adjusted(
                    &env,
                    oracle.clone(),
                    old_weight,
                    new_weight,
                    reputation,
                );
            }

            if should_remove_oracle(&env, &oracle) {
                removed_oracles.push_back(oracle.clone());
            }
        }

        // Remove poor performing oracles (but keep minimum 2)
        if oracles.len() - removed_oracles.len() >= 2 {
            for i in 0..removed_oracles.len() {
                let oracle = removed_oracles.get(i).unwrap();
                Self::remove_oracle_internal(&env, &oracle);
                events::emit_oracle_removed(&env, oracle, "Low reputation");
            }
        }

        // Store consensus
        let consensus_data = ConsensusPriceData {
            price: consensus_price,
            timestamp: env.ledger().timestamp(),
            num_oracles: submissions.len(),
        };
        env.storage()
            .persistent()
            .set(&StorageKey::ConsensusPrice, &consensus_data);

        // Clear submissions for next round
        env.storage().instance().set(
            &StorageKey::PriceSubmissions,
            &Vec::<PriceSubmission>::new(&env),
        );

        events::emit_consensus_reached(&env, consensus_price, submissions.len());

        Ok(consensus_price)
    }

    /// Get oracle reputation stats
    pub fn get_oracle_reputation(env: Env, oracle: Address) -> OracleReputation {
        get_oracle_stats(&env, &oracle)
    }

    /// Get all registered oracles
    pub fn get_oracles(env: &Env) -> Vec<Address> {
        env.storage()
            .persistent()
            .get(&StorageKey::Oracles)
            .unwrap_or(Vec::new(env))
    }

    /// Get current consensus price
    pub fn get_consensus_price(env: Env) -> Option<ConsensusPriceData> {
        env.storage().persistent().get(&StorageKey::ConsensusPrice)
    }

    /// Remove an oracle (admin only)
    pub fn remove_oracle(env: Env, admin: Address, oracle: Address) -> Result<(), OracleError> {
        admin.require_auth();
        Self::require_admin(&env, &admin)?;
        Self::remove_oracle_internal(&env, &oracle);
        Ok(())
    }

    // Internal helpers

    fn require_admin(env: &Env, caller: &Address) -> Result<(), OracleError> {
        let admin: Address = env
            .storage()
            .instance()
            .get(&StorageKey::Admin)
            .ok_or(OracleError::Unauthorized)?;

        if caller != &admin {
            return Err(OracleError::Unauthorized);
        }
        Ok(())
    }

    fn get_price_submissions(env: &Env) -> Vec<PriceSubmission> {
        env.storage()
            .instance()
            .get(&StorageKey::PriceSubmissions)
            .unwrap_or(Vec::new(env))
    }

    fn weighted_median(env: &Env, submissions: &Vec<PriceSubmission>) -> i128 {
        if submissions.is_empty() {
            return 0;
        }

        // Create weighted list
        let mut weighted_prices = Vec::new(env);
        for i in 0..submissions.len() {
            let submission = submissions.get(i).unwrap();
            let stats = get_oracle_stats(env, &submission.oracle);
            let weight = stats.weight.max(1);

            for _ in 0..weight {
                weighted_prices.push_back(submission.price);
            }
        }

        // Sort prices
        let len = weighted_prices.len();
        for i in 0..len {
            for j in 0..(len - i - 1) {
                let curr = weighted_prices.get(j).unwrap();
                let next = weighted_prices.get(j + 1).unwrap();
                if curr > next {
                    weighted_prices.set(j, next);
                    weighted_prices.set(j + 1, curr);
                }
            }
        }

        // Return median
        let mid = len / 2;
        if len % 2 == 0 {
            (weighted_prices.get(mid - 1).unwrap() + weighted_prices.get(mid).unwrap()) / 2
        } else {
            weighted_prices.get(mid).unwrap()
        }
    }

    fn remove_oracle_internal(env: &Env, oracle: &Address) {
        let oracles = Self::get_oracles(env);
        let mut new_oracles = Vec::new(env);

        for i in 0..oracles.len() {
            let o = oracles.get(i).unwrap();
            if o != *oracle {
                new_oracles.push_back(o);
            }
        }

        env.storage()
            .persistent()
            .set(&StorageKey::Oracles, &new_oracles);
    }
}

#[cfg(test)]
mod test;
