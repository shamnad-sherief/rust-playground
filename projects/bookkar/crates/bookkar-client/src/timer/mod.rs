use chrono::{Local, NaiveTime, Timelike};
use bookkar_common::TrainClass;
use tracing::info;

/// Wait for the exact Tatkal booking window to open.
///
/// AC classes open at 10:00:00 AM, non-AC at 11:00:00 AM.
///
/// Strategy:
/// - Coarse wait using `tokio::time::sleep` until 500ms before target
/// - Spin-lock for the final milliseconds for maximum precision
///
/// Returns the actual time the function completed (for logging precision).
pub async fn wait_for_tatkal_window(class: &TrainClass) -> chrono::DateTime<Local> {
    let target_hour = class.tatkal_hour();
    let target_time = NaiveTime::from_hms_opt(target_hour, 0, 0).unwrap();
    let now = Local::now();
    let target = now
        .date_naive()
        .and_time(target_time)
        .and_local_timezone(Local)
        .single()
        .expect("Failed to construct target datetime");

    let remaining = target.signed_duration_since(now);

    if remaining.num_milliseconds() <= 0 {
        info!("Tatkal window is already open! Proceeding immediately.");
        return Local::now();
    }

    let total_secs = remaining.num_seconds();
    let total_mins = total_secs / 60;

    info!(
        "⏰ Tatkal window opens at {:02}:00:00. Waiting {} min {} sec...",
        target_hour,
        total_mins,
        total_secs % 60,
    );

    // Coarse wait: sleep until 500ms before target
    let coarse_ms = remaining.num_milliseconds() - 500;
    if coarse_ms > 0 {
        // Print countdown at intervals
        let coarse_duration = std::time::Duration::from_millis(coarse_ms as u64);
        let sleep_until = std::time::Instant::now() + coarse_duration;

        // Print updates every 30 seconds until last minute, then every 5 seconds
        loop {
            let now = Local::now();
            let remaining = target.signed_duration_since(now);
            let secs_left = remaining.num_seconds();

            if secs_left <= 1 {
                break;
            }

            let status = format!(
                "⏰ {:02}:{:02}:{:02} — {} seconds to Tatkal window",
                now.hour(),
                now.minute(),
                now.second(),
                secs_left,
            );
            info!("{}", status);

            let sleep_for = if secs_left > 60 {
                30_000u64 // 30 sec intervals
            } else if secs_left > 10 {
                5_000 // 5 sec intervals
            } else {
                1_000 // 1 sec intervals
            };

            let sleep_dur = std::time::Duration::from_millis(sleep_for.min(secs_left as u64 * 1000));
            tokio::time::sleep(sleep_dur).await;

            if std::time::Instant::now() >= sleep_until {
                break;
            }
        }
    }

    // Fine-grained spin-lock for final milliseconds
    // This busy-waits to achieve sub-millisecond precision
    info!("⚡ Final countdown — spin-locking for precision...");
    loop {
        let now = Local::now();
        if now >= target {
            let fire_time = Local::now();
            let delta_ms = fire_time.signed_duration_since(target).num_milliseconds();
            info!(
                "🚀 FIRED at {:02}:{:02}:{:02}.{:03} ({}ms after target)",
                fire_time.hour(),
                fire_time.minute(),
                fire_time.second(),
                fire_time.timestamp_subsec_millis(),
                delta_ms,
            );
            return fire_time;
        }
        std::hint::spin_loop();
    }
}

/// Check if the current time is past the Tatkal window for the given class.
pub fn is_tatkal_window_open(class: &TrainClass) -> bool {
    let now = Local::now();
    let target_hour = class.tatkal_hour();
    now.hour() >= target_hour
}

/// Get human-readable time until Tatkal window opens.
pub fn time_until_tatkal(class: &TrainClass) -> String {
    let now = Local::now();
    let target_hour = class.tatkal_hour();
    let target_time = NaiveTime::from_hms_opt(target_hour, 0, 0).unwrap();
    let target = now
        .date_naive()
        .and_time(target_time)
        .and_local_timezone(Local)
        .single()
        .expect("Failed to construct target datetime");

    let remaining = target.signed_duration_since(now);

    if remaining.num_milliseconds() <= 0 {
        return "NOW (window is open)".to_string();
    }

    let mins = remaining.num_minutes();
    let secs = remaining.num_seconds() % 60;

    format!("{} min {} sec", mins, secs)
}
