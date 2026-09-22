use std::time::Duration;

use anyhow::Result;
use chromiumoxide::Page;
use bookkar_common::Passenger;
use tracing::info;

use crate::browser::page_state::{click_element, eval_js, js_quote, wait_for_element};
use crate::irctc::selectors;

/// Fill passenger details in the booking form.
///
/// Handles adding multiple passengers and filling each one's name, age,
/// gender, and berth preference. Also handles auto-upgrade checkbox.
pub async fn fill_passengers(
    page: &Page,
    passengers: &[Passenger],
    auto_upgrade: bool,
) -> Result<()> {
    info!("Filling passenger details ({} passenger(s))...", passengers.len());

    // Wait for the booking form to load
    wait_for_element(page, selectors::booking::PASSENGER_NAME, Duration::from_secs(15)).await?;
    tokio::time::sleep(Duration::from_millis(500)).await;

    for (i, passenger) in passengers.iter().enumerate() {
        if i > 0 {
            // Click "Add Passenger" for passengers beyond the first
            info!("Adding passenger {}...", i + 1);
            click_element(page, selectors::booking::ADD_PASSENGER_BUTTON).await?;
            tokio::time::sleep(Duration::from_millis(500)).await;
        }

        // Fill passenger details using indexed selectors
        // IRCTC typically uses array-indexed form controls
        fill_single_passenger(page, i, passenger).await?;
        info!(
            "  Passenger {}: {} | {} | {} | {}",
            i + 1,
            passenger.name,
            passenger.gender,
            passenger.age,
            passenger.berth_preference
        );
    }

    // Handle auto-upgrade checkbox
    if auto_upgrade {
        let checked = eval_js(
            page,
            &format!(
                "document.querySelector({})?.checked || false",
                js_quote(selectors::booking::AUTO_UPGRADE_CHECKBOX)
            ),
        )
        .await?;

        if checked != "true" {
            click_element(page, selectors::booking::AUTO_UPGRADE_CHECKBOX).await?;
            info!("✅ Auto-upgrade enabled");
        }
    }

    info!("✅ All passenger details filled");
    Ok(())
}

/// Fill details for a single passenger at the given index.
async fn fill_single_passenger(page: &Page, index: usize, passenger: &Passenger) -> Result<()> {
    // Build indexed selectors — IRCTC's Angular uses formArrayName
    // The exact indexing pattern needs to be verified against the live DOM.
    // Common patterns: nth-child, array index in formcontrolname, or data attributes.

    let name_js = format!(
        r#"
        (() => {{
            const inputs = document.querySelectorAll({});
            if (inputs[{index}]) {{
                const nativeSetter = Object.getOwnPropertyDescriptor(
                    window.HTMLInputElement.prototype, 'value'
                )?.set;
                if (nativeSetter) {{
                    nativeSetter.call(inputs[{index}], {});
                }} else {{
                    inputs[{index}].value = {};
                }}
                inputs[{index}].dispatchEvent(new Event('input', {{ bubbles: true }}));
                return 'OK';
            }}
            return 'NOT_FOUND';
        }})()
        "#,
        js_quote(selectors::booking::PASSENGER_NAME),
        js_quote(&passenger.name),
        js_quote(&passenger.name),
        index = index,
    );
    eval_js(page, &name_js).await?;

    let age_js = format!(
        r#"
        (() => {{
            const inputs = document.querySelectorAll({});
            if (inputs[{index}]) {{
                const nativeSetter = Object.getOwnPropertyDescriptor(
                    window.HTMLInputElement.prototype, 'value'
                )?.set;
                if (nativeSetter) {{
                    nativeSetter.call(inputs[{index}], {});
                }} else {{
                    inputs[{index}].value = {};
                }}
                inputs[{index}].dispatchEvent(new Event('input', {{ bubbles: true }}));
                return 'OK';
            }}
            return 'NOT_FOUND';
        }})()
        "#,
        js_quote(selectors::booking::PASSENGER_AGE),
        js_quote(&passenger.age.to_string()),
        js_quote(&passenger.age.to_string()),
        index = index,
    );
    eval_js(page, &age_js).await?;

    // Select gender from dropdown
    let gender_js = format!(
        r#"
        (() => {{
            const selects = document.querySelectorAll({});
            if (selects[{index}]) {{
                selects[{index}].value = {};
                selects[{index}].dispatchEvent(new Event('change', {{ bubbles: true }}));
                return 'OK';
            }}
            return 'NOT_FOUND';
        }})()
        "#,
        js_quote(selectors::booking::PASSENGER_GENDER),
        js_quote(passenger.gender.irctc_value()),
        index = index,
    );
    eval_js(page, &gender_js).await?;

    // Select berth preference
    let berth_js = format!(
        r#"
        (() => {{
            const selects = document.querySelectorAll({});
            if (selects[{index}]) {{
                selects[{index}].value = {};
                selects[{index}].dispatchEvent(new Event('change', {{ bubbles: true }}));
                return 'OK';
            }}
            return 'NOT_FOUND';
        }})()
        "#,
        js_quote(selectors::booking::BERTH_PREFERENCE),
        js_quote(passenger.berth_preference.irctc_value()),
        index = index,
    );
    eval_js(page, &berth_js).await?;

    // Small delay between passengers for Angular to process
    tokio::time::sleep(Duration::from_millis(200)).await;
    Ok(())
}

/// Submit the passenger form and proceed to payment.
/// This may trigger Aadhaar OTP verification.
pub async fn submit_passengers(page: &Page) -> Result<()> {
    info!("Submitting passenger details...");
    click_element(page, selectors::booking::CONTINUE_BUTTON).await?;
    tokio::time::sleep(Duration::from_secs(1)).await;
    Ok(())
}

/// Wait for the Aadhaar OTP step.
/// The user enters the OTP manually in Chrome.
pub async fn wait_for_aadhaar_otp_completion(page: &Page, timeout: Duration) -> Result<()> {
    info!("⚠️  Aadhaar OTP has been sent to your registered mobile number.");
    info!("   Please enter the OTP in the Chrome window.");

    let start = std::time::Instant::now();

    loop {
        // Check if we've moved past the Aadhaar OTP page
        let state_js = r#"
            (() => {
                // Check if Aadhaar OTP input is still visible
                const otpInput = document.querySelector('input[formcontrolname="aadhaarOtp"], input#aadhaarOtp');
                if (otpInput && otpInput.offsetParent !== null) return 'WAITING';

                // Check if we've moved to the payment page
                const paymentSection = document.querySelector('.payment-options, .pay-section, .bank-type');
                if (paymentSection) return 'PAYMENT_READY';

                // Check for error
                const error = document.querySelector('.alert-danger, .error-message');
                if (error && error.innerText.trim()) return 'ERROR:' + error.innerText.trim();

                return 'TRANSITIONING';
            })()
        "#;

        let result = eval_js(page, state_js).await?;

        match result.as_str() {
            "PAYMENT_READY" => {
                info!("✅ Aadhaar verification complete, proceeding to payment");
                return Ok(());
            }
            s if s.starts_with("ERROR:") => {
                let error_msg = s.strip_prefix("ERROR:").unwrap_or("Unknown");
                return Err(anyhow::anyhow!("Aadhaar verification failed: {}", error_msg));
            }
            "WAITING" => {
                // Still waiting for user to enter OTP
            }
            _ => {
                // Page transitioning
            }
        }

        if start.elapsed() > timeout {
            return Err(anyhow::anyhow!(
                "Timeout waiting for Aadhaar OTP. Please enter the OTP faster next time."
            ));
        }

        tokio::time::sleep(Duration::from_millis(500)).await;
    }
}
