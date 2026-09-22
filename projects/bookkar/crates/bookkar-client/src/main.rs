mod browser;
mod config;
mod irctc;
mod license;
mod timer;

use std::time::Duration;

use anyhow::Result;
use chrono::NaiveDate;
use console::style;
use dialoguer::{Input, Password, Select};
use tracing::info;

use bookkar_common::TrainClass;

const LICENSE_SERVER_URL: &str = "http://localhost:8080";
const VERSION: &str = env!("CARGO_PKG_VERSION");

#[tokio::main]
async fn main() -> Result<()> {
    // Load .env if present
    dotenvy::dotenv().ok();

    // Parse CLI flags and environment options for automated testing
    let args: Vec<String> = std::env::args().collect();
    let auto_mode = args.iter().any(|a| a == "--auto" || a == "-y" || a == "--yes")
        || std::env::var("AUTO_CONFIRM").map(|v| v == "1" || v.eq_ignore_ascii_case("true")).unwrap_or(false);

    // Initialize logging (silence internal chromiumoxide deserialization noise)
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_default_env()
                .add_directive("bookkar=info".parse()?)
                .add_directive("bookkar_client=info".parse()?)
                .add_directive("chromiumoxide=off".parse()?),
        )
        .with_target(false)
        .init();

    print_banner();

    let license_server = std::env::var("LICENSE_SERVER")
        .unwrap_or_else(|_| LICENSE_SERVER_URL.to_string());

    // Step 1: License token validation (reads env or auto-fetches dev token from license server)
    let token: String = match std::env::var("LICENSE_TOKEN") {
        Ok(t) if !t.trim().is_empty() => {
            println!("  {} Using LICENSE_TOKEN from environment", style("🔑").cyan().bold());
            t.trim().to_string()
        }
        _ => {
            // Attempt auto-fetching dev token from running license server
            let dev_url = format!("{}/api/devtoken", license_server);
            let auto_dev = match reqwest::Client::builder()
                .timeout(Duration::from_millis(1500))
                .build()
            {
                Ok(client) => match client.get(&dev_url).send().await {
                    Ok(resp) if resp.status().is_success() => {
                        resp.json::<serde_json::Value>()
                            .await
                            .ok()
                            .and_then(|j| j.get("token").and_then(|t| t.as_str()).map(|s| s.to_string()))
                    }
                    _ => None,
                },
                _ => None,
            };

            if let Some(tok) = auto_dev {
                println!(
                    "  {} Auto-retrieved dev license token from {}",
                    style("🔑").green().bold(),
                    style(&license_server).cyan()
                );
                tok
            } else {
                Input::new()
                    .with_prompt("License token")
                    .interact_text()?
            }
        }
    };

    match license::validate_token(&token, &license_server).await {
        Ok(claims) => {
            println!(
                "  {} Token valid ({} booking remaining)",
                style("✅").green().bold(),
                claims.max_bookings,
            );
        }
        Err(e) => {
            println!("  {} {}", style("❌").red().bold(), e);
            println!("  Please purchase a valid token from the Telegram bot.");
            return Ok(());
        }
    }

    println!();

    // Step 2: Collect IRCTC credentials (check env vars first for fast testing)
    println!("{}", style("─── IRCTC Credentials ───").cyan().bold());
    let username: String = match std::env::var("IRCTC_USERNAME") {
        Ok(u) if !u.trim().is_empty() => {
            println!("  Username: {}", style(&u).bold());
            u.trim().to_string()
        }
        _ => Input::new()
            .with_prompt("  Username")
            .interact_text()?,
    };

    let password: String = match std::env::var("IRCTC_PASSWORD") {
        Ok(p) if !p.trim().is_empty() => {
            println!("  Password: {}", style("[loaded from environment]").dim());
            p.trim().to_string()
        }
        _ => Password::new()
            .with_prompt("  Password")
            .interact()?,
    };

    println!();

    // Step 3: Load or create journey config
    let saved = config::load_config()?;
    let booking_config = if let Some(ref saved) = saved {
        println!(
            "{} Found saved config: {} → {} on {}",
            style("📋").bold(),
            saved.from_station,
            saved.to_station,
            saved.journey_date,
        );

        let use_saved = if auto_mode {
            println!("  {} Auto-using saved configuration", style("⚡").green().bold());
            0
        } else {
            Select::new()
                .with_prompt("  Use saved configuration?")
                .items(&["Yes, use saved config", "No, enter new details"])
                .default(0)
                .interact()?
        };

        if use_saved == 0 {
            saved.to_booking_config(&username, &password)?
        } else {
            collect_journey_details(&username, &password)?
        }
    } else {
        collect_journey_details(&username, &password)?
    };

    // Display booking summary
    println!();
    println!("{}", style("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━").cyan());
    println!(
        "  {} {} → {}",
        style("🚉").bold(),
        style(&booking_config.from_station).bold(),
        style(&booking_config.to_station).bold(),
    );
    println!("  📅 {}", booking_config.journey_date.format("%d-%b-%Y"));
    println!("  🎫 {} | {}", booking_config.class, booking_config.quota);
    if let Some(ref train) = booking_config.train_number {
        println!("  🚂 Train: {}", train);
    } else {
        println!("  🚂 Train: First available");
    }
    for (i, p) in booking_config.passengers.iter().enumerate() {
        println!("  👤 {}. {}", i + 1, p);
    }
    println!("  💳 {}", booking_config.payment);
    println!(
        "  ⏰ Tatkal window: {:02}:00:00",
        booking_config.class.tatkal_hour()
    );
    println!("{}", style("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━").cyan());
    println!();

    // Time until Tatkal
    let time_left = timer::time_until_tatkal(&booking_config.class);
    println!(
        "  ⏰ Time until Tatkal window: {}",
        style(&time_left).yellow().bold()
    );
    println!();

    let proceed = if auto_mode {
        println!("  {} Auto-confirming launch (AUTO_CONFIRM enabled)...", style("🚀").green().bold());
        0
    } else {
        Select::new()
            .with_prompt("  Ready to launch browser and begin?")
            .items(&["🚀 Launch!", "❌ Cancel"])
            .default(0)
            .interact()?
    };

    if proceed != 0 {
        println!("Cancelled.");
        return Ok(());
    }

    // Step 4: Launch browser and execute booking
    println!();
    println!(
        "  {} Launching Chrome...",
        style("🌐").bold()
    );

    let (browser, _handler) = browser::launcher::launch_stealth_browser().await?;
    let page = browser.new_page("about:blank").await?;

    // Inject stealth JS
    browser::page_state::inject_stealth(&page, browser::launcher::STEALTH_JS).await?;

    // Step 5: Login
    println!(
        "  {} Navigating to IRCTC login...",
        style("🔐").bold()
    );

    irctc::login::perform_login(&page, &booking_config.username, &booking_config.password).await?;

    // Step 6: Fill search form (pre-Tatkal)
    let date_str = booking_config.journey_date.format("%d-%m-%Y").to_string();
    irctc::search::fill_search_form(
        &page,
        &booking_config.from_station,
        &booking_config.to_station,
        &date_str,
        &booking_config.class,
        &booking_config.quota,
    )
    .await?;

    // Step 7: Wait for Tatkal window
    if !timer::is_tatkal_window_open(&booking_config.class) {
        println!();
        println!(
            "  {} Search form pre-filled. Waiting for Tatkal window...",
            style("⏳").yellow().bold()
        );
        timer::wait_for_tatkal_window(&booking_config.class).await;
    }

    // Step 8: FIRE — click search at exact time
    irctc::search::click_search(&page).await?;
    irctc::search::wait_for_results(&page, Duration::from_secs(15)).await?;

    // Step 9: Select train
    let train_name = irctc::search::select_train(
        &page,
        booking_config.train_number.as_deref(),
    )
    .await?;
    println!(
        "  {} Train selected: {}",
        style("🚂").bold(),
        style(&train_name).green()
    );

    // Step 10: Fill passengers
    tokio::time::sleep(Duration::from_secs(2)).await;
    irctc::booking::fill_passengers(
        &page,
        &booking_config.passengers,
        booking_config.auto_upgrade,
    )
    .await?;
    irctc::booking::submit_passengers(&page).await?;

    // Step 11: Aadhaar OTP
    println!();
    println!(
        "  {} Aadhaar OTP sent to your phone. Enter it in the Chrome window.",
        style("⚠️").yellow().bold()
    );
    irctc::booking::wait_for_aadhaar_otp_completion(&page, Duration::from_secs(120)).await?;

    // Step 12: Payment
    irctc::payment::process_payment(&page, &booking_config.payment).await?;

    // Step 13: Wait for confirmation
    let result = irctc::payment::wait_for_confirmation(&page, Duration::from_secs(180)).await?;

    // Success!
    println!();
    println!("{}", style("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━").green());
    println!(
        "  {} {}",
        style("🎉").bold(),
        style("BOOKING CONFIRMED!").green().bold()
    );
    println!("  📋 PNR: {}", style(&result.pnr).bold());
    println!("  {}", result.status_message);
    println!("{}", style("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━").green());

    // Consume the license token
    license::consume_token(&token, &license_server).await?;

    // Keep browser open for user to see
    println!();
    println!("  Browser will stay open. Press Ctrl+C to exit.");
    tokio::signal::ctrl_c().await?;

    Ok(())
}

