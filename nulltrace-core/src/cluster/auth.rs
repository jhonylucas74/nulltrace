use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, Validation};
use serde::{Deserialize, Serialize};
use std::time::{SystemTime, UNIX_EPOCH};
use uuid::Uuid;

#[derive(Debug, Serialize, Deserialize)]
pub struct Claims {
    pub sub: String,     // Subject: player_id (UUID)
    pub username: String, // Player username
    pub exp: i64,        // Expiration time (Unix timestamp)
    pub iat: i64,        // Issued at (Unix timestamp)
    pub nbf: i64,        // Not before (Unix timestamp)
}

/// Get JWT secret from environment variable or use development default
pub fn get_jwt_secret() -> String {
    std::env::var("JWT_SECRET").unwrap_or_else(|_| {
        // Only warn once and only in production mode
        use std::sync::atomic::{AtomicBool, Ordering};
        static WARNED: AtomicBool = AtomicBool::new(false);
        if !WARNED.swap(true, Ordering::Relaxed) {
            if std::env::var("PRODUCTION").is_ok() || std::env::var("PROD").is_ok() {
                eprintln!("[WARNING] JWT_SECRET not set, using development default");
                eprintln!("[WARNING] For production, set JWT_SECRET environment variable");
            }
        }
        "dev_secret_change_in_production_use_openssl_rand_base64_32".to_string()
    })
}

/// Generate a JWT token with 24-hour expiry
pub fn generate_token(
    player_id: Uuid,
    username: &str,
    secret: &str,
) -> Result<String, jsonwebtoken::errors::Error> {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("Time went backwards")
        .as_secs() as i64;

    let expiry = now + (24 * 60 * 60); // 24 hours

    let claims = Claims {
        sub: player_id.to_string(),
        username: username.to_string(),
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

/// Validate a JWT token and return the claims
pub fn validate_token(
    token: &str,
    secret: &str,
) -> Result<Claims, jsonwebtoken::errors::Error> {
    let validation = Validation::default();

    let token_data = decode::<Claims>(
        token,
        &DecodingKey::from_secret(secret.as_ref()),
        &validation,
    )?;

    Ok(token_data.claims)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_and_validate_token() {
        let secret = "test_secret_key";
        let player_id = Uuid::new_v4();
        let username = "test_user";

        // Generate token
        let token = generate_token(player_id, username, secret).expect("Failed to generate token");

        // Validate token
        let claims = validate_token(&token, secret).expect("Failed to validate token");

        assert_eq!(claims.sub, player_id.to_string());
        assert_eq!(claims.username, username);
        assert!(claims.exp > claims.iat);
    }

    #[test]
    fn test_invalid_secret_fails_validation() {
        let player_id = Uuid::new_v4();
        let username = "test_user";

        let token = generate_token(player_id, username, "secret1")
            .expect("Failed to generate token");

        let result = validate_token(&token, "wrong_secret");
        assert!(result.is_err());
    }

    #[test]
    fn test_expired_token_fails_validation() {
        let secret = "test_secret_key";
        let player_id = Uuid::new_v4();

        // Manually create an expired token
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64;

        let claims = Claims {
            sub: player_id.to_string(),
            username: "test_user".to_string(),
            exp: now - 3600, // Expired 1 hour ago
            iat: now - 7200,
            nbf: now - 7200,
        };

        let token = encode(
            &Header::default(),
            &claims,
            &EncodingKey::from_secret(secret.as_ref()),
        )
        .expect("Failed to encode token");

        let result = validate_token(&token, secret);
        assert!(result.is_err());
    }
}
