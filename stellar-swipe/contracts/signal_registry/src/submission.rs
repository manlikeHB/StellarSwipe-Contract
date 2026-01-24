#![allow(dead_code)]
use soroban_sdk::{contracttype, Address, Env, Map, String};
use crate::stake::{can_submit_signal, StakeInfo, DEFAULT_MINIMUM_STAKE};

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
    // 1️⃣ Verify provider stake
    can_submit_signal(provider_stakes, provider).map_err(|_| Error::NoStake)?;
    let stake_info = provider_stakes.get(provider.clone()).unwrap();
    if stake_info.amount < DEFAULT_MINIMUM_STAKE {
        return Err(Error::BelowMinimumStake);
    }

    // 2️⃣ Validate asset pair
    let asset_bytes = asset_pair.to_bytes();
    let has_slash = asset_bytes.iter().any(|b| b == b'/');
    let len = asset_bytes.len();
    if !has_slash || len < 3 || len > 20 {
        return Err(Error::InvalidAssetPair);
    }

    // 3️⃣ Validate price
    if price <= 0 {
        return Err(Error::InvalidPrice);
    }

    // 4️⃣ Validate rationale
    let rationale_len = rationale.to_bytes().len();
    if rationale_len == 0 || rationale_len > 500 {
        return Err(Error::EmptyRationale);
    }

    // 5️⃣ Check for duplicate signals in the last 1 hour
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

    // 6️⃣ Generate signal ID
    let next_id = storage.len() as u64 + 1;

    // 7️⃣ Set expiry (24 hours default)
    let expiry = now + 86400;

    // 8️⃣ Store the signal
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

    // 9️⃣ Event placeholder (optional)
    // env.events().publish("SignalSubmitted", (provider, signal.asset_pair.clone(), action, price, signal.rationale.clone(), expiry));

    Ok(next_id)
}

#[cfg(test)]
mod tests {
    use super::*;
    use soroban_sdk::{testutils::Address as TestAddress, Env, Map, String as SdkString};
    use crate::stake::{stake, StakeInfo, DEFAULT_MINIMUM_STAKE, can_submit_signal};

    fn setup_env() -> Env {
        Env::default()
    }

    fn sample_provider(env: &Env) -> Address {
        <Address as TestAddress>::generate(env)
    }

    fn sdk_string(s: &str) -> SdkString {
        SdkString::from_slice(&Env::default(), s)
    }

    #[test]
    fn test_submit_signal_success() {
        let env = setup_env();
        let mut stakes: Map<Address, StakeInfo> = Map::new(&env);
        let mut signals: Map<u64, Signal> = Map::new(&env);

        let provider = sample_provider(&env);

        // Stake enough
        stake(&env, &mut stakes, &provider, DEFAULT_MINIMUM_STAKE).unwrap();

        let signal_id = submit_signal(
            &env,
            &mut signals,
            &stakes,
            &provider,
            sdk_string("XLM/USDC"),
            Action::Buy,
            120_000_000,
            sdk_string("Bullish on XLM"),
        )
        .unwrap();

        assert_eq!(signal_id, 1);
        let stored = signals.get(signal_id).unwrap();
        assert_eq!(stored.provider, provider);
        assert_eq!(stored.asset_pair.to_bytes(), sdk_string("XLM/USDC").to_bytes());
        assert_eq!(stored.action, Action::Buy);
        assert_eq!(stored.price, 120_000_000);
    }

    #[test]
    fn test_submit_signal_no_stake() {
        let env = setup_env();
        let mut stakes: Map<Address, StakeInfo> = Map::new(&env);
        let mut signals: Map<u64, Signal> = Map::new(&env);
        let provider = sample_provider(&env);

        let res = submit_signal(
            &env,
            &mut signals,
            &stakes,
            &provider,
            sdk_string("XLM/USDC"),
            Action::Buy,
            120_000_000,
            sdk_string("Bullish on XLM"),
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
            sdk_string("XLM/USDC"),
            Action::Buy,
            0,
            sdk_string("Bullish on XLM"),
        );

        assert_eq!(res, Err(Error::InvalidPrice));
    }

    #[test]
    fn test_submit_signal_duplicate() {
        let env = setup_env();
        let mut stakes: Map<Address, StakeInfo> = Map::new(&env);
        let mut signals: Map<u64, Signal> = Map::new(&env);
        let provider = sample_provider(&env);

        stake(&env, &mut stakes, &provider, DEFAULT_MINIMUM_STAKE).unwrap();

        // First signal
        let _ = submit_signal(
            &env,
            &mut signals,
            &stakes,
            &provider,
            sdk_string("XLM/USDC"),
            Action::Buy,
            120_000_000,
            sdk_string("Bullish"),
        )
        .unwrap();

        // Duplicate within 1 hour
        let res = submit_signal(
            &env,
            &mut signals,
            &stakes,
            &provider,
            sdk_string("XLM/USDC"),
            Action::Buy,
            120_000_000,
            sdk_string("Bullish"),
        );

        assert_eq!(res, Err(Error::DuplicateSignal));
    }
}
