use std::time::Duration;

use anyhow::{anyhow, Result};
use chromiumoxide::Page;
use bookkar_common::{BookingResult, PaymentMethod};
use tracing::{info, warn};

use crate::browser::page_state::{click_element, eval_js, human_type, wait_for_element};
use crate::irctc::selectors;

/// Handle the payment step of the booking flow.
///
/// Selects the payment method, fills in UPI ID if applicable,
/// and waits for the user to approve the payment.
pub async fn process_payment(page: &Page, payment: &PaymentMethod) -> Result<()> {
    info!("Processing payment...");

    // Wait for the payment options to load
    tokio::time::sleep(Duration::from_secs(2)).await;

    match payment {
        PaymentMethod::Upi { vpa } => {
            info!("Selecting UPI payment...");
            // Try to find and click the UPI option
            let upi_js = r#"
                (() => {
                    // Try various selectors for the UPI option
                    const options = document.querySelectorAll('.bank-type, .pay-option, .payment-type');
                    for (const opt of options) {
                        if (opt.innerText.toLowerCase().includes('upi')) {
                            opt.click();
                            return 'CLICKED';
                        }
                    }
                    // Fallback: try radio button or input
                    const radio = document.querySelector('input[value*="UPI"], input[value*="upi"]');
                    if (radio) {
                        radio.click();
                        return 'CLICKED_RADIO';
                    }
                    return 'NOT_FOUND';
                })()
            "#;

            let result = eval_js(page, upi_js).await?;
            if result == "NOT_FOUND" {
                warn!("Could not find UPI option automatically. Please select it in the browser.");
            }

            tokio::time::sleep(Duration::from_millis(500)).await;

            // Fill UPI VPA (Virtual Payment Address)
            info!("Filling UPI ID: {}", vpa);
            let vpa_js = format!(
                r#"
                (() => {{
                    // Try the standard VPA input
                    let input = document.querySelector('{}');
                    // Fallback: try any input with placeholder containing 'upi' or 'vpa'
                    if (!input) {{
                        input = document.querySelector('input[placeholder*="UPI"], input[placeholder*="VPA"], input[placeholder*="upi"]');
                    }}
                    if (input) {{
                        const nativeSetter = Object.getOwnPropertyDescriptor(
                            window.HTMLInputElement.prototype, 'value'
                        ).set;
                        nativeSetter.call(input, '{}');
                        input.dispatchEvent(new Event('input', {{ bubbles: true }}));
                        input.dispatchEvent(new Event('change', {{ bubbles: true }}));
                        return 'OK';
                    }}
                    return 'NOT_FOUND';
                }})()
                "#,
                selectors::payment::UPI_VPA_INPUT,
                vpa,
            );

            let result = eval_js(page, &vpa_js).await?;
            if result == "NOT_FOUND" {
                warn!("Could not find UPI VPA input. Please enter your UPI ID in the browser.");
                // Wait for user to manually enter VPA
                tokio::time::sleep(Duration::from_secs(10)).await;
            }
        }
        PaymentMethod::IrctcEwallet => {
            info!("Selecting IRCTC eWallet...");
            let wallet_js = r#"
                (() => {
                    const options = document.querySelectorAll('.bank-type, .pay-option, .payment-type');
                    for (const opt of options) {
                        if (opt.innerText.toLowerCase().includes('wallet') ||
                            opt.innerText.toLowerCase().includes('ewallet')) {
                            opt.click();
                            return 'CLICKED';
                        }
                    }
                    return 'NOT_FOUND';
                })()
            "#;

            let result = eval_js(page, wallet_js).await?;
            if result == "NOT_FOUND" {
                warn!("Could not find eWallet option. Please select it in the browser.");
            }
        }
    }

    tokio::time::sleep(Duration::from_millis(500)).await;

    // Click the Pay button
    info!("Clicking 'Pay'...");
    let pay_js = format!(
        r#"
        (() => {{
            let btn = document.querySelector('{}');
            if (!btn) {{
                // Fallback: try any submit-like button in the payment section
                btn = document.querySelector('button[type="submit"], button.btn-primary.pay, input[type="submit"]');
            }}
            if (btn) {{
                btn.click();
                return 'CLICKED';
            }}
            return 'NOT_FOUND';
        }})()
        "#,
        selectors::payment::PAY_BUTTON,
    );

    let result = eval_js(page, &pay_js).await?;
    if result == "NOT_FOUND" {
        warn!("Could not find Pay button. Please click it manually in the browser.");
    }

    info!("⚠️  APPROVE THE PAYMENT ON YOUR UPI APP NOW!");
    info!("   Waiting for payment confirmation...");

    Ok(())
}