/// Collect journey details interactively from the user.
fn collect_journey_details(username: &str, password: &str) -> Result<bookkar_common::BookingConfig> {
    println!("{}", style("─── Journey Details ───").cyan().bold());

    let from: String = Input::new()
        .with_prompt("  From station (code)")
        .interact_text()?;

    let to: String = Input::new()
        .with_prompt("  To station (code)")
        .interact_text()?;

    let date_str: String = Input::new()
        .with_prompt("  Journey date (DD-MM-YYYY)")
        .interact_text()?;

    let class_options = &[
        "First AC (1A)",
        "Second AC (2A)",
        "Third AC (3A)",
        "Third AC Economy (3E)",
        "AC Chair Car (CC)",
        "Executive Chair Car (EC)",
        "Sleeper (SL)",
        "Second Sitting (2S)",
    ];
    let class_idx = Select::new()
        .with_prompt("  Class")
        .items(class_options)
        .default(2) // 3A
        .interact()?;

    let class = match class_idx {
        0 => TrainClass::FirstAC,
        1 => TrainClass::SecondAC,
        2 => TrainClass::ThirdAC,
        3 => TrainClass::ThirdACEconomy,
        4 => TrainClass::ChairCar,
        5 => TrainClass::ExecChairCar,
        6 => TrainClass::Sleeper,
        7 => TrainClass::SecondSitting,
        _ => TrainClass::ThirdAC,
    };

    let quota_options = &["Tatkal", "Premium Tatkal"];
    let quota_idx = Select::new()
        .with_prompt("  Quota")
        .items(quota_options)
        .default(0)
        .interact()?;

    let quota = if quota_idx == 0 {
        bookkar_common::Quota::Tatkal
    } else {
        bookkar_common::Quota::PremiumTatkal
    };

    let train_number: String = Input::new()
        .with_prompt("  Train number (or 'any')")
        .default("any".to_string())
        .interact_text()?;

    let train_number = if train_number.to_lowercase() == "any" {
        None
    } else {
        Some(train_number)
    };

    // Passengers
    println!();
    println!("{}", style("─── Passengers ───").cyan().bold());
    let mut passengers = Vec::new();

    loop {
        let idx = passengers.len() + 1;
        println!("  Passenger {}:", idx);

        let name: String = Input::new()
            .with_prompt("    Name")
            .interact_text()?;

        let age: u8 = Input::new()
            .with_prompt("    Age")
            .interact_text()?;

        let gender_options = &["Male", "Female", "Transgender"];
        let gender_idx = Select::new()
            .with_prompt("    Gender")
            .items(gender_options)
            .default(0)
            .interact()?;

        let gender = match gender_idx {
            0 => bookkar_common::Gender::Male,
            1 => bookkar_common::Gender::Female,
            2 => bookkar_common::Gender::Transgender,
            _ => bookkar_common::Gender::Male,
        };

        let berth_options = &[
            "Lower",
            "Middle",
            "Upper",
            "Side Lower",
            "Side Upper",
            "No Preference",
        ];
        let berth_idx = Select::new()
            .with_prompt("    Berth preference")
            .items(berth_options)
            .default(5)
            .interact()?;

        let berth = match berth_idx {
            0 => bookkar_common::BerthPreference::Lower,
            1 => bookkar_common::BerthPreference::Middle,
            2 => bookkar_common::BerthPreference::Upper,
            3 => bookkar_common::BerthPreference::SideLower,
            4 => bookkar_common::BerthPreference::SideUpper,
            _ => bookkar_common::BerthPreference::NoPreference,
        };

        passengers.push(bookkar_common::Passenger {
            name,
            age,
            gender,
            berth_preference: berth,
            nationality: None,
        });

        if passengers.len() >= 6 {
            println!("  Maximum 6 passengers reached.");
            break;
        }

        let more = Select::new()
            .with_prompt("  Add another passenger?")
            .items(&["Yes", "No"])
            .default(1)
            .interact()?;

        if more != 0 {
            break;
        }
    }

    // Payment
    println!();
    println!("{}", style("─── Payment ───").cyan().bold());
    let payment_options = &["UPI", "IRCTC eWallet"];
    let payment_idx = Select::new()
        .with_prompt("  Payment method")
        .items(payment_options)
        .default(0)
        .interact()?;

    let payment = if payment_idx == 0 {
        let vpa: String = Input::new()
            .with_prompt("  UPI ID")
            .interact_text()?;
        bookkar_common::PaymentMethod::Upi { vpa }
    } else {
        bookkar_common::PaymentMethod::IrctcEwallet
    };

    let auto_upgrade = Select::new()
        .with_prompt("  Consider for auto-upgradation?")
        .items(&["Yes", "No"])
        .default(0)
        .interact()?
        == 0;

    let journey_date = NaiveDate::parse_from_str(&date_str, "%d-%m-%Y")?;

    // Save config for next time
    let saved = config::SavedConfig {
        from_station: from.clone(),
        to_station: to.clone(),
        journey_date: date_str.clone(),
        class: class.irctc_value().to_string(),
        quota: quota.irctc_value().to_string(),
        train_number: train_number.clone(),
        passengers: passengers
            .iter()
            .map(|p| config::SavedPassenger {
                name: p.name.clone(),
                age: p.age,
                gender: p.gender.irctc_value().to_string(),
                berth_preference: p.berth_preference.irctc_value().to_string(),
            })
            .collect(),
        payment_method: if payment_idx == 0 {
            "UPI".to_string()
        } else {
            "EWALLET".to_string()
        },
        upi_id: if let bookkar_common::PaymentMethod::Upi { ref vpa } = payment {
            Some(vpa.clone())
        } else {
            None
        },
        auto_upgrade,
    };
    config::save_config(&saved)?;
    println!("  {} Config saved to {:?}", style("💾").bold(), config::config_path());

    Ok(bookkar_common::BookingConfig {
        username: username.to_string(),
        password: password.to_string(),
        from_station: from,
        to_station: to,
        journey_date,
        class,
        quota,
        train_number,
        passengers,
        payment,
        auto_upgrade,
    })
}

fn print_banner() {
    println!();
    println!(
        "{}",
        style("  ╔══════════════════════════════════════════╗").cyan()
    );
    println!(
        "{}",
        style("  ║   🚂 BOOKKAR                      ║").cyan()
    );
    println!(
        "  ║   {}                       ║",
        style(format!("Fast-track your bookings v{}", VERSION)).dim()
    );
    println!(
        "{}",
        style("  ╚══════════════════════════════════════════╝").cyan()
    );
    println!();
}
