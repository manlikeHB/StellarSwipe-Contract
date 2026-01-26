use soroban_sdk::contracterror;

#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum AutoTradeError {
    InvalidAmount = 1,
    SignalNotFound = 2,
    SignalExpired = 3,
    Unauthorized = 4,
    InsufficientBalance = 5,
    SdexError = 6,
}