/// Wait for payment to complete and extract the PNR number.
pub async fn wait_for_confirmation(page: &Page, timeout: Duration) -> Result<BookingResult> {
    let start = std::time::Instant::now();

    loop {
        let check_js = r#"
            (() => {
                // Check for PNR on the confirmation page
                const pnrEl = document.querySelector('.pnr-val, .pnr-number, td:has(+ td:contains("PNR"))');
                if (pnrEl) {
                    return 'PNR:' + pnrEl.innerText.trim();
                }

                // Check for success indicators
                const success = document.querySelector('.booking-confirmed, .ticket-booked, .success-message');
                if (success) {
                    // Try to extract PNR from the success page
                    const text = success.innerText;
                    const pnrMatch = text.match(/\b(\d{10})\b/);
                    if (pnrMatch) return 'PNR:' + pnrMatch[1];
                    return 'SUCCESS_NO_PNR';
                }

                // Check for payment failure
                const fail = document.querySelector('.payment-failed, .transaction-failed, .alert-danger');
                if (fail && fail.innerText.toLowerCase().includes('fail')) {
                    return 'FAILED:' + fail.innerText.trim();
                }

                // Check if we're still on the payment processing page
                const processing = document.querySelector('.payment-processing, .loading, .spinner');
                if (processing) return 'PROCESSING';

                return 'WAITING';
            })()
        "#;

        let result = eval_js(page, check_js).await?;

        if result.starts_with("PNR:") {
            let pnr = result.strip_prefix("PNR:").unwrap_or("").to_string();
            info!("🎉 BOOKING CONFIRMED! PNR: {}", pnr);

            // Try to extract train info
            let train_info_js = r#"
                (() => {
                    const info = {};
                    const trainNo = document.querySelector('.train-number, .trainNo');
                    const trainName = document.querySelector('.train-name, .trainName');
                    info.number = trainNo ? trainNo.innerText.trim() : 'Unknown';
                    info.name = trainName ? trainName.innerText.trim() : 'Unknown';
                    return JSON.stringify(info);
                })()
            "#;

            let _train_json = eval_js(page, train_info_js).await.unwrap_or_default();

            return Ok(BookingResult {
                pnr,
                train_number: "Unknown".to_string(),
                train_name: "Unknown".to_string(),
                status_message: "Booking confirmed successfully!".to_string(),
            });
        }

        if result == "SUCCESS_NO_PNR" {
            warn!("Booking appears successful but could not extract PNR automatically");
            return Ok(BookingResult {
                pnr: "CHECK_EMAIL".to_string(),
                train_number: "Unknown".to_string(),
                train_name: "Unknown".to_string(),
                status_message: "Booking confirmed — check your email/SMS for PNR".to_string(),
            });
        }

        if result.starts_with("FAILED:") {
            let reason = result.strip_prefix("FAILED:").unwrap_or("Unknown").to_string();
            return Err(anyhow!("Payment failed: {}", reason));
        }

        if start.elapsed() > timeout {
            return Err(anyhow!(
                "Timeout ({:?}) waiting for payment confirmation. Check the browser for status.",
                timeout,
            ));
        }

        tokio::time::sleep(Duration::from_millis(1000)).await;
    }
}
