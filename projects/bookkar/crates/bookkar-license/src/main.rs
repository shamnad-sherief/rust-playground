mod bot;
mod db;
mod token;

use std::sync::Arc;

use anyhow::Result;
use axum::{
    extract::State,
    http::StatusCode,
    routing::post,
    Json, Router,
};
use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;
use tokio::sync::Mutex;
use tower_http::cors::CorsLayer;
use tracing::info;

use bookkar_common::TokenResponse;

/// Shared application state
pub struct AppState {
    pub db: SqlitePool,
    pub jwt_secret: String,
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter("bookkar_license=info,teloxide=warn")
        .with_target(false)
        .init();

    info!("Starting Tatkal License Server...");

    // Load configuration from environment
    let db_url = std::env::var("DATABASE_URL")
        .unwrap_or_else(|_| "sqlite:bookkar_licenses.db?mode=rwc".to_string());
    let jwt_secret = std::env::var("JWT_SECRET")
        .unwrap_or_else(|_| {
            tracing::warn!("JWT_SECRET not set! Using default (INSECURE for production)");
            "tatkal-dev-secret-change-in-production".to_string()
        });
    let bot_token = std::env::var("TELOXIDE_TOKEN")
        .expect("TELOXIDE_TOKEN environment variable must be set");
    let api_port: u16 = std::env::var("API_PORT")
        .unwrap_or_else(|_| "8080".to_string())
        .parse()?;

    // Initialize database
    let pool = db::init_db(&db_url).await?;
    info!("Database initialized");

    let state = Arc::new(AppState {
        db: pool.clone(),
        jwt_secret: jwt_secret.clone(),
    });

    // Start the REST API server (for token validation)
    let api_state = state.clone();
    let api = Router::new()
        .route("/api/validate", post(validate_token_handler))
        .route("/api/consume", post(consume_token_handler))
        .layer(CorsLayer::permissive())
        .with_state(api_state);

    let api_handle = tokio::spawn(async move {
        let listener = tokio::net::TcpListener::bind(format!("0.0.0.0:{}", api_port))
            .await
            .expect("Failed to bind API port");
        info!("REST API listening on port {}", api_port);
        axum::serve(listener, api).await.expect("API server failed");
    });

    // Start the Telegram bot
    info!("Starting Telegram bot...");
    bot::run_bot(bot_token, state).await?;

    api_handle.await?;
    Ok(())
}

// === REST API Handlers ===

#[derive(Deserialize)]
struct ValidateRequest {
    token: String,
}

async fn validate_token_handler(
    State(state): State<Arc<AppState>>,
    Json(req): Json<ValidateRequest>,
) -> (StatusCode, Json<TokenResponse>) {
    match token::validate(&req.token, &state.jwt_secret, &state.db).await {
        Ok(response) => (StatusCode::OK, Json(response)),
        Err(e) => (
            StatusCode::BAD_REQUEST,
            Json(TokenResponse {
                valid: false,
                bookings_remaining: 0,
                error: Some(e.to_string()),
            }),
        ),
    }
}

#[derive(Deserialize)]
struct ConsumeRequest {
    token: String,
}

async fn consume_token_handler(
    State(state): State<Arc<AppState>>,
    Json(req): Json<ConsumeRequest>,
) -> (StatusCode, Json<serde_json::Value>) {
    match token::consume(&req.token, &state.jwt_secret, &state.db).await {
        Ok(_) => (
            StatusCode::OK,
            Json(serde_json::json!({ "consumed": true })),
        ),
        Err(e) => (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({ "consumed": false, "error": e.to_string() })),
        ),
    }
}
