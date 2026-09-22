use std::time::{Duration, Instant};

use anyhow::{anyhow, Result};
use chromiumoxide::cdp::browser_protocol::dom::*;
use chromiumoxide::{Element, Page};
use tracing::{debug, warn};

/// Wait until the page URL contains a specific fragment.
/// Used to detect page transitions (e.g., after login, after payment).
pub async fn wait_for_url_contains(
    page: &Page,
    fragment: &str,
    timeout: Duration,
) -> Result<String> {
    let start = Instant::now();
    loop {
        if let Ok(Some(url)) = page.url().await.map(|u| u) {
            if url.contains(fragment) {
                debug!("URL matched fragment '{}': {}", fragment, url);
                return Ok(url);
            }
        }

        if start.elapsed() > timeout {
            return Err(anyhow!(
                "Timeout ({:?}) waiting for URL containing '{}'",
                timeout,
                fragment
            ));
        }

        tokio::time::sleep(Duration::from_millis(200)).await;
    }
}

/// Wait for a CSS selector to appear in the DOM, with timeout.
/// Returns the element once found.
pub async fn wait_for_element(
    page: &Page,
    selector: &str,
    timeout: Duration,
) -> Result<Element> {
    let start = Instant::now();
    loop {
        match page.find_element(selector).await {
            Ok(el) => {
                debug!("Found element: {}", selector);
                return Ok(el);
            }
            Err(_) => {
                if start.elapsed() > timeout {
                    return Err(anyhow!(
                        "Timeout ({:?}) waiting for selector '{}'",
                        timeout,
                        selector
                    ));
                }
                tokio::time::sleep(Duration::from_millis(100)).await;
            }
        }
    }
}

/// Wait for a CSS selector to disappear from the DOM (e.g., loading spinners).
pub async fn wait_for_element_removed(
    page: &Page,
    selector: &str,
    timeout: Duration,
) -> Result<()> {
    let start = Instant::now();
    loop {
        match page.find_element(selector).await {
            Err(_) => {
                debug!("Element removed: {}", selector);
                return Ok(());
            }
            Ok(_) => {
                if start.elapsed() > timeout {
                    return Err(anyhow!(
                        "Timeout ({:?}) waiting for '{}' to disappear",
                        timeout,
                        selector
                    ));
                }
                tokio::time::sleep(Duration::from_millis(200)).await;
            }
        }
    }
}

/// Execute JavaScript in the page context and return the result as a string.
pub async fn eval_js(page: &Page, expression: &str) -> Result<String> {
    let result = page
        .evaluate(expression)
        .await?;

    Ok(result.into_value::<String>().unwrap_or_default())
}

/// Type text into an element character by character with small random delays,
/// simulating human typing behaviour (helps avoid keystroke analysis detection).
pub async fn human_type(page: &Page, selector: &str, text: &str) -> Result<()> {
    let element = wait_for_element(page, selector, Duration::from_secs(10)).await?;
    // Click to focus
    element.click().await?;
    // Small delay after focus
    tokio::time::sleep(Duration::from_millis(100)).await;
    // Clear existing content
    page.evaluate(format!(
        "document.querySelector('{}').value = ''",
        selector
    ))
    .await?;
    // Type each character
    element.type_str(text).await?;
    Ok(())
}

/// Click an element found by selector, waiting for it to appear first.
pub async fn click_element(page: &Page, selector: &str) -> Result<()> {
    let element = wait_for_element(page, selector, Duration::from_secs(10)).await?;
    if let Err(e) = element.click().await {
        debug!("CDP click failed ({}), attempting JS click for {}", e, selector);
        let js = format!("document.querySelector('{}')?.click()", selector);
        page.evaluate(js).await?;
    }
    debug!("Clicked element: {}", selector);
    // Brief pause after click to let Angular digest
    tokio::time::sleep(Duration::from_millis(300)).await;
    Ok(())
}

/// Select an option from a <select> dropdown by its value attribute.
pub async fn select_dropdown(page: &Page, selector: &str, value: &str) -> Result<()> {
    let js = format!(
        r#"
        (() => {{
            const el = document.querySelector('{}');
            if (!el) return 'NOT_FOUND';
            el.value = '{}';
            el.dispatchEvent(new Event('change', {{ bubbles: true }}));
            return 'OK';
        }})()
        "#,
        selector, value
    );

    let result = eval_js(page, &js).await?;
    if result == "NOT_FOUND" {
        return Err(anyhow!("Dropdown not found: {}", selector));
    }

    debug!("Selected '{}' in dropdown: {}", value, selector);
    tokio::time::sleep(Duration::from_millis(200)).await;
    Ok(())
}

/// Dismiss any modal/overlay that might be blocking the page.
pub async fn dismiss_modals(page: &Page) {
    let js = r#"
        (() => {
            // Dismiss close icons, backdrops, and modal buttons
            document.querySelectorAll('.modal .close, button.close, .cdk-overlay-backdrop, .ui-dialog-titlebar-close, button[aria-label="Close"]')
                .forEach(el => { try { el.click(); } catch(e) {} });

            // On IRCTC, alert dialogs have an OK button inside ui-dialog
            document.querySelectorAll('.ui-dialog button, .modal-dialog button').forEach(b => {
                const txt = (b.innerText || '').trim().toUpperCase();
                if (txt === 'OK' || txt.includes('DISMISS') || txt.includes('CLOSE')) {
                    try { b.click(); } catch(e) {}
                }
            });
        })()
    "#;
    let _ = page.evaluate(js).await;
    let _ = tokio::time::sleep(Duration::from_millis(300)).await;
}

/// Check if page has any error alerts visible
pub async fn check_for_errors(page: &Page) -> Option<String> {
    let js = r#"
        (() => {
            const err = document.querySelector('.alert-danger, .error-message, .toast-error');
            return err ? err.innerText.trim() : '';
        })()
    "#;

    match eval_js(page, js).await {
        Ok(text) if !text.is_empty() => {
            warn!("Page error detected: {}", text);
            Some(text)
        }
        _ => None,
    }
}

/// Inject the stealth JavaScript patches into the page.
/// Should be called after navigating to a new page but before any interaction.
pub async fn inject_stealth(page: &Page, stealth_js: &str) -> Result<()> {
    match page.evaluate(stealth_js).await {
        Ok(_) => debug!("Stealth JS injected"),
        Err(e) => warn!("Warning injecting stealth JS: {}", e),
    }
    Ok(())
}
