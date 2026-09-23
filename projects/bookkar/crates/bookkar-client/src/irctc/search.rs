use std::time::Duration;

use anyhow::{anyhow, Result};
use chromiumoxide::Page;
use bookkar_common::{Quota, TrainClass};
use tracing::{debug, info};

use crate::browser::page_state::{
    click_element, eval_js, js_quote, wait_for_element,
};
use crate::irctc::selectors;

/// Fill the train search form with journey details.
///
/// This should be called BEFORE the Tatkal window opens, so everything
/// is pre-filled and ready. When the window opens, we just click "Find Trains".
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
    fill_station(page, true, from).await?;
    tokio::time::sleep(Duration::from_millis(500)).await;

    // Fill "To" station with autocomplete handling
    fill_station(page, false, to).await?;
    tokio::time::sleep(Duration::from_millis(500)).await;

    // Fill date
    info!("Setting journey date: {}", date);
    let date_js = format!(
        r#"
        (() => {{
            const input = document.querySelector('input[formcontrolname="journeyDate"], p-calendar input, input[placeholder*="Date" i]');
            if (input) {{
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
                document.body.click();
                return 'OK';
            }}

            // On beta site: if there is a calendar date button, click it and try to find matching date cell
            const dateBtn = document.querySelector('[role="button"][aria-label="Select travel date"]');
            if (dateBtn) {{
                dateBtn.click();
                // Extract day number (e.g. "25" from "25-09-2026" or "2026-09-25")
                const dayMatch = ({}).match(/^(\d{{1,2}})|-(\d{{1,2}})-/);
                const day = dayMatch ? (dayMatch[1] || dayMatch[2]) : '';
                if (day) {{
                    const cells = Array.from(document.querySelectorAll('table.ui-datepicker-calendar td:not(.ui-state-disabled) a, table.ui-datepicker-calendar td:not(.ui-state-disabled) span'));
                    const target = cells.find(c => c.innerText.trim() === String(parseInt(day, 10)));
                    if (target) {{
                        target.click();
                        return 'CLICKED_CALENDAR_DAY_' + day;
                    }}
                }}
                // Close calendar if open
                document.body.click();
            }}
            return 'FALLBACK';
        }})()
        "#,
        js_quote(date),
        js_quote(date),
        js_quote(date)
    );
    let _ = eval_js(page, &date_js).await;
    tokio::time::sleep(Duration::from_millis(300)).await;

    // Select quota (TATKAL or PREMIUM_TATKAL)
    info!("Selecting quota: {}", quota);
    let quota_str = quota.to_string().to_uppercase();
    let quota_irctc = quota.irctc_value().to_uppercase();
    let select_quota_js = format!(
        r#"
        (() => {{
            // 1. Try combobox on beta site
            const quotaCombobox = document.querySelector('[role="combobox"][aria-label="Quota"]');
            if (quotaCombobox) {{
                quotaCombobox.click();
                const options = Array.from(document.querySelectorAll('.custom-dropdown-panel .custom-option, [role="option"].custom-option'));
                const match = options.find(o => {{
                    const t = (o.innerText || '').toUpperCase();
                    return t.includes({}) || t.includes({});
                }});
                if (match) {{
                    match.click();
                    return 'CLICKED_COMBOBOX_QUOTA';
                }}
            }}

            // 2. Try legacy select#journeyQuota
            const select = document.querySelector('select#journeyQuota');
            if (select) {{
                select.value = {};
                select.dispatchEvent(new Event('change', {{ bubbles: true }}));
                return 'SET_LEGACY_SELECT';
            }}
            return 'QUOTA_NOT_CHANGED';
        }})()
        "#,
        js_quote(&quota_str),
        js_quote(&quota_irctc),
        js_quote(quota.irctc_value())
    );
    let quota_res = eval_js(page, &select_quota_js).await.unwrap_or_default();
    debug!("Quota selection result: {}", quota_res);
    tokio::time::sleep(Duration::from_millis(300)).await;

    // Select class (if dropdown exists on legacy site; beta site selects class in train card)
    let select_class_js = format!(
        r#"
        (() => {{
            const select = document.querySelector('select#journeyClass');
            if (select) {{
                select.value = {};
                select.dispatchEvent(new Event('change', {{ bubbles: true }}));
                return 'SET_CLASS';
            }}
            return 'NO_CLASS_DROPDOWN';
        }})()
        "#,
        js_quote(class.irctc_value())
    );
    let _ = eval_js(page, &select_class_js).await;
    tokio::time::sleep(Duration::from_millis(300)).await;

    info!("✅ Search form pre-filled and ready");
    Ok(())
}

