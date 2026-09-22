use std::time::Duration;

use anyhow::{anyhow, Result};
use chromiumoxide::Page;
use bookkar_common::{Quota, TrainClass};
use tracing::{debug, info};

use crate::browser::page_state::{
    click_element, eval_js, js_quote, select_dropdown, wait_for_element,
};
use crate::irctc::selectors;

/// Fill the train search form with journey details.
///
/// This should be called BEFORE the Tatkal window opens, so everything
/// is pre-filled and ready. When the window opens, we just click "Find Trains".
pub async fn fill_search_form(
    page: &Page,
    from: &str,
    to: &str,
    date: &str,
    class: &TrainClass,
    quota: &Quota,
) -> Result<()> {
    info!("Filling search form...");

    // Fill "From" station with autocomplete handling
    fill_station(page, selectors::search::FROM_STATION_INPUT, from).await?;
    tokio::time::sleep(Duration::from_millis(500)).await;

    // Fill "To" station with autocomplete handling
    fill_station(page, selectors::search::TO_STATION_INPUT, to).await?;
    tokio::time::sleep(Duration::from_millis(500)).await;

    // Fill date
    info!("Setting journey date: {}", date);
    let date_js = format!(
        r#"
        (() => {{
            const input = document.querySelector({});
            if (!input) return 'NOT_FOUND';
            // Clear and set the date via Angular's model
            const nativeInputValueSetter = Object.getOwnPropertyDescriptor(
                window.HTMLInputElement.prototype, 'value'
            )?.set;
            if (nativeInputValueSetter) {{
                nativeInputValueSetter.call(input, {});
            }} else {{
                input.value = {};
            }}
            input.dispatchEvent(new Event('input', {{ bubbles: true }}));
            input.dispatchEvent(new Event('change', {{ bubbles: true }}));
            // Close any date picker that opened
            document.body.click();
            return 'OK';
        }})()
        "#,
        js_quote(selectors::search::DATE_INPUT),
        js_quote(date),
        js_quote(date)
    );
    eval_js(page, &date_js).await?;
    tokio::time::sleep(Duration::from_millis(300)).await;

    // Select class
    info!("Selecting class: {}", class);
    select_dropdown(page, selectors::search::CLASS_DROPDOWN, class.irctc_value()).await?;
    tokio::time::sleep(Duration::from_millis(300)).await;

    // Select quota (TATKAL or PREMIUM_TATKAL)
    info!("Selecting quota: {}", quota);
    select_dropdown(page, selectors::search::QUOTA_DROPDOWN, quota.irctc_value()).await?;
    tokio::time::sleep(Duration::from_millis(300)).await;

    info!("✅ Search form pre-filled and ready");
    Ok(())
}

/// Fill a station input with autocomplete selection.
/// Types the station code and selects the first matching suggestion.
async fn fill_station(page: &Page, selector: &str, station_code: &str) -> Result<()> {
    debug!("Filling station input: {} = {}", selector, station_code);

    // Click to focus the input
    click_element(page, selector).await?;
    tokio::time::sleep(Duration::from_millis(200)).await;

    // Clear existing text and type the station code
    let clear_js = format!(
        r#"
        (() => {{
            const el = document.querySelector({});
            if (el) {{
                el.value = '';
                el.dispatchEvent(new Event('input', {{ bubbles: true }}));
            }}
        }})()
        "#,
        js_quote(selector)
    );
    eval_js(page, &clear_js).await?;

    // Type station code to trigger autocomplete
    let element = wait_for_element(page, selector, Duration::from_secs(5)).await?;
    element.type_str(station_code).await?;
    tokio::time::sleep(Duration::from_millis(800)).await;

    // Click the first autocomplete suggestion
    let select_js = format!(
        r#"
        (() => {{
            const items = document.querySelectorAll({});
            if (items.length > 0) {{
                items[0].click();
                return 'SELECTED';
            }}
            // Fallback: try clicking any visible dropdown item
            const fallback = document.querySelector('.ui-autocomplete-list-item, .mat-option, li.ui-autocomplete-item');
            if (fallback) {{
                fallback.click();
                return 'SELECTED_FALLBACK';
            }}
            return 'NO_SUGGESTIONS';
        }})()
        "#,
        js_quote(selectors::search::AUTOCOMPLETE_OPTION)
    );

    let result = eval_js(page, &select_js).await?;
    if result == "NO_SUGGESTIONS" {
        // Try pressing Enter instead — sometimes that works
        element.press_key("Enter").await?;
        debug!("No autocomplete suggestions found, pressed Enter instead");
    } else {
        debug!("Station selected: {}", result);
    }

    Ok(())
}

