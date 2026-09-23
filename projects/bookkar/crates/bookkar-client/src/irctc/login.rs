use std::time::Duration;

use anyhow::Result;
use chromiumoxide::Page;
use tracing::{debug, info, warn};

use crate::browser::launcher::STEALTH_JS;
use crate::browser::page_state::{
    click_element, dismiss_modals, human_type, inject_stealth,
};
use crate::irctc::selectors;

pub fn get_irctc_base_url() -> String {
    std::env::var("IRCTC_URL")
        .or_else(|_| std::env::var("IRCTC_BASE_URL"))
        .unwrap_or_else(|_| "https://www.irctc.co.in/eticket/train-search".to_string())
}

/// Navigate to IRCTC and perform login.
///
/// This function fills in the username and password, submits the form,
/// and detects login completion by watching for the post-login state.
pub async fn perform_login(page: &Page, username: &str, password: &str) -> Result<()> {
    let base_url = get_irctc_base_url();
    // Navigate to IRCTC
    info!("Navigating to IRCTC ({base_url})...");
    page.goto(&base_url).await?;
    tokio::time::sleep(Duration::from_secs(3)).await;

    // Inject stealth patches
    inject_stealth(page, STEALTH_JS).await?;

    // Dismiss any initial popups/modals
    dismiss_modals(page).await;
    tokio::time::sleep(Duration::from_millis(500)).await;

    // Click the LOGIN button to open the login form
    info!("Opening login form...");
    let click_login_js = r#"
        (() => {
            const loginBtn = document.querySelector('button.btn-login, a.loginText, button[aria-label*="Login"], a[aria-label*="Login"], a[aria-label="Click here to Login in application"]');
            if (loginBtn) {
                loginBtn.click();
                return 'CLICKED';
            }
            return 'NOT_FOUND';
        })()
    "#;

    if let Ok(res) = page.evaluate(click_login_js).await {
        if res.into_value::<String>().unwrap_or_default() != "CLICKED" {
            if let Ok(trigger) = page.find_element(selectors::login::LOGIN_TRIGGER).await {
                let _ = trigger.click().await;
            }
        }
    }
    tokio::time::sleep(Duration::from_secs(2)).await;

    // Fill username
    info!("Filling credentials...");
    human_type(page, selectors::login::USERNAME_INPUT, username).await?;
    tokio::time::sleep(Duration::from_millis(300)).await;

    // Fill password
    human_type(page, selectors::login::PASSWORD_INPUT, password).await?;
    tokio::time::sleep(Duration::from_millis(300)).await;

    // Trigger Angular form control change/blur events so submit button activates
    let trigger_validation_js = r#"
        (() => {
            const u = document.querySelector('input#username, input[formcontrolname="userid"]');
            const p = document.querySelector('input#password, input[formcontrolname="password"]');
            if (u) {
                u.dispatchEvent(new Event('input', { bubbles: true }));
                u.dispatchEvent(new Event('change', { bubbles: true }));
                u.dispatchEvent(new Event('blur', { bubbles: true }));
            }
            if (p) {
                p.dispatchEvent(new Event('input', { bubbles: true }));
                p.dispatchEvent(new Event('change', { bubbles: true }));
                p.dispatchEvent(new Event('blur', { bubbles: true }));
            }
        })()
    "#;
    let _ = page.evaluate(trigger_validation_js).await;
    tokio::time::sleep(Duration::from_millis(300)).await;

    // Check for CAPTCHA
    let captcha_status_js = r#"
        (() => {
            const img = document.querySelector('img.captcha-img, img[src*="captcha" i], .captcha-img, app-captcha');
            const input = document.querySelector('input[formcontrolname*="captcha" i], input#captcha, input#nlpAnswer, input[placeholder*="captcha" i]');
            return JSON.stringify({
                hasImg: !!img,
                hasInput: !!input,
                hasValue: !!(input && input.value && input.value.trim().length > 0)
            });
        })()
    "#;

    let has_captcha = if let Ok(res) = page.evaluate(captcha_status_js).await {
        let val = res.into_value::<String>().unwrap_or_default();
        if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(&val) {
            parsed["hasImg"].as_bool().unwrap_or(false) || parsed["hasInput"].as_bool().unwrap_or(false)
        } else {
            false
        }
    } else {
        false
    };

    if has_captcha {
        warn!("⚠️  CAPTCHA detected on login form! Please solve the CAPTCHA in the Chrome window.");
        info!("   Waiting for you to enter the CAPTCHA...");
        wait_for_captcha_solved(page).await?;
        info!("✅ CAPTCHA entered!");
        tokio::time::sleep(Duration::from_millis(500)).await;
    }

    // Natural pause before submitting to avoid bot velocity detection
    tokio::time::sleep(Duration::from_millis(800)).await;

    // Click login button
    info!("Submitting login...");

    // First attempt: real CDP click on the active login button inside the dialog
    let cdp_click_res = match page
        .find_element(
            ".login-dialog button.btn-action.btn-login, button.btn-action.btn-login.active, .login-dialog button[type='submit']",
        )
        .await
    {
        Ok(el) => match el.click().await {
            Ok(_) => {
                info!("Login submit action: CDP_CLICKED_BUTTON");
                true
            }
            Err(e) => {
                debug!("CDP click on login button failed ({}), trying JS click", e);
                false
            }
        },
        Err(_) => false,
    };

    if !cdp_click_res {
        let submit_js = r#"
            (() => {
                // 1. Check strictly within .login-dialog or modal
                const dialog = document.querySelector('.login-dialog, app-login, .modal-dialog, div[role="dialog"]');
                if (dialog) {
                    let btn = dialog.querySelector('button[type="submit"], button.btn-action.btn-login, button.btn-login');
                    if (!btn) {
                        const buttons = Array.from(dialog.querySelectorAll('button'));
                        btn = buttons.find(b => {
                            const t = (b.innerText || '').trim().toUpperCase();
                            return (t === 'LOGIN' || t === 'SIGN IN' || t === 'SIGN-IN') && !b.classList.contains('close-btn');
                        });
                    }
                    if (btn) {
                        if (btn.disabled) {
                            btn.disabled = false;
                            btn.removeAttribute('disabled');
                        }
                        btn.click();
                        return 'CLICKED_DIALOG_BTN: ' + (btn.className || '') + ' (' + (btn.innerText || '').trim() + ')';
                    }
                }

                // 2. Direct unique class for beta site submit button
                const directBtn = document.querySelector('button.btn-action.btn-login');
                if (directBtn) {
                    if (directBtn.disabled) {
                        directBtn.disabled = false;
                        directBtn.removeAttribute('disabled');
                    }
                    directBtn.click();
                    return 'CLICKED_DIRECT_BTN: ' + (directBtn.className || '') + ' (' + (directBtn.innerText || '').trim() + ')';
                }

                return 'NOT_FOUND';
            })()
        "#;

        let clicked_result = page
            .evaluate(submit_js)
            .await
            .ok()
            .and_then(|v| v.into_value::<String>().ok())
            .unwrap_or_else(|| "ERROR".to_string());

        info!("Login submit action: {}", clicked_result);

        if clicked_result.starts_with("NOT_FOUND") {
            // Fallback to selector
            let _ = click_element(page, selectors::login::LOGIN_BUTTON).await;
        }
    }

    // Wait for login to complete (IRCTC standard login does not require OTP)
    info!("Waiting for login confirmation...");
    wait_for_login_complete(page, Duration::from_secs(60)).await?;

    info!("✅ Login successful!");
    Ok(())
}

