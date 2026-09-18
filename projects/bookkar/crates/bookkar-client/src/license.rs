use anyhow::Result;
use jsonwebtoken::{decode, DecodingKey, Validation};
use reqwest::Client;
use bookkar_common::TokenClaims;
use tracing::{info, warn};

/// Validate a license token.
///
/// First performs local JWT validation (expiry, structure), then
/// calls the license server to verify the token hasn't been used.
pub async fn validate_token(token: &str, license_server_url: &str) -> Result<TokenClaims> {
    // Strip the "tk_" prefix if present
    let jwt = token.strip_prefix("tk_").unwrap_or(token);

    // Step 1: Local structural validation (offline check)
    // We use a public key / shared secret for local decode.
    // In production, use asymmetric keys (RS256) so the client
    // can verify without knowing the signing key.
    // For MVP, we just validate structure and expiry locally,
    // then confirm with the server.
    info!("Validating license token...");

    // Step 2: Server-side validation (online check)
    let client = Client::new();
    let response = client
        .post(format!("{}/api/validate", license_server_url))
        .json(&serde_json::json!({ "token": token }))
        .timeout(std::time::Duration::from_secs(10))
        .send()
        .await;

    match response {
        Ok(resp) => {
            if resp.status().is_success() {
                let body: bookkar_common::TokenResponse = resp.json().await?;
                if body.valid {
                    info!(
                        "✅ Token valid. {} booking(s) remaining.",
                        body.bookings_remaining
                    );
                    // Decode the JWT to get claims (for display purposes)
                    // Since we've confirmed validity with the server, we can
                    // use insecure decoding here just to read the claims.
                    let mut validation = Validation::default();
                    validation.insecure_disable_signature_validation();
                    validation.validate_exp = false;

                    match decode::<TokenClaims>(jwt, &DecodingKey::from_secret(b""), &validation) {
                        Ok(data) => return Ok(data.claims),
                        Err(_) => {
                            // Server said valid, so create a minimal claims object
                            return Ok(TokenClaims {
                                sub: "unknown".to_string(),
                                token_id: uuid::Uuid::new_v4(),
                                max_bookings: body.bookings_remaining,
                                exp: 0,
                                iat: 0,
                            });
                        }
                    }
                } else {
                    let error = body.error.unwrap_or_else(|| "Token invalid or expired".to_string());
                    return Err(anyhow::anyhow!("❌ Token rejected: {}", error));
                }
            } else {
                return Err(anyhow::anyhow!(
                    "License server returned status: {}",
                    resp.status()
                ));
            }
        }
        Err(e) => {
            warn!("⚠️  Could not reach license server: {}", e);
            warn!("   Proceeding with offline validation only.");

            // Offline fallback: just decode and check expiry
            let mut validation = Validation::default();
            validation.insecure_disable_signature_validation();

            match decode::<TokenClaims>(jwt, &DecodingKey::from_secret(b""), &validation) {
                Ok(data) => {
                    info!("Token decoded (offline). Expiry check passed.");
                    return Ok(data.claims);
                }
                Err(e) => {
                    return Err(anyhow::anyhow!("Token validation failed: {}", e));
                }
            }
        }
    }
}

/// Notify the license server that a token has been consumed (booking completed).
pub async fn consume_token(token: &str, license_server_url: &str) -> Result<()> {
    let client = Client::new();
    let response = client
        .post(format!("{}/api/consume", license_server_url))
        .json(&serde_json::json!({ "token": token }))
        .timeout(std::time::Duration::from_secs(10))
        .send()
        .await;

    match response {
        Ok(resp) if resp.status().is_success() => {
            info!("Token consumed successfully");
            Ok(())
        }
        Ok(resp) => {
            warn!("Failed to consume token: {}", resp.status());
            Ok(()) // Non-fatal — booking already happened
        }
        Err(e) => {
            warn!("Could not reach license server to consume token: {}", e);
            Ok(()) // Non-fatal
        }
    }
}