/// Fill a station input with autocomplete selection.
/// Types the station code and selects the matching suggestion.
async fn fill_station(page: &Page, is_from: bool, station_code: &str) -> Result<()> {
    let label = if is_from { "From station" } else { "To station" };
    let input_selector = if is_from {
        selectors::search::FROM_STATION_INPUT
    } else {
        selectors::search::TO_STATION_INPUT
    };

    debug!("Filling station: label='{}', code='{}'", label, station_code);

    let type_js = format!(
        r#"
        (() => {{
            // 1. Try to find and click combobox on beta site
            const combobox = document.querySelector('[role="combobox"][aria-label="{}"]');
            if (combobox) {{
                combobox.click();
            }}

            // 2. Find input (new beta input or legacy input)
            let input = document.querySelector('{}');
            if (!input) {{
                input = document.querySelector('input.station-search-input, input[placeholder*="search" i], input[type="text"]:focus');
            }}
            if (input) {{
                input.focus();
                input.value = {};
                input.dispatchEvent(new Event('input', {{ bubbles: true }}));
                input.dispatchEvent(new Event('change', {{ bubbles: true }}));
                return 'TYPED';
            }}
            return 'INPUT_NOT_FOUND';
        }})()
        "#,
        label,
        input_selector,
        js_quote(station_code)
    );

    let res = eval_js(page, &type_js).await?;
    if res == "INPUT_NOT_FOUND" {
        if let Ok(el) = wait_for_element(page, input_selector, Duration::from_secs(3)).await {
            let _ = el.type_str(station_code).await;
        }
    }

    tokio::time::sleep(Duration::from_millis(800)).await;

    // Click matching autocomplete suggestion
    let select_js = format!(
        r#"
        (() => {{
            const options = Array.from(document.querySelectorAll('{}'));
            const codeUpper = {}.toUpperCase();
            const match = options.find(o => (o.innerText || '').toUpperCase().includes(codeUpper));
            if (match) {{
                match.click();
                return 'SELECTED_MATCH: ' + match.innerText.trim().replace(/\n+/g, ' ');
            }}
            if (options.length > 0) {{
                options[0].click();
                return 'SELECTED_FIRST: ' + options[0].innerText.trim().replace(/\n+/g, ' ');
            }}
            return 'NO_SUGGESTIONS';
        }})()
        "#,
        selectors::search::AUTOCOMPLETE_OPTION,
        js_quote(station_code)
    );

    let result = eval_js(page, &select_js).await?;
    debug!("Station selected: {}", result);

    Ok(())
}

/// Click the "Find Trains" / "Search Trains" button.
/// This is called at the exact Tatkal window opening time.
pub async fn click_search(page: &Page) -> Result<()> {
    info!("🚀 Clicking 'Search Trains'...");
    let click_js = r#"
        (() => {
            const btn = document.querySelector('button.search-btn, button.search_btn, button[type="submit"].search-btn, form button[type="submit"]');
            if (btn) {
                btn.click();
                return 'CLICKED: ' + btn.className;
            }
            return 'NOT_FOUND';
        })()
    "#;
    let res = eval_js(page, click_js).await?;
    if res == "NOT_FOUND" {
        click_element(page, selectors::search::SEARCH_BUTTON).await?;
    }
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
    let result = match train_number {
        Some(number) => {
            info!("Looking for train {}...", number);
            let select_js = format!(
                r#"
                (() => {{
                    const rows = document.querySelectorAll({});
                    for (const row of rows) {{
                        const text = row.innerText || '';
                        if (text.includes({})) {{
                            // Click availability button or class card
                            const avail = row.querySelector({});
                            if (avail) {{
                                avail.click();
                                return text.substring(0, 50).trim().replace(/\n+/g, ' ');
                            }}
                        }}
                    }}
                    return 'NOT_FOUND';
                }})()
                "#,
                js_quote(selectors::search::TRAIN_ROW),
                js_quote(number),
                js_quote(selectors::search::AVAILABILITY_LINK),
            );

            let res = eval_js(page, &select_js).await?;
            if res == "NOT_FOUND" {
                return Err(anyhow!("Train {} not found in results", number));
            }
            res
        }
        None => {
            info!("Selecting first available train...");
            let select_js = format!(
                r#"
                (() => {{
                    const rows = document.querySelectorAll({});
                    for (const row of rows) {{
                        const text = row.innerText || '';
                        if (!text.includes('TRAIN DEPARTED') && !text.includes('CANCELLED')) {{
                            const avail = row.querySelector({});
                            if (avail) {{
                                avail.click();
                                return text.substring(0, 50).trim().replace(/\n+/g, ' ');
                            }}
                        }}
                    }}
                    // Fallback to first row
                    if (rows.length > 0) {{
                        const avail = rows[0].querySelector({});
                        if (avail) avail.click();
                        return rows[0].innerText.substring(0, 50).trim().replace(/\n+/g, ' ');
                    }}
                    return 'NONE';
                }})()
                "#,
                js_quote(selectors::search::TRAIN_ROW),
                js_quote(selectors::search::AVAILABILITY_LINK),
                js_quote(selectors::search::AVAILABILITY_LINK),
            );

            let res = eval_js(page, &select_js).await?;
            if res == "NONE" {
                return Err(anyhow!("No trains available in search results"));
            }
            res
        }
    };

    info!("Selected train: {}", result);
    tokio::time::sleep(Duration::from_millis(800)).await;

    // Click "Book Now"
    info!("Clicking 'Book Now'...");
    let click_book_js = r#"
        (() => {
            const allButtons = Array.from(document.querySelectorAll('button, .btn, [role="button"], a.btn'));
            const bookBtn = allButtons.find(b => {
                const t = (b.innerText || '').trim().toUpperCase();
                return t === 'BOOK NOW' || t === 'BOOK' || t.includes('BOOK NOW');
            });
            if (bookBtn) {
                bookBtn.click();
                return 'CLICKED_BOOK_NOW';
            }
            return 'NOT_FOUND';
        })()
    "#;

    let book_res = eval_js(page, click_book_js).await.unwrap_or_default();
    if book_res == "NOT_FOUND" {
        let _ = click_element(page, selectors::search::BOOK_NOW_BUTTON).await;
    }

    Ok(result)
}