/// Click the "Find Trains" button.
/// This is called at the exact Tatkal window opening time.
pub async fn click_search(page: &Page) -> Result<()> {
    info!("🚀 Clicking 'Find Trains'...");
    click_element(page, selectors::search::SEARCH_BUTTON).await?;
    Ok(())
}

/// Wait for search results to load and return the number of trains found.
pub async fn wait_for_results(page: &Page, timeout: Duration) -> Result<u32> {
    info!("Waiting for train results...");
    let start = std::time::Instant::now();

    loop {
        let count_js = format!(
            "document.querySelectorAll({}).length",
            js_quote(selectors::search::TRAIN_ROW)
        );

        let result = page.evaluate(count_js.as_str()).await?;
        let count = result.into_value::<u32>().unwrap_or(0);

        if count > 0 {
            info!("Found {} train(s)", count);
            return Ok(count);
        }

        // Check for errors (e.g., "No trains found")
        let error_js = r#"
            (() => {
                const noTrain = document.querySelector('.alert-danger, .no-train-found');
                return noTrain ? noTrain.innerText.trim() : '';
            })()
        "#;

        let error = eval_js(page, error_js).await?;
        if !error.is_empty() {
            return Err(anyhow!("Search error: {}", error));
        }

        if start.elapsed() > timeout {
            return Err(anyhow!("Timeout waiting for search results"));
        }

        tokio::time::sleep(Duration::from_millis(300)).await;
    }
}

/// Select a specific train by number, or the first available train.
/// Then click "Book Now".
pub async fn select_train(page: &Page, train_number: Option<&str>) -> Result<String> {
    match train_number {
        Some(number) => {
            info!("Looking for train {}...", number);
            let select_js = format!(
                r#"
                (() => {{
                    const rows = document.querySelectorAll({});
                    for (const row of rows) {{
                        const heading = row.querySelector({});
                        if (heading && heading.innerText.includes({})) {{
                            // Click the availability link for this train
                            const avail = row.querySelector({});
                            if (avail) {{
                                avail.click();
                                return heading.innerText.trim();
                            }}
                        }}
                    }}
                    return 'NOT_FOUND';
                }})()
                "#,
                js_quote(selectors::search::TRAIN_ROW),
                js_quote(selectors::search::TRAIN_NUMBER),
                js_quote(number),
                js_quote(selectors::search::AVAILABILITY_LINK),
            );

            let result = eval_js(page, &select_js).await?;
            if result == "NOT_FOUND" {
                return Err(anyhow!("Train {} not found in results", number));
            }

            info!("Selected train: {}", result);
            tokio::time::sleep(Duration::from_millis(500)).await;

            // Click "Book Now"
            click_element(page, selectors::search::BOOK_NOW_BUTTON).await?;
            Ok(result)
        }
        None => {
            info!("Selecting first available train...");
            let select_js = format!(
                r#"
                (() => {{
                    const rows = document.querySelectorAll({});
                    if (rows.length === 0) return 'NONE';
                    const first = rows[0];
                    const heading = first.querySelector({});
                    const avail = first.querySelector({});
                    if (avail) avail.click();
                    return heading ? heading.innerText.trim() : 'UNKNOWN';
                }})()
                "#,
                js_quote(selectors::search::TRAIN_ROW),
                js_quote(selectors::search::TRAIN_NUMBER),
                js_quote(selectors::search::AVAILABILITY_LINK),
            );

            let result = eval_js(page, &select_js).await?;
            if result == "NONE" {
                return Err(anyhow!("No trains available"));
            }

            info!("Selected train: {}", result);
            tokio::time::sleep(Duration::from_millis(500)).await;

            // Click "Book Now"
            click_element(page, selectors::search::BOOK_NOW_BUTTON).await?;
            Ok(result)
        }
    }
}
