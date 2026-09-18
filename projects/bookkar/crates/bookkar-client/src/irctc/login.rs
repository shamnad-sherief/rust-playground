use std::time::Duration;

use anyhow::Result;
use chromiumoxide::Page;
use tracing::{info, warn};

use crate::browser::launcher::STEALTH_JS;
use crate::browser::page_state::{
    click_element, dismiss_modals, human_type, inject_stealth, wait_for_element,
    wait_for_url_contains,
};
use crate::irctc::selectors;

const IRCTC_BASE_URL: &str = "https://www.irctc.co.in/nget/train-search";
const IRCTC_LOGIN_URL: &str = "https://www.irctc.co.in/nget/train-search";

/// Navigate to IRCTC and perform login.
///
/// This function fills in the username and password, submits the form,
/// and then waits for the user to manually enter the OTP in the browser.
/// It detects login completion by watching for the post-login URL or
/// the appearance of the logged-in indicator element.
pub async fn perform_login(page: &Page, username: &str, password: &str) -> Result<()> {
    // Navigate to IRCTC
    info!("Navigating to IRCTC...");
    page.goto(IRCTC_BASE_URL).await?;
    tokio::time::sleep(Duration::from_secs(3)).await;

    // Inject stealth patches
    inject_stealth(page, STEALTH_JS).await?;

    // Dismiss any initial popups/modals
    dismiss_modals(page).await;

    // Click the LOGIN button to open the login form
    info!("Opening login form...");
    let login_trigger = page
        .find_element("a.loginText")
        .await;

    if let Ok(trigger) = login_trigger {
        trigger.click().await?;
        tokio::time::sleep(Duration::from_secs(2)).await;
    }

    // Fill username
    info!("Filling credentials...");
    human_type(page, selectors::login::USERNAME_INPUT, username).await?;
    tokio::time::sleep(Duration::from_millis(300)).await;

    // Fill password
    human_type(page, selectors::login::PASSWORD_INPUT, password).await?;
    tokio::time::sleep(Duration::from_millis(300)).await;

    // Handle CAPTCHA if present
    // Note: IRCTC has been moving away from visible CAPTCHA as of 2026,
    // but we still handle it if it appears. The user will need to solve it manually.
    let has_captcha = page.find_element(selectors::login::CAPTCHA_INPUT).await.is_ok();
    if has_captcha {
        warn!("⚠️  CAPTCHA detected! Please solve the CAPTCHA in the browser window.");
        // Wait for the user to fill in the CAPTCHA
        // We detect this by waiting for the CAPTCHA input to have a value
        wait_for_captcha_solved(page).await?;
    }

    // Click login button
    info!("Submitting login...");
    click_element(page, selectors::login::LOGIN_BUTTON).await?;

    // Now wait for OTP
    info!("⚠️  OTP has been sent to your phone. Please enter it in the Chrome window.");
    info!("   Waiting for you to complete the OTP verification...");

    // Wait for login to complete — detected by the post-login page loading
    // or the logged-in indicator appearing.
    // We give the user up to 120 seconds to enter the OTP.
    wait_for_login_complete(page, Duration::from_secs(120)).await?;

    info!("✅ Login successful!");
    Ok(())
}

/// Wait for the CAPTCHA input to be filled by the user.
async fn wait_for_captcha_solved(page: &Page) -> Result<()> {
    let start = std::time::Instant::now();
    loop {
        let js = format!(
            "document.querySelector('{}')?.value?.length > 0",
            selectors::login::CAPTCHA_INPUT
        );
        let result = page.evaluate(js).await?;
        if result.into_value::<bool>().unwrap_or(false) {
            return Ok(());
        }

        if start.elapsed() > Duration::from_secs(60) {
            return Err(anyhow::anyhow!("Timeout waiting for CAPTCHA to be solved"));
        }

        tokio::time::sleep(Duration::from_millis(500)).await;
    }
}

/// Wait for the login process to complete by detecting page state changes.
/// The user enters OTP manually; we just watch for the result.
async fn wait_for_login_complete(page: &Page, timeout: Duration) -> Result<()> {
    let start = std::time::Instant::now();

    loop {
        // Check if we're on the post-login page (train search with logged-in state)
        let check_js = r#"
            (() => {
                // Check for logged-in menu element
                const loggedIn = document.querySelector('a.logoutText, span.user-name, a.dropdown-toggle.profile');
                if (loggedIn) return 'LOGGED_IN';

                // Check for OTP input still visible (user hasn't entered it yet)
                const otpInput = document.querySelector('input#otp, input[type="text"][placeholder*="OTP"]');
                if (otpInput) return 'WAITING_OTP';

                // Check for error messages
                const error = document.querySelector('.alert-danger, .loginError');
                if (error && error.innerText.trim()) return 'ERROR:' + error.innerText.trim();

                return 'LOADING';
            })()
        "#;

        let result = page.evaluate(check_js).await?;
        let state = result.into_value::<String>().unwrap_or_default();

        match state.as_str() {
            "LOGGED_IN" => return Ok(()),
            s if s.starts_with("ERROR:") => {
                let error_msg = s.strip_prefix("ERROR:").unwrap_or("Unknown error");
                return Err(anyhow::anyhow!("Login failed: {}", error_msg));
            }
            "WAITING_OTP" => {
                // Still waiting for user to enter OTP — continue polling
            }
            _ => {
                // Loading or transitioning
            }
        }

        if start.elapsed() > timeout {
            return Err(anyhow::anyhow!(
                "Timeout ({:?}) waiting for login to complete. Did you enter the OTP?",
                timeout
            ));
        }

        tokio::time::sleep(Duration::from_millis(500)).await;
    }
}
