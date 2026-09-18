use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// JWT claims embedded in license tokens
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenClaims {
    /// Subject — Telegram user ID
    pub sub: String,
    /// Unique token identifier
    pub token_id: Uuid,
    /// Maximum number of bookings allowed
    pub max_bookings: u8,
    /// Expiry timestamp (Unix epoch seconds)
    pub exp: u64,
    /// Issued at timestamp (Unix epoch seconds)
    pub iat: u64,
}

/// Response from the license server on token validation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenResponse {
    pub valid: bool,
    pub bookings_remaining: u8,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}
