use anyhow::Result;
use chrono::{Duration, Utc};
use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, Validation};
use sqlx::SqlitePool;
use bookkar_common::{TokenClaims, TokenResponse};
use tracing::info;
use uuid::Uuid;

/// Generate a new license token for a Telegram user.
pub async fn generate(
    telegram_id: i64,
    jwt_secret: &str,
    pool: &SqlitePool,
    payment_ref: Option<&str>,
) -> Result<String> {
    let token_id = Uuid::new_v4();
    let now = Utc::now();
    let expires = now + Duration::hours(24);

    let claims = TokenClaims {
        sub: telegram_id.to_string(),
        token_id,
        max_bookings: 1,
        exp: expires.timestamp() as u64,
        iat: now.timestamp() as u64,
    };

    let jwt = encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(jwt_secret.as_bytes()),
    )?;

    let full_token = format!("tk_{}", jwt);

    // Store in database
    super::db::store_token(
        pool,
        &token_id.to_string(),
        telegram_id,
        &full_token,
        &expires.to_rfc3339(),
        payment_ref,
    )
    .await?;

    info!("Generated token {} for user {}", token_id, telegram_id);
    Ok(full_token)
}

/// Validate a token — check JWT signature, expiry, and database status.
pub async fn validate(token: &str, jwt_secret: &str, pool: &SqlitePool) -> Result<TokenResponse> {
    let jwt = token.strip_prefix("tk_").unwrap_or(token);

    // Decode and verify JWT
    let token_data = decode::<TokenClaims>(
        jwt,
        &DecodingKey::from_secret(jwt_secret.as_bytes()),
        &Validation::default(),
    )
    .map_err(|e| anyhow::anyhow!("Invalid token: {}", e))?;

    let claims = token_data.claims;
    let token_id = claims.token_id.to_string();

    // Check database status
    let record = super::db::check_token(pool, &token_id).await?;

    match record {
        Some(rec) => {
            if rec.status != "ACTIVE" {
                return Ok(TokenResponse {
                    valid: false,
                    bookings_remaining: 0,
                    error: Some("Token has already been used".to_string()),
                });
            }

            let remaining = (rec.max_bookings - rec.used_bookings).max(0) as u8;

            Ok(TokenResponse {
                valid: remaining > 0,
                bookings_remaining: remaining,
                error: None,
            })
        }
        None => Ok(TokenResponse {
            valid: false,
            bookings_remaining: 0,
            error: Some("Token not found".to_string()),
        }),
    }
}

/// Consume a token (mark one booking as used).
pub async fn consume(token: &str, jwt_secret: &str, pool: &SqlitePool) -> Result<()> {
    let jwt = token.strip_prefix("tk_").unwrap_or(token);

    let token_data = decode::<TokenClaims>(
        jwt,
        &DecodingKey::from_secret(jwt_secret.as_bytes()),
        &Validation::default(),
    )
    .map_err(|e| anyhow::anyhow!("Invalid token: {}", e))?;

    let token_id = token_data.claims.token_id.to_string();
    super::db::mark_consumed(pool, &token_id).await?;

    info!("Token {} consumed", token_id);
    Ok(())
}
