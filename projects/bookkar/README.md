# 🚂 Tatkal Booking Automation System

A Rust-based IRCTC Tatkal ticket booking automation tool that runs on the user's device.

## Architecture

```
┌─────────────────────────────┐     ┌─────────────────────────┐
│  User's Device              │     │  Your Server (minimal)  │
│                             │     │                         │
│  tatkal-client              │     │  tatkal-license-server  │
│  ├── Chrome (visible)  ────────>  │  ├── Telegram Bot       │
│  ├── Auto-fill forms        │     │  ├── REST API           │
│  ├── Countdown timer        │     │  └── SQLite DB          │
│  └── Stealth patches        │     │                         │
└─────────────────────────────┘     └─────────────────────────┘
         │                                     │
         │ Validates token ───────────────────>│
         │ User sees Chrome, enters OTPs       │
         │ User approves UPI payment           │
```

## Prerequisites

- **Rust** 1.80+ (`curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh`)
- **Google Chrome** or **Chromium** installed on the user's machine
- **Telegram Bot Token** (get from [@BotFather](https://t.me/BotFather))

## Quick Start

### 1. Clone and Build

```bash
git clone <repo>
cd tatkal-system
cargo build --release
```

### 2. Set Up the License Server

```bash
cp .env.example .env
# Edit .env with your Telegram bot token and JWT secret

# Run the license server
cargo run --release --bin tatkal-license
```

### 3. Get a Token

Open your Telegram bot and type:
- `/devtoken` — free test token (dev mode)
- `/buy` — paid token (₹25)

### 4. Run the Client

```bash
# On the user's machine (with Chrome installed)
./target/release/tatkal
```

Follow the interactive prompts to enter:
- License token
- IRCTC credentials
- Journey details (from, to, date, class)
- Passenger details
- UPI ID for payment

## Project Structure

```
crates/
├── tatkal-common/          # Shared types (Passenger, BookingConfig, Token)
├── tatkal-client/          # Desktop automation client
│   └── src/
│       ├── browser/        # Chrome launcher + stealth patches
│       ├── irctc/          # Login, search, booking, payment flows
│       ├── timer/          # Millisecond-precision Tatkal countdown
│       ├── config.rs       # TOML config save/load
│       └── license.rs      # Token validation
└── tatkal-license-server/  # Telegram bot + REST API
    └── src/
        ├── bot.rs          # Telegram command handlers
        ├── token.rs        # JWT generation/validation
        └── db.rs           # SQLite operations
```

## How It Works

1. **Pre-Tatkal**: App logs into IRCTC, pre-fills the search form
2. **Countdown**: Spin-lock timer waits for exact Tatkal opening (10:00 AM AC / 11:00 AM Non-AC)
3. **Fire**: At T-0, instantly clicks search, selects train, fills passengers
4. **OTP**: User enters Aadhaar OTP manually in the visible Chrome window
5. **Payment**: App fills UPI ID, user approves on their phone
6. **Done**: PNR confirmed!

## Important Notes

- **Selectors**: The CSS selectors in `irctc/selectors.rs` are best-effort guesses. They MUST be verified against the actual IRCTC DOM before use. Run the tool and inspect the Chrome DevTools to fix any mismatches.
- **For educational and testing purposes only.**

## License

MIT
