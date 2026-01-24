use soroban_sdk::{contracttype, Address, String};

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum SignalStatus {
    Pending,
    Active,
    Executed,
    Expired,
}

#[contracttype]
#[derive(Clone, Debug)]
pub enum SignalAction {
    Buy,
    Sell,
}

#[contracttype]
#[derive(Clone, Debug)]
pub struct Signal {
    pub id: u64,
    pub provider: Address,
    pub asset_pair: String, // e.g. "XLM/USDC"
    pub action: SignalAction,
    pub price: i128,
    pub rationale: String,
    pub timestamp: u64,
    pub expiry: u64,
    pub status: SignalStatus,
}

#[contracttype]
#[derive(Clone, Debug, Default)]
pub struct SignalStats {
    pub total_copies: u64,
    pub success_rate: u32, // basis points (0–10_000)
    pub avg_return: i128,
    pub total_volume: i128,
}
