#![allow(dead_code)]

use uuid::Uuid;

#[derive(Debug)]
pub enum WalletError {
    Db(sqlx::Error),
    InsufficientBalance,
    InvalidCurrency,
    CardLimitExceeded,
    /// Conversion result rounded to zero (amount too small for the rate).
    ConvertedAmountTooSmall,
    /// Recipient account/key not found (e.g. USD transfer to non-existent Fkebank key).
    RecipientNotFound,
}

impl From<sqlx::Error> for WalletError {
    fn from(e: sqlx::Error) -> Self {
        WalletError::Db(e)
    }
}

impl std::fmt::Display for WalletError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            WalletError::Db(e) => write!(f, "Database error: {}", e),
            WalletError::InsufficientBalance => write!(f, "Insufficient balance"),
            WalletError::InvalidCurrency => write!(f, "Invalid currency"),
            WalletError::CardLimitExceeded => write!(f, "Card credit limit exceeded"),
            WalletError::ConvertedAmountTooSmall => write!(f, "Converted amount is zero or too small"),
            WalletError::RecipientNotFound => write!(f, "Recipient not found"),
        }
    }
}

/// Fkebank PIX-style key: fkebank-{32 hex chars}
pub fn generate_fkebank_key() -> String {
    format!("fkebank-{}", Uuid::new_v4().simple())
}

/// ETH-style address: 0x{40 lowercase hex chars}
pub fn generate_eth_address() -> String {
    let hex1 = format!("{}", Uuid::new_v4().simple());
    let hex2 = format!("{}", Uuid::new_v4().simple());
    let combined = format!("{}{}", hex1, hex2);
    format!("0x{}", &combined[..40])
}

/// BTC bech32-style address: bc1q{38 chars}
pub fn generate_btc_address() -> String {
    const CHARSET: &[u8] = b"qpzry9x8gf2tvdw0s3jn54khce6mua7l";
    let bytes: Vec<u8> = Uuid::new_v4()
        .as_bytes()
        .iter()
        .chain(Uuid::new_v4().as_bytes().iter())
        .chain(Uuid::new_v4().as_bytes().iter())
        .copied()
        .collect();
    let chars: String = bytes[..38]
        .iter()
        .map(|b| CHARSET[(*b as usize) % CHARSET.len()] as char)
        .collect();
    format!("bc1q{}", chars)
}

/// SOL base58-style address: 44 chars
pub fn generate_sol_address() -> String {
    const BASE58: &[u8] = b"123456789ABCDEFGHJKLMNPQRSTUVWXYZabcdefghijkmnopqrstuvwxyz";
    let bytes: Vec<u8> = Uuid::new_v4()
        .as_bytes()
        .iter()
        .chain(Uuid::new_v4().as_bytes().iter())
        .chain(Uuid::new_v4().as_bytes().iter())
        .copied()
        .collect();
    bytes[..44]
        .iter()
        .map(|b| BASE58[(*b as usize) % BASE58.len()] as char)
        .collect()
}

pub fn generate_key_for_currency(currency: &str) -> String {
    match currency {
        "USD" => generate_fkebank_key(),
        "BTC" => generate_btc_address(),
        "ETH" => generate_eth_address(),
        "SOL" => generate_sol_address(),
        _ => format!("key-{}", Uuid::new_v4()),
    }
}

pub fn usd_factor_per_cent(currency: &str) -> Option<f64> {
    match currency {
        "USD" => Some(1.0),
        "BTC" => Some(250.0),
        "ETH" => Some(20.0),
        "SOL" => Some(1.0),
        _ => None,
    }
}

/// Converts amount DB-cents from `from` currency into DB-cents of `to` currency.
pub fn convert_amount(amount: i64, from: &str, to: &str) -> Option<i64> {
    let in_factor = usd_factor_per_cent(from)?;
    let out_factor = usd_factor_per_cent(to)?;
    if out_factor == 0.0 {
        return None;
    }
    Some((amount as f64 * in_factor / out_factor).floor() as i64)
}
