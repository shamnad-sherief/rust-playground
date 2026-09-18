use std::sync::Arc;

use anyhow::Result;
use teloxide::prelude::*;
use teloxide::types::ParseMode;
use teloxide::utils::command::BotCommands;
use tracing::info;

use super::AppState;

#[derive(BotCommands, Clone)]
#[command(rename_rule = "lowercase", description = "Available commands:")]
enum Command {
    #[command(description = "Get started")]
    Start,
    #[command(description = "Purchase a booking token (₹25)")]
    Buy,
    #[command(description = "Check your active tokens")]
    Status,
    #[command(description = "Show help")]
    Help,
    #[command(description = "Generate a free test token (dev only)")]
    DevToken,
}

pub async fn run_bot(bot_token: String, state: Arc<AppState>) -> Result<()> {
    let bot = Bot::new(bot_token);

    info!("Telegram bot started. Listening for commands...");

    Command::repl(bot, move |bot: Bot, msg: Message, cmd: Command| {
        let state = state.clone();
        async move {
            match cmd {
                Command::Start => handle_start(bot, msg).await,
                Command::Buy => handle_buy(bot, msg, state).await,
                Command::Status => handle_status(bot, msg, state).await,
                Command::Help => handle_help(bot, msg).await,
                Command::DevToken => handle_dev_token(bot, msg, state).await,
            }
        }
    })
    .await;

    Ok(())
}

async fn handle_start(bot: Bot, msg: Message) -> ResponseResult<()> {
    let text = r#"🚂 *Welcome to BookKar\!*

I help you get a license token for the Tatkal booking automation tool\.

*How it works:*
1\. Pay ₹25 using `/buy`
2\. Get a license token
3\. Enter the token in the Tatkal client app
4\. App books your Tatkal ticket automatically\!

Use `/help` to see all commands\."#;

    bot.send_message(msg.chat.id, text)
        .parse_mode(ParseMode::MarkdownV2)
        .await?;
    Ok(())
}

async fn handle_buy(bot: Bot, msg: Message, state: Arc<AppState>) -> ResponseResult<()> {
    let upi_id = std::env::var("PAYMENT_UPI_ID")
        .unwrap_or_else(|_| "tatkal@upi".to_string());

    let text = format!(
        "💳 *Payment Instructions*\n\n\
         Send ₹25 to:\n\
         UPI ID: `{}`\n\n\
         After payment, send me the UTR/transaction reference number \
         and I'll generate your token\\.\n\n\
         _For testing: use /devtoken to get a free token\\._",
        upi_id,
    );

    bot.send_message(msg.chat.id, text)
        .parse_mode(ParseMode::MarkdownV2)
        .await?;
    Ok(())
}

async fn handle_status(bot: Bot, msg: Message, state: Arc<AppState>) -> ResponseResult<()> {
    let telegram_id = msg.from.as_ref().map(|u| u.id.0 as i64).unwrap_or(0);

    match super::db::get_user_tokens(&state.db, telegram_id).await {
        Ok(tokens) => {
            if tokens.is_empty() {
                bot.send_message(msg.chat.id, "You don't have any active tokens. Use /buy to get one!")
                    .await?;
            } else {
                let mut text = "🎫 *Your Active Tokens:*\n\n".to_string();
                for t in tokens {
                    let remaining = t.max_bookings - t.used_bookings;
                    text.push_str(&format!(
                        "Token: `{}...`\nBookings remaining: {}\nExpires: {}\n\n",
                        &t.id[..8],
                        remaining,
                        t.expires_at,
                    ));
                }
                bot.send_message(msg.chat.id, text)
                    .parse_mode(ParseMode::MarkdownV2)
                    .await?;
            }
        }
        Err(e) => {
            bot.send_message(msg.chat.id, format!("Error checking tokens: {}", e))
                .await?;
        }
    }

    Ok(())
}

async fn handle_help(bot: Bot, msg: Message) -> ResponseResult<()> {
    bot.send_message(msg.chat.id, Command::descriptions().to_string())
        .await?;
    Ok(())
}

/// Dev-only: Generate a free test token for development.
async fn handle_dev_token(bot: Bot, msg: Message, state: Arc<AppState>) -> ResponseResult<()> {
    let telegram_id = msg.from.as_ref().map(|u| u.id.0 as i64).unwrap_or(0);

    match super::token::generate(telegram_id, &state.jwt_secret, &state.db, Some("DEV_FREE")).await
    {
        Ok(token) => {
            let text = format!(
                "🎫 *Test Token Generated\\!*\n\n\
                 ```\n{}\n```\n\n\
                 Copy this token and paste it into the Tatkal client app\\.\n\
                 Valid for 24 hours, 1 booking\\.",
                token,
            );
            bot.send_message(msg.chat.id, text)
                .parse_mode(ParseMode::MarkdownV2)
                .await?;
        }
        Err(e) => {
            bot.send_message(msg.chat.id, format!("Error generating token: {}", e))
                .await?;
        }
    }

    Ok(())
}
