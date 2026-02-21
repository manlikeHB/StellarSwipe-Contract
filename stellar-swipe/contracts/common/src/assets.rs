//! Stellar asset pair validation.
//!
//! Supports native (XLM), issued assets (code + issuer). Format: "ASSET1:ISSUER1/ASSET2:ISSUER2"
//! or "XLM/ASSET2:ISSUER2". All Stellar assets use 7 decimal precision.

#![allow(clippy::manual_range_contains)]

use soroban_sdk::{contracttype, Address, Bytes, Env, String};

/// Native XLM asset code
pub const NATIVE_ASSET_CODE: &[u8] = b"XLM";

/// Min/max asset code length (Stellar spec)
const ASSET_CODE_MIN_LEN: u32 = 1;
const ASSET_CODE_MAX_LEN: u32 = 12;

/// Stellar account ID length (G... format)
const STELLAR_ACCOUNT_ID_LEN: u32 = 56;

#[contracttype]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AssetPairError {
    InvalidFormat,
    InvalidAssetCode,
    InvalidIssuer,
    SameAssets,
}

/// Represents a Stellar asset (native or issued)
#[contracttype]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Asset {
    /// Asset code (1-12 alphanumeric). "XLM" for native.
    pub code: String,
    /// Issuer address (G... format). None for native XLM.
    pub issuer: Option<Address>,
}

/// Base/quote asset pair for trading
#[contracttype]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AssetPair {
    pub base: Asset,
    pub quote: Asset,
}

/// Check if byte is alphanumeric
#[inline]
fn is_alnum(b: u8) -> bool {
    (b >= b'A' && b <= b'Z') || (b >= b'a' && b <= b'z') || (b >= b'0' && b <= b'9')
}

/// Check if byte is valid Stellar base32 (for issuer)
#[inline]
fn is_base32(b: u8) -> bool {
    (b >= b'A' && b <= b'Z') || (b >= b'2' && b <= b'7')
}

/// Validate asset code bytes: 1-12 alphanumeric
fn validate_asset_code_bytes(bytes: &Bytes, start: u32, end: u32) -> bool {
    let len = end.saturating_sub(start);
    if len < ASSET_CODE_MIN_LEN || len > ASSET_CODE_MAX_LEN {
        return false;
    }
    for i in start..end {
        if !is_alnum(bytes.get(i).unwrap()) {
            return false;
        }
    }
    true
}

/// Validate issuer bytes: G followed by 55 base32 chars (56 total)
fn validate_issuer_bytes(bytes: &Bytes, start: u32, end: u32) -> bool {
    let len = end.saturating_sub(start);
    if len != STELLAR_ACCOUNT_ID_LEN {
        return false;
    }
    if bytes.get(start).unwrap() != b'G' {
        return false;
    }
    for i in (start + 1)..end {
        if !is_base32(bytes.get(i).unwrap()) {
            return false;
        }
    }
    true
}

/// Check if slice equals "XLM"
fn is_native_xlm(bytes: &Bytes, start: u32, end: u32) -> bool {
    if end.saturating_sub(start) != 3 {
        return false;
    }
    bytes.get(start).unwrap() == b'X'
        && bytes.get(start + 1).unwrap() == b'L'
        && bytes.get(start + 2).unwrap() == b'M'
}

/// Validate a single asset part: "XLM" or "CODE:ISSUER"
fn validate_asset_part(bytes: &Bytes, start: u32, end: u32) -> Result<(), AssetPairError> {
    if start >= end {
        return Err(AssetPairError::InvalidFormat);
    }

    let mut colon_at = None;
    for i in start..end {
        if bytes.get(i).unwrap() == b':' {
            colon_at = Some(i);
            break;
        }
    }

    match colon_at {
        None => {
            if is_native_xlm(bytes, start, end) || validate_asset_code_bytes(bytes, start, end) {
                Ok(())
            } else {
                Err(AssetPairError::InvalidAssetCode)
            }
        }
        Some(colon_at) => {
            if !validate_asset_code_bytes(bytes, start, colon_at) {
                return Err(AssetPairError::InvalidAssetCode);
            }
            if !validate_issuer_bytes(bytes, colon_at + 1, end) {
                return Err(AssetPairError::InvalidIssuer);
            }
            Ok(())
        }
    }
}

