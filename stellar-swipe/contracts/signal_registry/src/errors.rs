use soroban_sdk::contracterror;

#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[repr(u32)]
pub enum AdminError {
    Unauthorized = 1,
    AlreadyInitialized = 2,
    NotInitialized = 3,
    InvalidParameter = 4,
    TradingPaused = 5,
    PauseExpired = 6,
    InvalidFeeRate = 7,
    InvalidRiskParameter = 8,
    InsufficientSignatures = 9,
    DuplicateSigner = 10,
    InvalidAssetPair = 11,
    CannotFollowSelf = 12,
}

#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[repr(u32)]
pub enum FeeError {
    TradeTooSmall = 100,
    FeeRoundedToZero = 101,
    ArithmeticOverflow = 102,
    InvalidAmount = 103,
    InvalidProviderAddress = 104,
}

#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[repr(u32)]
pub enum SocialError {
    CannotFollowSelf = 50,
}

#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[repr(u32)]
pub enum PerformanceError {
    SignalNotFound = 200,
    InvalidPrice = 201,
    DivisionByZero = 202,
    InvalidVolume = 203,
    SignalExpired = 204,
    NoExecutions = 205,
}

#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[repr(u32)]
pub enum TemplateError {
    TemplateNotFound = 300,
    Unauthorized = 301,
    PrivateTemplate = 302,
    MissingVariable = 303,
    InvalidTemplate = 304,
    InvalidAction = 305,
    InvalidExpiry = 306,
}
