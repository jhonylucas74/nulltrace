//! Admin JWT authentication. Admin tokens include role: "admin" so they cannot be
//! confused with player tokens.

use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, Validation};
use serde::{Deserialize, Serialize};
use std::time::{SystemTime, UNIX_EPOCH};
use uuid::Uuid;

/// Admin JWT claims. Includes role: "admin" so admin RPCs can distinguish from player tokens.
#[derive(Debug, Serialize, Deserialize)]
pub struct AdminClaims {
    pub sub: String,      // Admin id (UUID)
    pub email: String,
    pub role: String,     // "admin"
    pub exp: i64,
    pub iat: i64,
    pub nbf: i64,
}

/// Generate an admin JWT token with 24-hour expiry.
pub fn generate_admin_token(
    admin_id: Uuid,
    email: &str,
    secret: &str,
) -> Result<String, jsonwebtoken::errors::Error> {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("Time went backwards")
        .as_secs() as i64;

    let expiry = now + (24 * 60 * 60); // 24 hours

    let claims = AdminClaims {
        sub: admin_id.to_string(),
        email: email.to_string(),
        role: "admin".to_string(),
        exp: expiry,
        iat: now,
        nbf: now,
    };

    encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(secret.as_ref()),
    )
}

/// Validate an admin JWT token. Returns AdminClaims only if role == "admin".
/// Player tokens (no role or role != "admin") are rejected.
pub fn validate_admin_token(
    token: &str,
    secret: &str,
) -> Result<AdminClaims, jsonwebtoken::errors::Error> {
    let validation = Validation::default();

    let token_data = decode::<AdminClaims>(
        token,
        &DecodingKey::from_secret(secret.as_ref()),
        &validation,
    )?;

    if token_data.claims.role != "admin" {
        return Err(jsonwebtoken::errors::ErrorKind::InvalidToken.into());
    }

    Ok(token_data.claims)
}