/// Check if two byte ranges are equal
fn ranges_equal(bytes: &Bytes, a_start: u32, a_end: u32, b_start: u32, b_end: u32) -> bool {
    let a_len = a_end.saturating_sub(a_start);
    let b_len = b_end.saturating_sub(b_start);
    if a_len != b_len {
        return false;
    }
    for i in 0..a_len {
        if bytes.get(a_start + i).unwrap() != bytes.get(b_start + i).unwrap() {
            return false;
        }
    }
    true
}

/// Validate asset pair string format.
///
/// Format: "BASE/QUOTE" where each is "XLM" or "CODE:ISSUER".
/// - Asset codes: 1-12 alphanumeric
/// - Issuer: 56 chars, G... format
/// - Base and quote must differ
pub fn validate_asset_pair(_env: &Env, asset_pair: &String) -> Result<(), AssetPairError> {
    let bytes = asset_pair.clone().to_bytes();

    let mut slash_at = None;
    for i in 0..bytes.len() {
        if bytes.get(i).unwrap() == b'/' {
            if slash_at.is_some() {
                return Err(AssetPairError::InvalidFormat);
            }
            slash_at = Some(i);
        }
    }

    let slash_at = slash_at.ok_or(AssetPairError::InvalidFormat)?;
    let len = bytes.len();

    if slash_at == 0 || slash_at >= len - 1 {
        return Err(AssetPairError::InvalidFormat);
    }

    validate_asset_part(&bytes, 0, slash_at)?;
    validate_asset_part(&bytes, slash_at + 1, len)?;

    if ranges_equal(&bytes, 0, slash_at, slash_at + 1, len) {
        return Err(AssetPairError::SameAssets);
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn s(env: &Env, x: &str) -> String {
        String::from_str(env, x)
    }

    #[test]
    fn test_xlm_usdc_shorthand_valid() {
        let env = Env::default();
        let pair = s(&env, "XLM/USDC");
        assert!(validate_asset_pair(&env, &pair).is_ok());
    }

    #[test]
    fn test_xlm_usdc_with_issuer_valid() {
        let env = Env::default();
        let pair = s(&env, "XLM/USDC:GDUKMGUGDZQK6YHYA5Z6AY2G4XDSZPSZ3SW5UN3ARVMO6QSRDWP5YLEX");
        assert!(validate_asset_pair(&env, &pair).is_ok());
    }

    #[test]
    fn test_custom_usdc_with_valid_issuers() {
        let env = Env::default();
        let pair = s(&env, "CUSTOM:GDUKMGUGDZQK6YHYA5Z6AY2G4XDSZPSZ3SW5UN3ARVMO6QSRDWP5YLEX/USDC:GAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAF");
        assert!(validate_asset_pair(&env, &pair).is_ok());
    }

    #[test]
    fn test_invalid_format_no_slash() {
        let env = Env::default();
        assert_eq!(
            validate_asset_pair(&env, &s(&env, "XLMUSDC")),
            Err(AssetPairError::InvalidFormat)
        );
    }

    #[test]
    fn test_invalid_format_empty_base() {
        let env = Env::default();
        let pair = s(&env, "/USDC:GDUKMGUGDZQK6YHYA5Z6AY2G4XDSZPSZ3SW5UN3ARVMO6QSRDWP5YLEX");
        assert_eq!(validate_asset_pair(&env, &pair), Err(AssetPairError::InvalidFormat));
    }

    #[test]
    fn test_same_assets_rejected() {
        let env = Env::default();
        assert_eq!(
            validate_asset_pair(&env, &s(&env, "XLM/XLM")),
            Err(AssetPairError::SameAssets)
        );
    }

    #[test]
    fn test_invalid_asset_code_special_chars() {
        let env = Env::default();
        let pair = s(&env, "XLM/USD!");
        assert!(validate_asset_pair(&env, &pair).is_err());
    }

    #[test]
    fn test_invalid_issuer_format() {
        let env = Env::default();
        let pair = s(&env, "XLM/USDC:INVALID");
        assert_eq!(
            validate_asset_pair(&env, &pair),
            Err(AssetPairError::InvalidIssuer)
        );
    }

    #[test]
    fn test_xlm_btc_valid() {
        let env = Env::default();
        let pair = s(&env, "XLM/BTC:GDUKMGUGDZQK6YHYA5Z6AY2G4XDSZPSZ3SW5UN3ARVMO6QSRDWP5YLEX");
        assert!(validate_asset_pair(&env, &pair).is_ok());
    }
}