/// Wait for the CAPTCHA input to be filled by the user.
async fn wait_for_captcha_solved(page: &Page) -> Result<()> {
    let start = std::time::Instant::now();
    let check_js = r#"
        (() => {
            // Check if already logged in (user may have clicked SIGN IN manually)
            const allLinks = Array.from(document.querySelectorAll('a, button, span, strong'));
            for (const el of allLinks) {
                const txt = (el.innerText || '').trim().toUpperCase();
                if (txt === 'LOGOUT' || txt === 'LOG OUT' || txt.includes('LOGOUT')) return 'LOGGED_IN';
            }

            // Check if captcha input has at least 3 characters
            const input = document.querySelector('input[formcontrolname*="captcha" i], input#captcha, input#nlpAnswer, input[placeholder*="captcha" i]');
            if (input && input.value && input.value.trim().length >= 3) {
                return 'SOLVED';
            }

            // If captcha input is gone / modal closed
            const modal = document.querySelector('app-login, .modal-dialog, p-dialog');
            if (!modal) return 'MODAL_CLOSED';

            return 'WAITING';
        })()
    "#;

    loop {
        if let Ok(res) = page.evaluate(check_js).await {
            let status = res.into_value::<String>().unwrap_or_default();
            if status == "SOLVED" || status == "LOGGED_IN" || status == "MODAL_CLOSED" {
                return Ok(());
            }
        }

        if start.elapsed() > Duration::from_secs(120) {
            return Err(anyhow::anyhow!("Timeout waiting for CAPTCHA to be solved"));
        }

        tokio::time::sleep(Duration::from_millis(500)).await;
    }
}

