use anyhow::Result;
use sqlx::{sqlite::SqlitePoolOptions, SqlitePool};
use tracing::info;

/// Initialize the SQLite database and run migrations.
pub async fn init_db(database_url: &str) -> Result<SqlitePool> {
    let pool = SqlitePoolOptions::new()
        .max_connections(5)
        .connect(database_url)
        .await?;

    // Create tables
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS tokens (
            id          TEXT PRIMARY KEY,
            telegram_id INTEGER NOT NULL,
            jwt         TEXT NOT NULL,
            max_bookings INTEGER NOT NULL DEFAULT 1,
            used_bookings INTEGER NOT NULL DEFAULT 0,
            created_at  TEXT NOT NULL DEFAULT (datetime('now')),
            expires_at  TEXT NOT NULL,
            consumed_at TEXT,
            payment_ref TEXT,
            status      TEXT NOT NULL DEFAULT 'ACTIVE'
        );

        CREATE INDEX IF NOT EXISTS idx_tokens_telegram ON tokens(telegram_id);
        CREATE INDEX IF NOT EXISTS idx_tokens_status ON tokens(status);
        "#,
    )
    .execute(&pool)
    .await?;

    info!("Database tables initialized");
    Ok(pool)
}

/// Store a newly generated token.
pub async fn store_token(
    pool: &SqlitePool,
    token_id: &str,
    telegram_id: i64,
    jwt: &str,
    expires_at: &str,
    payment_ref: Option<&str>,
) -> Result<()> {
    sqlx::query(
        r#"
        INSERT INTO tokens (id, telegram_id, jwt, expires_at, payment_ref)
        VALUES (?, ?, ?, ?, ?)
        "#,
    )
    .bind(token_id)
    .bind(telegram_id)
    .bind(jwt)
    .bind(expires_at)
    .bind(payment_ref)
    .execute(pool)
    .await?;

    Ok(())
}

/// Check if a token is valid and return remaining bookings.
pub async fn check_token(pool: &SqlitePool, token_id: &str) -> Result<Option<TokenRecord>> {
    let record = sqlx::query_as::<_, TokenRecord>(
        r#"
        SELECT id, telegram_id, max_bookings, used_bookings, expires_at, status
        FROM tokens
        WHERE id = ?
        "#,
    )
    .bind(token_id)
    .fetch_optional(pool)
    .await?;

    Ok(record)
}

/// Mark a token as consumed (booking used).
pub async fn mark_consumed(pool: &SqlitePool, token_id: &str) -> Result<()> {
    sqlx::query(
        r#"
        UPDATE tokens
        SET used_bookings = used_bookings + 1,
            consumed_at = datetime('now'),
            status = CASE
                WHEN used_bookings + 1 >= max_bookings THEN 'CONSUMED'
                ELSE status
            END
        WHERE id = ? AND status = 'ACTIVE'
        "#,
    )
    .bind(token_id)
    .execute(pool)
    .await?;

    Ok(())
}

/// Get all active tokens for a Telegram user.
pub async fn get_user_tokens(pool: &SqlitePool, telegram_id: i64) -> Result<Vec<TokenRecord>> {
    let records = sqlx::query_as::<_, TokenRecord>(
        r#"
        SELECT id, telegram_id, max_bookings, used_bookings, expires_at, status
        FROM tokens
        WHERE telegram_id = ? AND status = 'ACTIVE'
        ORDER BY created_at DESC
        "#,
    )
    .bind(telegram_id)
    .fetch_all(pool)
    .await?;

    Ok(records)
}

#[derive(Debug, sqlx::FromRow)]
pub struct TokenRecord {
    pub id: String,
    pub telegram_id: i64,
    pub max_bookings: i32,
    pub used_bookings: i32,
    pub expires_at: String,
    pub status: String,
}
