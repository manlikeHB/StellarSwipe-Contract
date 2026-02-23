use soroban_sdk::{contracttype, Address, String, Symbol};

#[contracttype]
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SortOption {
    PerformanceDesc,
    RecencyDesc,
    VolumeDesc,
}

#[contracttype]
#[derive(Clone, Debug)]
pub struct SignalSummary {
    pub id: u64,
    pub provider: Address,
    pub asset_pair: String,
    pub action: SignalAction,
    pub price: i128,
    pub success_rate: u32,
    pub total_copies: u32,
    pub timestamp: u64,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum SignalStatus {
    Pending,
    Active,
    Executed,
    Expired,
    Successful, // Signal met success criteria (avg ROI > 2%)
    Failed,     // Signal met failure criteria (avg ROI < -5% or expired with no executions)
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
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
    // Performance tracking fields
    pub executions: u32,            // Number of trade executions for this signal
    pub successful_executions: u32, // Number of successful trade executions
    pub total_volume: i128,         // Cumulative volume across all executions
    pub total_roi: i128,            // Cumulative ROI in basis points (10000 = 100%)
}

#[contracttype]
#[derive(Clone, Debug, Default)]
pub struct ProviderPerformance {
    pub total_signals: u32,      // Total number of signals provided
    pub successful_signals: u32, // Signals marked as successful
    pub failed_signals: u32,     // Signals marked as failed
    pub total_copies: u64,       // Legacy field: total times signals were copied
    pub success_rate: u32,       // Success rate in basis points (10000 = 100%)
    pub avg_return: i128,        // Average return in basis points
    pub total_volume: i128,      // Cumulative volume across all signals
}

#[contracttype]
#[derive(Clone)]
pub enum FeeStorageKey {
    PlatformTreasury,
    ProviderTreasury,
    TreasuryBalances,
}

#[contracttype]
#[derive(Clone, Debug)]
pub struct FeeBreakdown {
    pub total_fee: i128,
    pub platform_fee: i128,
    pub provider_fee: i128,
    pub trade_amount_after_fee: i128,
}

#[contracttype]
#[derive(Clone, Debug, PartialEq)]
pub struct Asset {
    pub symbol: Symbol,
    pub contract: Address,
}

/// Record of a single trade execution for a signal
#[contracttype]
#[derive(Clone, Debug)]
pub struct TradeExecution {
    pub signal_id: u64,
    pub executor: Address,
    pub entry_price: i128,
    pub exit_price: i128,
    pub volume: i128,
    pub roi: i128, // ROI in basis points (10000 = 100%)
    pub timestamp: u64,
}

/// View struct for signal performance queries
#[contracttype]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SignalPerformanceView {
    pub signal_id: u64,
    pub executions: u32,
    pub total_volume: i128,
    pub average_roi: i128, // In basis points
    pub status: SignalStatus,
}

// Type alias for backward compatibility
#[allow(dead_code)]
pub type SignalStats = ProviderPerformance;

#[contracttype]
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ImportFormat {
    CSV,
    JSON,
    TradingView,
    TwitterParse,
}

#[contracttype]
#[derive(Clone, Debug)]
pub struct ImportRequest {
    pub format: ImportFormat,
    pub data: soroban_sdk::Bytes,
    pub provider: Address,
    pub validate_only: bool,
}

#[contracttype]
#[derive(Clone, Debug)]
pub struct ImportResultView {
    pub success_count: u32,
    pub error_count: u32,
    pub signal_ids: soroban_sdk::Vec<u64>,
}