/// Wait for the login process to complete by detecting page state changes.
/// Standard logins complete directly without OTP. If an OTP prompt exceptionally
/// appears, we detect it dynamically and prompt the user.
async fn wait_for_login_complete(page: &Page, timeout: Duration) -> Result<()> {
    let start = std::time::Instant::now();
    let mut otp_prompted = false;
    let mut last_log = std::time::Instant::now();
    let mut last_error_log = std::time::Instant::now() - Duration::from_secs(10);

    loop {
        // Check page state
        let check_js = r#"
            (() => {
                // 1. Check for logged-in user element or logout icon in header
                const profile = document.querySelector('.nav-link-1, a.logoutText, span.user-name, a.dropdown-toggle.profile, .fa-sign-out');
                if (profile && profile.innerText && profile.innerText.trim().length > 0) {
                    return 'LOGGED_IN:profile_' + profile.innerText.trim();
                }

                // Check for any link or button containing "Logout"
                const elements = Array.from(document.querySelectorAll('a, button, span, strong'));
                for (const el of elements) {
                    const txt = (el.innerText || '').trim().toUpperCase();
                    if (txt === 'LOGOUT' || txt === 'LOG OUT' || txt.includes('LOGOUT')) {
                        return 'LOGGED_IN:logout_text';
                    }
                }

                // Check for user greeting (e.g. "Welcome <user>")
                for (const el of elements) {
                    const txt = (el.innerText || '').trim();
                    if (txt.toLowerCase().startsWith('welcome ') && txt.length > 8) {
                        return 'LOGGED_IN:greeting';
                    }
                }

                // Check sessionStorage for authentication tokens
                try {
                    if (sessionStorage.getItem('token') || sessionStorage.getItem('userName') || sessionStorage.getItem('userId')) {
                        return 'LOGGED_IN:storage_token';
                    }
                } catch (e) {}

                // Check if login dialog/modal is closed and login button is gone from header
                const modal = document.querySelector('.login-dialog, div[role="dialog"], .modal-dialog, app-login');
                const modalVisible = modal && modal.offsetParent !== null && window.getComputedStyle(modal).display !== 'none';
                const loginBtn = document.querySelector('button.btn-login:not([type="submit"]), a.loginText');
                const loginBtnVisible = loginBtn && loginBtn.offsetParent !== null && window.getComputedStyle(loginBtn).display !== 'none';

                if (!modalVisible && !loginBtnVisible && window.location.href.includes('train-search')) {
                    return 'LOGGED_IN:modal_closed';
                }

                // 2. Check for error messages
                const error = document.querySelector('.alert-danger, .loginError, .toast-error, .ui-messages-error, .ui-toast-detail');
                if (error && error.innerText && error.innerText.trim().length > 0 && error.offsetParent !== null) {
                    return 'ERROR:' + error.innerText.trim();
                }

                // 3. Check if an OTP input is ACTUALLY present and visible in the DOM
                const otpInput = document.querySelector('input#otp, input[type="text"][placeholder*="OTP" i], input[formcontrolname*="otp" i]');
                if (otpInput && otpInput.offsetParent !== null) {
                    return 'WAITING_OTP';
                }

                // 4. Check if captcha is still visible and empty
                const captchaInput = document.querySelector('input[formcontrolname*="captcha" i], input#captcha, input#nlpAnswer, input[placeholder*="captcha" i]');
                if (captchaInput && captchaInput.offsetParent !== null && (!captchaInput.value || captchaInput.value.trim().length === 0)) {
                    return 'CAPTCHA_EMPTY';
                }

                return 'LOADING (modal=' + (modalVisible ? 'visible' : 'hidden') + ')';
            })()
        "#;

        let result = page.evaluate(check_js).await?;
        let state = result.into_value::<String>().unwrap_or_default();

        if state.starts_with("LOGGED_IN") {
            info!("Login state: {}", state);
            return Ok(());
        } else if let Some(error_msg) = state.strip_prefix("ERROR:") {
            if last_error_log.elapsed() > Duration::from_secs(5) {
                warn!("⚠️  Login notification from IRCTC: {error_msg}");
                info!("   You can complete login or click LOGIN in the visible Chrome window if needed...");
                last_error_log = std::time::Instant::now();
            }
        } else if state == "WAITING_OTP" {
            if !otp_prompted {
                info!("⚠️  OTP verification requested by IRCTC. Please enter the OTP in the Chrome window.");
                otp_prompted = true;
            }
        } else if state == "CAPTCHA_EMPTY" {
            warn!("⚠️  CAPTCHA is required on the login dialog. Please enter it in the Chrome window.");
            tokio::time::sleep(Duration::from_secs(2)).await;
            continue;
        }

        // Periodic diagnostic log every 5 seconds
        if last_log.elapsed() > Duration::from_secs(5) {
            info!("Waiting for login confirmation... [{}]", state);
            last_log = std::time::Instant::now();
        }

        if start.elapsed() > timeout {
            return Err(anyhow::anyhow!(
                "Timeout ({:?}) waiting for login to complete. Last state: {}",
                timeout,
                state
            ));
        }

        tokio::time::sleep(Duration::from_millis(500)).await;
    }
}
