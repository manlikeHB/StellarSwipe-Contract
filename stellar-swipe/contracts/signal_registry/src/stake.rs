use soroban_sdk::{contracttype, Env, Address, Map};

pub const DEFAULT_MINIMUM_STAKE: i128 = 100_000_000; // 100 XLM
pub const UNSTAKE_LOCK_PERIOD: u64 = 7 * 24 * 60 * 60; // 7 days in seconds

#[contracttype]
#[derive(Clone)]
pub struct StakeInfo {
    pub amount: i128,
    pub last_signal_time: u64,
    pub locked_until: u64,
}

#[derive(Debug)]
pub enum ContractError {
    InvalidStakeAmount,
    NoStakeFound,
    StakeLocked,
    InsufficientStake,
    BelowMinimumStake,
}

/// Stake XLM for a provider
pub fn stake(
    env: &Env,
    storage: &mut Map<Address, StakeInfo>,
    provider: &Address,
    amount: i128,
) -> Result<(), ContractError> {
    if amount <= 0 {
        return Err(ContractError::InvalidStakeAmount);
    }

    let mut info = storage.get(provider.clone()).unwrap_or(StakeInfo {
        amount: 0,
        last_signal_time: 0,
        locked_until: 0,
    });

    info.amount += amount;

    // Ensure minimum stake is satisfied
    if info.amount < DEFAULT_MINIMUM_STAKE {
        return Err(ContractError::BelowMinimumStake);
    }

    storage.set(provider.clone(), info);
    Ok(())
}

/// Unstake XLM (only after lock period)
pub fn unstake(
    env: &Env,
    storage: &mut Map<Address, StakeInfo>,
    provider: &Address,
) -> Result<i128, ContractError> {
    let mut info = storage.get(provider.clone()).ok_or(ContractError::NoStakeFound)?;

    let now = env.ledger().timestamp();

    if now < info.locked_until {
        return Err(ContractError::StakeLocked);
    }

    if info.amount <= 0 {
        return Err(ContractError::InsufficientStake);
    }

    let amount = info.amount;
    info.amount = 0;
    storage.set(provider.clone(), info);

    Ok(amount)
}

/// Record that a signal was submitted
/// Updates last_signal_time and locks stake for UNSTAKE_LOCK_PERIOD
pub fn record_signal(
    env: &Env,
    storage: &mut Map<Address, StakeInfo>,
    provider: &Address,
) -> Result<(), ContractError> {
    let mut info = storage.get(provider.clone()).ok_or(ContractError::NoStakeFound)?;
    let now = env.ledger().timestamp();

    info.last_signal_time = now;
    info.locked_until = now + UNSTAKE_LOCK_PERIOD;

    storage.set(provider.clone(), info);
    Ok(())
}

/// Check if a provider can submit a signal
pub fn can_submit_signal(
    storage: &Map<Address, StakeInfo>,
    provider: &Address,
) -> Result<(), ContractError> {
    let info = storage.get(provider.clone()).ok_or(ContractError::NoStakeFound)?;

    if info.amount < DEFAULT_MINIMUM_STAKE {
        return Err(ContracjhtError::BelowMinimumStake);
    }

    Ok(())
}
