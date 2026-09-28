use std::io::Write;
use std::time::Duration;
use anyhow::Result;
use bookkar_client::browser::launcher::{launch_stealth_browser, STEALTH_JS};
use bookkar_client::browser::page_state::inject_stealth;

#[tokio::main]
async fn main() -> Result<()> {
    dotenvy::dotenv().ok();

    println!("Launching browser to inspect eticket portal...");
    let (browser, _handler) = launch_stealth_browser().await?;
    let page = browser.new_page("about:blank").await?;
    inject_stealth(&page, STEALTH_JS).await?;

    let url = std::env::var("IRCTC_URL")
        .unwrap_or_else(|_| "https://www.irctc.co.in/eticket/train-search".to_string());
    println!("Navigating to {}...", url);
    page.goto(&url).await?;

    println!("Waiting 6 seconds for page to hydrate...");
    tokio::time::sleep(Duration::from_secs(6)).await;

    // Inspect all form fields inside app-jp-input form
    let inspect_search_js = r#"
        (() => {
            const form = document.querySelector('app-jp-input form');
            if (!form) return JSON.stringify({ error: 'form not found' });

            const fields = Array.from(form.querySelectorAll('[role="combobox"], [role="button"], input, select, .field-box, .field-inner'))
                .map(el => ({
                    tag: el.tagName.toLowerCase(),
                    role: el.getAttribute('role'),
                    ariaLabel: el.getAttribute('aria-label'),
                    classes: el.className,
                    id: el.id || null,
                    text: (el.innerText || '').trim().replace(/\n+/g, ' | ').substring(0, 100),
                    inputs: Array.from(el.querySelectorAll('input')).map(i => ({
                        id: i.id,
                        placeholder: i.placeholder,
                        formcontrolname: i.getAttribute('formcontrolname'),
                        type: i.type,
                        classes: i.className
                    }))
                }));

            return JSON.stringify({
                fieldsCount: fields.length,
                fields
            }, null, 2);
        })()
    "#;

    let res = page.evaluate(inspect_search_js).await?;
    let data = res.into_value::<String>().unwrap_or_default();
    // Test Quota combobox
    println!("Clicking 'Quota' combobox...");
    let click_quota_js = r#"
        (() => {
            const el = document.querySelector('[role="combobox"][aria-label="Quota"]');
            if (!el) return 'NOT_FOUND';
            el.click();
            return 'CLICKED';
        })()
    "#;
    let _ = page.evaluate(click_quota_js).await?;
    tokio::time::sleep(Duration::from_secs(1)).await;

    let check_quota_js = r#"
        (() => {
            const options = Array.from(document.querySelectorAll('.custom-dropdown-panel div, [role="listbox"] div, [role="option"], .quota-item, .custom-option'))
                .filter(el => el.offsetParent !== null)
                .map(el => ({
                    tag: el.tagName.toLowerCase(),
                    role: el.getAttribute('role'),
                    classes: el.className,
                    text: el.innerText.trim()
                }));
            return JSON.stringify(options, null, 2);
        })()
    "#;
    let quota_res = page.evaluate(check_quota_js).await?;
    println!("Quota options:\n{}", quota_res.into_value::<String>().unwrap_or_default());

    // Test Date picker button
    println!("Clicking Date button...");
    let click_date_js = r#"
        (() => {
            const el = document.querySelector('[role="button"][aria-label="Select travel date"], .field-box[aria-label*="date" i]');
            if (!el) return 'NOT_FOUND';
            el.click();
            return 'CLICKED';
        })()
    "#;
    let _ = page.evaluate(click_date_js).await?;
    tokio::time::sleep(Duration::from_secs(1)).await;

    let check_date_js = r#"
        (() => {
            const datepicker = Array.from(document.querySelectorAll('p-calendar, .custom-calendar, .calendar-panel, .ui-datepicker, table.ui-datepicker-calendar, .calendar-container'))
                .map(el => ({
                    tag: el.tagName.toLowerCase(),
                    classes: el.className,
                    text: (el.innerText || '').substring(0, 100)
                }));
            return JSON.stringify(datepicker, null, 2);
        })()
    "#;
    let date_res = page.evaluate(check_date_js).await?;
    println!("Date picker elements:\n{}", date_res.into_value::<String>().unwrap_or_default());


    // Check DOM after clicking
    let check_after_click_js = r#"
        (() => {
            const inputs = Array.from(document.querySelectorAll('input:focus, input[type="text"], input.ui-inputtext, p-autocomplete input, app-jp-input input, .cdk-overlay-container input, p-dialog input, input[placeholder*="station" i], input[placeholder*="source" i]'))
                .map(i => ({
                    id: i.id,
                    placeholder: i.placeholder,
                    classes: i.className,
                    visible: i.offsetParent !== null,
                    parent: i.parentElement ? i.parentElement.tagName : null
                }));

            const overlays = Array.from(document.querySelectorAll('.cdk-overlay-container, p-autocomplete, .ui-autocomplete-panel, .dropdown-menu, .station-list, .p-dialog, div[role="listbox"], [role="option"]'))
                .map(o => ({
                    tag: o.tagName.toLowerCase(),
                    classes: o.className,
                    text: (o.innerText || '').trim().substring(0, 200)
                }));

            return JSON.stringify({ inputs, overlays }, null, 2);
        })()
    "#;
    let res = page.evaluate(check_after_click_js).await?;
    println!("After clicking From station:\n{}", res.into_value::<String>().unwrap_or_default());

    // Try typing in active element / newly appeared input
    println!("Trying to type 'NDLS' into station search...");
    let type_station_js = r#"
        (() => {
            const input = document.activeElement && document.activeElement.tagName === 'INPUT' 
                ? document.activeElement 
                : document.querySelector('app-jp-input input, .cdk-overlay-container input, input[placeholder*="station" i], input[placeholder*="source" i], input[type="text"]');
            if (input) {
                input.value = 'NDLS';
                input.dispatchEvent(new Event('input', { bubbles: true }));
                input.dispatchEvent(new Event('change', { bubbles: true }));
                return 'TYPED_INTO: ' + input.outerHTML.substring(0, 150);
            }
            return 'NO_ACTIVE_INPUT';
        })()
    "#;
    let res = page.evaluate(type_station_js).await?;
    println!("Type station result: {}", res.into_value::<String>().unwrap_or_default());
    tokio::time::sleep(Duration::from_secs(2)).await;

    // Check for autocomplete suggestions
    let check_suggestions_js = r#"
        (() => {
            const suggestions = Array.from(document.querySelectorAll('[role="option"], .station-item, .suggestion-item, li, p-autocomplete li'))
                .filter(el => el.offsetParent !== null && (el.innerText || '').toUpperCase().includes('NDLS'))
                .map(el => ({
                    tag: el.tagName.toLowerCase(),
                    classes: el.className,
                    role: el.getAttribute('role'),
                    text: el.innerText.trim()
                }));
            return JSON.stringify(suggestions, null, 2);
        })()
    "#;
    let res = page.evaluate(check_suggestions_js).await?;
    println!("Suggestions:\n{}", res.into_value::<String>().unwrap_or_default());

    // ── LOGIN FLOW ─────────────────────────────────────────────────────────────
    println!("\n=== LOGIN FLOW ===");
    println!("\n╔══════════════════════════════════════════════════════════════╗");
    println!(  "║  CHROME WINDOW OPENED — PLEASE LOOK AT YOUR SCREEN          ║");
    println!(  "║  The automation will fill your credentials and click LOGIN.  ║");
    println!(  "║  If a CAPTCHA appears, solve it in the Chrome window.        ║");
    println!(  "║  If OTP is requested, enter it in the Chrome window.         ║");
    println!(  "║  The program will wait up to 5 minutes for login.            ║");
    println!(  "╚══════════════════════════════════════════════════════════════╝\n");

    // Click login trigger
    let click_login_js = r#"
        (() => {
            const btn = document.querySelector('button.btn-login, a.loginText, button[aria-label*="Login"]');
            if (!btn) return 'LOGIN_TRIGGER_NOT_FOUND';
            btn.click();
            return 'LOGIN_TRIGGER_CLICKED: ' + btn.className;
        })()
    "#;
    let res = page.evaluate(click_login_js).await?;
    println!("{}", res.into_value::<String>().unwrap_or_default());
    tokio::time::sleep(Duration::from_secs(2)).await;

    let username = std::env::var("IRCTC_USERNAME").unwrap_or_default();
    let password = std::env::var("IRCTC_PASSWORD").unwrap_or_default();

    // Fill credentials via JS (Angular-compatible native value setter)
    println!("Filling credentials for '{}'...", username);
    let fill_creds_js = format!(
        r#"
        (() => {{
            const uInput = document.querySelector('input#username, input[formcontrolname="userid"]');
            const pInput = document.querySelector('input#password, input[formcontrolname="password"]');
            if (!uInput || !pInput) {{
                return 'INPUTS_NOT_FOUND: u=' + !!uInput + ', p=' + !!pInput;
            }}
            const nativeSetter = Object.getOwnPropertyDescriptor(window.HTMLInputElement.prototype, 'value')?.set;
            if (nativeSetter) {{
                nativeSetter.call(uInput, {});
                nativeSetter.call(pInput, {});
            }} else {{
                uInput.value = {};
                pInput.value = {};
            }}
            uInput.dispatchEvent(new Event('input', {{ bubbles: true }}));
            uInput.dispatchEvent(new Event('change', {{ bubbles: true }}));
            uInput.dispatchEvent(new Event('blur', {{ bubbles: true }}));
            pInput.dispatchEvent(new Event('input', {{ bubbles: true }}));
            pInput.dispatchEvent(new Event('change', {{ bubbles: true }}));
            pInput.dispatchEvent(new Event('blur', {{ bubbles: true }}));
            return 'FILLED';
        }})()
        "#,
        serde_json::to_string(&username)?,
        serde_json::to_string(&password)?,
        serde_json::to_string(&username)?,
        serde_json::to_string(&password)?
    );
    let res = page.evaluate(fill_creds_js.as_str()).await?;
    println!("Credential fill: {}", res.into_value::<String>().unwrap_or_default());
    tokio::time::sleep(Duration::from_millis(600)).await;

    // Check submit button state in dialog
    let check_submit_js = r#"
        (() => {
            const modal = document.querySelector('#newLogin, .login-dialog, app-login');
            const container = modal || document;
            const submitBtn = container.querySelector('button[type="submit"].btn-login, button.btn-action.btn-login[type="submit"], button[type="submit"]');
            if (!submitBtn) return 'SUBMIT_BTN_NOT_FOUND';
            return JSON.stringify({
                tag: submitBtn.tagName,
                classes: submitBtn.className,
                text: submitBtn.innerText.trim(),
                disabled: submitBtn.disabled,
                disabledStateClass: submitBtn.classList.contains('disabled-state')
            });
        })()
    "#;
    let res = page.evaluate(check_submit_js).await?;
    println!("Submit button state before click:\n{}", res.into_value::<String>().unwrap_or_default());

    // ── ACTUALLY SUBMIT LOGIN ─────────────────────────────────────────────────
    println!("\n=== SUBMITTING LOGIN ===");
    let click_submit_js = r#"
        (() => {
            const modal = document.querySelector('#newLogin, .login-dialog, app-login, div[role="dialog"]');
            const container = modal || document;
            const btn = container.querySelector('button.btn-action.btn-login, button[type="submit"].btn-login, button[type="submit"]');
            if (btn) {
                if (btn.disabled) { btn.disabled = false; btn.removeAttribute('disabled'); }
                btn.click();
                return 'SUBMIT_CLICKED: ' + btn.className + ' | ' + btn.innerText.trim();
            }
            // Fallback: any visible LOGIN button in the page
            const all = Array.from(document.querySelectorAll('button')).filter(b =>
                b.offsetParent !== null && (b.innerText || '').trim().toUpperCase() === 'LOGIN'
            );
            if (all.length > 0) { all[all.length - 1].click(); return 'FALLBACK_CLICKED'; }
            return 'NOT_FOUND';
        })()
    "#;
    let submit_res = page.evaluate(click_submit_js).await?;
    println!("Submit click result: {}", submit_res.into_value::<String>().unwrap_or_default());

    // Wait for login to complete (up to 5 minutes — user may need to enter OTP/CAPTCHA)
    println!("⏳ Waiting for login (up to 5 min) — handle CAPTCHA/OTP in the Chrome window if prompted...");
    let _ = std::io::stdout().flush();
    let login_start = std::time::Instant::now();
    loop {
        tokio::time::sleep(Duration::from_secs(1)).await;
        let state_js = r#"
            (() => {
                // Check for logout link / user name badge
                const logout = Array.from(document.querySelectorAll('a, button, span, strong'))
                    .find(el => (el.innerText || '').trim().toUpperCase().includes('LOGOUT'));
                if (logout) return 'LOGGED_IN:logout_found';
                if (sessionStorage.getItem('token') || sessionStorage.getItem('userName') || sessionStorage.getItem('userId'))
                    return 'LOGGED_IN:storage';
                const modal = document.querySelector('.login-dialog, div[role="dialog"], app-login');
                const modalVisible = modal && modal.offsetParent !== null;
                if (!modalVisible && window.location.href.includes('train-search')) return 'LOGGED_IN:modal_closed';
                return 'WAITING';
            })()
        "#;
        let state = page.evaluate(state_js).await?;
        let state_str = state.into_value::<String>().unwrap_or_default();
        println!("Login state: {}", state_str);
        if state_str.starts_with("LOGGED_IN") { println!("✅ Logged in!"); break; }
        if login_start.elapsed() > Duration::from_secs(300) {
            println!("⚠️  Login timed out after 5 minutes — proceeding anyway");
            break;
        }
    }
    tokio::time::sleep(Duration::from_secs(1)).await;

    // Now let's test Station Selection and Search Trains
    println!("\n=== TESTING SEARCH TRAINS ON NEW BETA SITE ===");

    // 1. Select From Station
    println!("Selecting From Station (NDLS)...");
    let select_from_js = r#"
        (() => {
            const combobox = document.querySelector('[role="combobox"][aria-label="From station"]');
            if (!combobox) return 'FROM_COMBOBOX_NOT_FOUND';
            combobox.click();
            return 'FROM_COMBOBOX_CLICKED';
        })()
    "#;
    println!("{}", page.evaluate(select_from_js).await?.into_value::<String>().unwrap_or_default());
    tokio::time::sleep(Duration::from_millis(500)).await;

    let type_from_js = r#"
        (() => {
            const input = document.querySelector('input.from-search-input, input[formcontrolname="origin"]');
            if (!input) return 'FROM_INPUT_NOT_FOUND';
            input.value = 'NDLS';
            input.dispatchEvent(new Event('input', { bubbles: true }));
            input.dispatchEvent(new Event('change', { bubbles: true }));
            return 'FROM_INPUT_TYPED';
        })()
    "#;
    println!("{}", page.evaluate(type_from_js).await?.into_value::<String>().unwrap_or_default());
    tokio::time::sleep(Duration::from_secs(1)).await;

    let click_from_option_js = r#"
        (() => {
            const options = Array.from(document.querySelectorAll('.custom-dropdown-panel .custom-option, [role="option"].custom-option'));
            const match = options.find(o => (o.innerText || '').toUpperCase().includes('NDLS'));
            if (match) {
                match.click();
                return 'FROM_OPTION_CLICKED: ' + match.innerText.trim().replace(/\n+/g, ' ');
            }
            if (options.length > 0) {
                options[0].click();
                return 'FROM_FIRST_OPTION_CLICKED: ' + options[0].innerText.trim().replace(/\n+/g, ' ');
            }
            return 'NO_FROM_OPTIONS';
        })()
    "#;
    println!("{}", page.evaluate(click_from_option_js).await?.into_value::<String>().unwrap_or_default());
    tokio::time::sleep(Duration::from_millis(500)).await;

    // 2. Select To Station
    println!("Selecting To Station (CNB)...");
    let select_to_js = r#"
        (() => {
            const combobox = document.querySelector('[role="combobox"][aria-label="To station"]');
            if (!combobox) return 'TO_COMBOBOX_NOT_FOUND';
            combobox.click();
            return 'TO_COMBOBOX_CLICKED';
        })()
    "#;
    println!("{}", page.evaluate(select_to_js).await?.into_value::<String>().unwrap_or_default());
    tokio::time::sleep(Duration::from_millis(500)).await;

    let type_to_js = r#"
        (() => {
            const input = document.querySelector('input.to-search-input, input[formcontrolname="destination"]');
            if (!input) return 'TO_INPUT_NOT_FOUND';
            input.value = 'CNB';
            input.dispatchEvent(new Event('input', { bubbles: true }));
            input.dispatchEvent(new Event('change', { bubbles: true }));
            return 'TO_INPUT_TYPED';
        })()
    "#;
    println!("{}", page.evaluate(type_to_js).await?.into_value::<String>().unwrap_or_default());
    tokio::time::sleep(Duration::from_secs(1)).await;

    let click_to_option_js = r#"
        (() => {
            const options = Array.from(document.querySelectorAll('.custom-dropdown-panel .custom-option, [role="option"].custom-option'));
            const match = options.find(o => (o.innerText || '').toUpperCase().includes('CNB'));
            if (match) {
                match.click();
                return 'TO_OPTION_CLICKED: ' + match.innerText.trim().replace(/\n+/g, ' ');
            }
            if (options.length > 0) {
                options[0].click();
                return 'TO_FIRST_OPTION_CLICKED: ' + options[0].innerText.trim().replace(/\n+/g, ' ');
            }
            return 'NO_TO_OPTIONS';
        })()
    "#;
    println!("{}", page.evaluate(click_to_option_js).await?.into_value::<String>().unwrap_or_default());
    tokio::time::sleep(Duration::from_millis(500)).await;

    // 3. Click "Search Trains" button
    println!("Clicking 'Search Trains' button...");
    let click_search_btn_js = r#"
        (() => {
            const btn = document.querySelector('button.search-btn, button[type="submit"].search-btn');
            if (!btn) return 'SEARCH_BTN_NOT_FOUND';
            btn.click();
            return 'SEARCH_BTN_CLICKED: ' + btn.className;
        })()
    "#;
    println!("{}", page.evaluate(click_search_btn_js).await?.into_value::<String>().unwrap_or_default());

    println!("Waiting 6 seconds for search results...");
    tokio::time::sleep(Duration::from_secs(6)).await;

    // 4. Inspect Search Results DOM
    let inspect_results_js = r#"
        (() => {
            const url = window.location.href;
            const trainCards = Array.from(document.querySelectorAll('app-train-avl-enq'));
            const allButtons = Array.from(document.querySelectorAll('button, .btn'))
                .filter(b => b.offsetParent !== null)
                .map(b => ({
                    tag: b.tagName.toLowerCase(),
                    classes: b.className,
                    text: b.innerText.trim().substring(0, 50)
                }));


            const sampleCard = trainCards.length > 0 ? {
                tag: trainCards[0].tagName.toLowerCase(),
                classes: trainCards[0].className,
                text: trainCards[0].innerText.trim().substring(0, 300),
                buttons: Array.from(trainCards[0].querySelectorAll('button, .btn, [role="button"]')).map(b => ({
                    classes: b.className,
                    text: b.innerText.trim()
                }))
            } : null;

            return JSON.stringify({
                url,
                trainCardsCount: trainCards.length,
                sampleCard,
                visibleButtonsCount: allButtons.length,
                visibleButtons: allButtons.slice(0, 25)
            }, null, 2);
        })()
    "#;
    let res = page.evaluate(inspect_results_js).await?;
    println!("Search results DOM inspection:\n{}", res.into_value::<String>().unwrap_or_default());

    // 5. Inspect Class selection & Book Now button
    println!("Inspecting classes and availability in first train card...");
    let inspect_card_details_js = r#"
        (() => {
            const card = document.querySelector('app-train-avl-enq');
            if (!card) return JSON.stringify({ error: 'No train card' });

            const classes = Array.from(card.querySelectorAll('.class-box, [class*="class"], [class*="avl"], span, div'))
                .filter(el => ['SL', '3A', '2A', '1A', '3E', 'CC', '2S'].includes((el.innerText || '').trim()))
                .map(el => ({
                    tag: el.tagName.toLowerCase(),
                    classes: el.className,
                    text: el.innerText.trim()
                }));

            const availBtn = card.querySelector('button.btn-availability, .btn-availability');

            return JSON.stringify({
                classes,
                availBtnFound: !!availBtn
            }, null, 2);
        })()
    "#;
    println!("Card classes:\n{}", page.evaluate(inspect_card_details_js).await?.into_value::<String>().unwrap_or_default());

    // Click 3A or SL class in first card, or click btn-availability
    println!("Clicking availability button in first card...");
    let click_avail_js = r#"
        (() => {
            const card = document.querySelector('app-train-avl-enq');
            if (!card) return 'NO_CARD';
            const btn = card.querySelector('button.btn-availability, .btn-availability');
            if (btn) {
                btn.click();
                return 'CLICKED_AVAIL_BTN: ' + btn.className;
            }
            return 'NO_AVAIL_BTN';
        })()
    "#;
    println!("{}", page.evaluate(click_avail_js).await?.into_value::<String>().unwrap_or_default());
    tokio::time::sleep(Duration::from_secs(3)).await;

    println!("Waiting 5 seconds for availability data to populate...");
    tokio::time::sleep(Duration::from_secs(5)).await;

    // Check DOM after clicking availability
    let check_book_now_js = r#"
        (() => {
            const card = document.querySelector('app-train-avl-enq');
            if (!card) return 'NO_CARD';
            const buttons = Array.from(card.querySelectorAll('button, .btn, [role="button"], a.btn, .btnDefault'))
                .map(b => ({
                    tag: b.tagName.toLowerCase(),
                    classes: b.className,
                    text: b.innerText.trim().replace(/\n+/g, ' ')
                }));
            const avlBoxes = Array.from(card.querySelectorAll('[class*="avl"], [class*="book"], .pre-avl, [class*="card"], [class*="slot"], [class*="status"]'))
                .map(d => ({
                    tag: d.tagName.toLowerCase(),
                    classes: d.className,
                    text: d.innerText.trim().replace(/\n+/g, ' ').substring(0, 100)
                }));
            return JSON.stringify({ buttons, avlBoxes: avlBoxes.slice(0, 15) }, null, 2);
        })()
    "#;
    println!("After waiting 5 seconds for availability:\n{}", page.evaluate(check_book_now_js).await?.into_value::<String>().unwrap_or_default());

    // ── CLICK BOOK NOW ────────────────────────────────────────────────────────
    println!("\n=== CLICKING BOOK NOW ===");
    let click_book_js = r#"
        (() => {
            // Try within the train card first
            const card = document.querySelector('app-train-avl-enq');
            const container = card || document;
            const allBtns = Array.from(container.querySelectorAll('button, .btn, [role="button"], a.btn, .btnDefault'));
            const bookBtn = allBtns.find(b => {
                const t = (b.innerText || '').trim().toUpperCase();
                return t === 'BOOK NOW' || t === 'BOOK' || t.includes('BOOK NOW');
            });
            if (bookBtn) { bookBtn.click(); return 'CLICKED: ' + bookBtn.className + ' | ' + bookBtn.innerText.trim(); }
            return 'BOOK_BTN_NOT_FOUND — available buttons: ' + allBtns.map(b => b.innerText.trim().substring(0,20)).join(' | ');
        })()
    "#;
    println!("{}", page.evaluate(click_book_js).await?.into_value::<String>().unwrap_or_default());

    println!("Waiting 5 seconds for booking form to load...");
    tokio::time::sleep(Duration::from_secs(5)).await;

    // ── INSPECT BOOKING FORM ──────────────────────────────────────────────────
    println!("\n=== BOOKING FORM DOM INSPECTION ===");
    let inspect_booking_js = r#"
        (() => {
            const url = window.location.href;
            const allInputs = Array.from(document.querySelectorAll('input, select, textarea'))
                .map(el => ({
                    tag: el.tagName.toLowerCase(),
                    id: el.id || null,
                    cls: el.className || null,
                    fc: el.getAttribute('formcontrolname'),
                    ph: el.placeholder || null,
                    type: el.type || null,
                    name: el.name || null,
                    ariaLabel: el.getAttribute('aria-label') || null,
                    visible: el.offsetParent !== null
                }))
                .filter(el => el.visible);

            const allButtons = Array.from(document.querySelectorAll('button'))
                .filter(b => b.offsetParent !== null)
                .map(b => ({
                    cls: b.className,
                    text: b.innerText.trim().substring(0, 60),
                    type: b.type,
                    id: b.id
                }));

            const passengerForm = document.querySelector('app-passenger-details, app-pax-details, .pax-details, [class*="passenger"]');
            const paymentSection = document.querySelector('.payment-options, .pay-section, .bank-type, [class*="payment"]');

            return JSON.stringify({
                url,
                allInputs,
                allButtons: allButtons.slice(0, 30),
                passengerFormTag: passengerForm ? passengerForm.tagName + '.' + passengerForm.className.split(' ')[0] : null,
                paymentSectionTag: paymentSection ? paymentSection.tagName + '.' + paymentSection.className.split(' ')[0] : null,
            }, null, 2);
        })()
    "#;
    println!("Booking form:\n{}", page.evaluate(inspect_booking_js).await?.into_value::<String>().unwrap_or_default());

    println!("\nDone! Closing browser in 5 seconds...");
    tokio::time::sleep(Duration::from_secs(5)).await;

    Ok(())
}

