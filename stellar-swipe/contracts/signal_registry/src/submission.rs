#![allow(dead_code)]
use crate::stake::{can_submit_signal, StakeInfo, DEFAULT_MINIMUM_STAKE};
use soroban_sdk::{contracttype, Address, Env, Map, String};

#[contracttype]
#[derive(Clone, Debug, PartialEq)]
pub enum Action {
    Buy,
    Sell,
    Hold,
}

#[contracttype]
#[derive(Clone)]
pub struct Signal {
    pub provider: Address,
    pub asset_pair: String,
    pub action: Action,
    pub price: i128,
    pub rationale: String,
    pub timestamp: u64,
    pub expiry: u64,
}

#[derive(Debug, PartialEq)]
pub enum Error {
    NoStake,
    BelowMinimumStake,
    InvalidAssetPair,
    InvalidPrice,
    EmptyRationale,
    DuplicateSignal,
}

#[allow(clippy::too_many_arguments, clippy::manual_range_contains)]
pub fn submit_signal(
    env: &Env,
    storage: &mut Map<u64, Signal>,
    provider_stakes: &Map<Address, StakeInfo>,
    provider: &Address,
    asset_pair: String,
    action: Action,
    price: i128,
    rationale: String,
) -> Result<u64, Error> {
    // Verify provider stake
    can_submit_signal(provider_stakes, provider).map_err(|_| Error::NoStake)?;
    let stake_info = provider_stakes.get(provider.clone()).unwrap();
    if stake_info.amount < DEFAULT_MINIMUM_STAKE {
        return Err(Error::BelowMinimumStake);
    }

    // Validate asset pair
    let asset_bytes = asset_pair.to_bytes();
    let has_slash = asset_bytes.iter().any(|b| b == b'/');
    let len = asset_bytes.len();
    if !has_slash || len < 5 || len > 20 {
        return Err(Error::InvalidAssetPair);
    }

    // Validate price
    if price <= 0 {
        return Err(Error::InvalidPrice);
    }

    // Validate rationale
    let rationale_len = rationale.to_bytes().len();
    if rationale_len == 0 || rationale_len > 500 {
        return Err(Error::EmptyRationale);
    }

    // Check for duplicate signals in the last 1 hour
    let now = env.ledger().timestamp();
    for (_, sig) in storage.iter() {
        if sig.provider == *provider
            && sig.asset_pair.to_bytes() == asset_pair.to_bytes()
            && sig.action == action
            && sig.price == price
            && now < sig.timestamp + 3600
        {
            return Err(Error::DuplicateSignal);
        }
    }

    // Generate signal ID
    let next_id = storage.len() as u64 + 1;

    // Set expiry (24 hours default)
    let expiry = now + 86400;

    // Store the signal
    let signal = Signal {
        provider: provider.clone(),
        asset_pair,
        action,
        price,
        rationale,
        timestamp: now,
        expiry,
    };
    storage.set(next_id, signal);

    Ok(next_id)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::stake::{stake, StakeInfo, DEFAULT_MINIMUM_STAKE};
    use soroban_sdk::{testutils::Address as TestAddress, Env, Map};

    fn sdk_string(env: &Env, s: &str) -> String {
        #[allow(deprecated)]
        String::from_slice(env, s)
    }

    fn setup_env() -> Env {
        Env::default()
    }

    fn sample_provider(env: &Env) -> Address {
        <Address as TestAddress>::generate(env)
    }

    #[test]
    fn test_submit_signal_success() {
        let env = setup_env();
        let mut stakes: Map<Address, StakeInfo> = Map::new(&env);
        let mut signals: Map<u64, Signal> = Map::new(&env);
        let provider = sample_provider(&env);

        stake(&env, &mut stakes, &provider, DEFAULT_MINIMUM_STAKE).unwrap();

        let signal_id = submit_signal(
            &env,
            &mut signals,
            &stakes,
            &provider,
            sdk_string(&env, "XLM/USDC"),
            Action::Buy,
            120_000_000,
            sdk_string(&env, "Bullish on XLM"),
        )
        .unwrap();

        assert_eq!(signal_id, 1);
        let stored = signals.get(signal_id).unwrap();
        assert_eq!(stored.provider, provider);
        assert_eq!(
            stored.asset_pair.to_bytes(),
            sdk_string(&env, "XLM/USDC").to_bytes()
        );
        assert_eq!(stored.action, Action::Buy);
        assert_eq!(stored.price, 120_000_000);
        assert_eq!(
            stored.rationale.to_bytes(),
            sdk_string(&env, "Bullish on XLM").to_bytes()
        );
    }

    #[test]
    fn test_submit_signal_no_stake() {
        let env = setup_env();
        let stakes: Map<Address, StakeInfo> = Map::new(&env);
        let mut signals: Map<u64, Signal> = Map::new(&env);
        let provider = sample_provider(&env);

        let res = submit_signal(
            &env,
            &mut signals,
            &stakes,
            &provider,
            sdk_string(&env, "XLM/USDC"),
            Action::Buy,
            120_000_000,
            sdk_string(&env, "Bullish on XLM"),
        );

        assert_eq!(res, Err(Error::NoStake));
    }

    #[test]
    fn test_submit_signal_invalid_price() {
        let env = setup_env();
        let mut stakes: Map<Address, StakeInfo> = Map::new(&env);
        let mut signals: Map<u64, Signal> = Map::new(&env);
        let provider = sample_provider(&env);

        stake(&env, &mut stakes, &provider, DEFAULT_MINIMUM_STAKE).unwrap();

        let res = submit_signal(
            &env,
            &mut signals,
            &stakes,
            &provider,
            sdk_string(&env, "XLM/USDC"),
            Action::Buy,
            0,
            sdk_string(&env, "Bullish on XLM"),
        );

        assert_eq!(res, Err(Error::InvalidPrice));
    }

    #[test]
    fn test_submit_signal_empty_rationale() {
        let env = setup_env();
        let mut stakes: Map<Address, StakeInfo> = Map::new(&env);
        let mut signals: Map<u64, Signal> = Map::new(&env);
        let provider = sample_provider(&env);

        stake(&env, &mut stakes, &provider, DEFAULT_MINIMUM_STAKE).unwrap();

        let res = submit_signal(
            &env,
            &mut signals,
            &stakes,
            &provider,
            sdk_string(&env, "XLM/USDC"),
            Action::Buy,
            100_000_000,
            sdk_string(&env, ""),
        );

        assert_eq!(res, Err(Error::EmptyRationale));
    }

    #[test]
    fn test_submit_signal_duplicate() {
        let env = setup_env();
        let mut stakes: Map<Address, StakeInfo> = Map::new(&env);
        let mut signals: Map<u64, Signal> = Map::new(&env);
        let provider = sample_provider(&env);

        stake(&env, &mut stakes, &provider, DEFAULT_MINIMUM_STAKE).unwrap();

        let _ = submit_signal(
            &env,
            &mut signals,
            &stakes,
            &provider,
            sdk_string(&env, "XLM/USDC"),
            Action::Buy,
            120_000_000,
            sdk_string(&env, "Bullish"),
        )
        .unwrap();

        let res = submit_signal(
            &env,
            &mut signals,
            &stakes,
            &provider,
            sdk_string(&env, "XLM/USDC"),
            Action::Buy,
            120_000_000,
            sdk_string(&env, "Bullish"),
        );

        assert_eq!(res, Err(Error::DuplicateSignal));
    }

    #[test]
    fn test_submit_signal_below_minimum_stake() {
        let env = setup_env();
        let mut stakes: Map<Address, StakeInfo> = Map::new(&env);
        let mut signals: Map<u64, Signal> = Map::new(&env);
        let provider = sample_provider(&env);

        let below_min = DEFAULT_MINIMUM_STAKE / 2;

        let low_stake = StakeInfo {
            amount: below_min,
            locked_until: 0,
            last_signal_time: 0,
        };
        stakes.set(provider.clone(), low_stake);

        let res = submit_signal(
            &env,
            &mut signals,
            &stakes,
            &provider,
            sdk_string(&env, "XLM/USDC"),
            Action::Buy,
            100_000_000,
            sdk_string(&env, "Bullish"),
        );

        assert_eq!(res, Err(Error::NoStake));
    }

    #[test]
    fn test_submit_signal_invalid_asset_pair() {
        let env = setup_env();
        let mut stakes: Map<Address, StakeInfo> = Map::new(&env);
        let mut signals: Map<u64, Signal> = Map::new(&env);
        let provider = sample_provider(&env);

        stake(&env, &mut stakes, &provider, DEFAULT_MINIMUM_STAKE).unwrap();

        // Missing slash
        let res = submit_signal(
            &env,
            &mut signals,
            &stakes,
            &provider,
            sdk_string(&env, "XLMUSDC"),
            Action::Buy,
            120_000_000,
            sdk_string(&env, "Bullish"),
        );
        assert_eq!(res, Err(Error::InvalidAssetPair));

        // Too short
        let res = submit_signal(
            &env,
            &mut signals,
            &stakes,
            &provider,
            sdk_string(&env, "X/US"),
            Action::Buy,
            120_000_000,
            sdk_string(&env, "Bullish"),
        );
        assert_eq!(res, Err(Error::InvalidAssetPair));

        // Too long
        let res = submit_signal(
            &env,
            &mut signals,
            &stakes,
            &provider,
            sdk_string(&env, "XLM/USDC_EXTRA_LONG_PAIR"),
            Action::Buy,
            120_000_000,
            sdk_string(&env, "Bullish"),
        );
        assert_eq!(res, Err(Error::InvalidAssetPair));
    }
}
