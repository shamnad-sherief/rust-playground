use anyhow::Result;
use chromiumoxide::browser::{Browser, BrowserConfig};
use futures::StreamExt;
use tracing::info;

/// JavaScript stealth patches injected before any page loads.
/// These mask automation signals that Akamai and other bot detectors check.
pub const STEALTH_JS: &str = r#"
    (() => {
        // 1. Remove the webdriver flag that Chrome DevTools Protocol sets
        try {
            Object.defineProperty(navigator, 'webdriver', {
                get: () => undefined,
            });
        } catch (e) {}

        // 2. Mock chrome.runtime to look like a real Chrome install
        try {
            if (!window.chrome) {
                window.chrome = {};
            }
            window.chrome.runtime = {};
            window.chrome.loadTimes = function() {
                return {
                    commitLoadTime: Date.now() / 1000,
                    connectionInfo: "h2",
                    finishDocumentLoadTime: Date.now() / 1000 + 0.1,
                    finishLoadTime: Date.now() / 1000 + 0.2,
                    firstPaintAfterLoadTime: 0,
                    firstPaintTime: Date.now() / 1000 + 0.05,
                    navigationType: "Other",
                    npnNegotiatedProtocol: "h2",
                    requestTime: Date.now() / 1000 - 0.5,
                    startLoadTime: Date.now() / 1000 - 0.3,
                    wasAlternateProtocolAvailable: false,
                    wasFetchedViaSpdy: true,
                    wasNpnNegotiated: true,
                };
            };
            window.chrome.csi = function() {
                return {
                    onloadT: Date.now(),
                    startE: Date.now() - 300,
                    pageT: 300,
                };
            };
        } catch (e) {}

        // 3. Fix the permissions API (headless returns inconsistent values)
        try {
            const originalQuery = window.navigator.permissions.query;
            window.navigator.permissions.query = (parameters) =>
                parameters.name === 'notifications'
                    ? Promise.resolve({ state: Notification.permission })
                    : originalQuery(parameters);
        } catch (e) {}

        // 4. Mock a realistic plugin array
        try {
            Object.defineProperty(navigator, 'plugins', {
                get: () => [
                    { name: 'Chrome PDF Plugin', filename: 'internal-pdf-viewer', description: 'Portable Document Format' },
                    { name: 'Chrome PDF Viewer', filename: 'mhjfbmdgcfjbbpaeojofohoefgiehjai', description: '' },
                    { name: 'Native Client', filename: 'internal-nacl-plugin', description: '' },
                ],
            });
        } catch (e) {}

        // 5. Set realistic languages
        try {
            Object.defineProperty(navigator, 'languages', {
                get: () => ['en-US', 'en', 'hi'],
            });
        } catch (e) {}

        // 6. Override WebGL vendor and renderer
        try {
            const getParameterProto = WebGLRenderingContext.prototype.getParameter;
            WebGLRenderingContext.prototype.getParameter = function(parameter) {
                // UNMASKED_VENDOR_WEBGL
                if (parameter === 37445) return 'Intel Inc.';
                // UNMASKED_RENDERER_WEBGL
                if (parameter === 37446) return 'Intel Iris OpenGL Engine';
                return getParameterProto.call(this, parameter);
            };
        } catch (e) {}

        // 7. Fix connection properties
        try {
            Object.defineProperty(navigator, 'connection', {
                get: () => ({
                    effectiveType: '4g',
                    rtt: 50,
                    downlink: 10,
                    saveData: false,
                }),
            });
        } catch (e) {}

        // 8. Make hardwareConcurrency realistic
        try {
            Object.defineProperty(navigator, 'hardwareConcurrency', {
                get: () => 8,
            });
        } catch (e) {}

        // 9. Fix deviceMemory
        try {
            Object.defineProperty(navigator, 'deviceMemory', {
                get: () => 8,
            });
        } catch (e) {}

        return 'OK';
    })()
"#;

/// Launch Chrome in headed mode with stealth flags.
///
/// Returns a `Browser` handle and a background task handle that drives
/// the CDP event loop. The background task must keep running for the
/// browser to function.
pub async fn launch_stealth_browser() -> Result<(Browser, tokio::task::JoinHandle<()>)> {
    info!("Launching Chrome in headed (visible) mode with stealth flags...");

    let mut builder = BrowserConfig::builder()
        .with_head() // VISIBLE browser window
        .window_size(1366, 768)
        .arg("--disable-blink-features=AutomationControlled")
        .arg("--disable-features=IsolateOrigins,site-per-process")
        .arg("--disable-infobars")
        .arg("--no-first-run")
        .arg("--no-default-browser-check")
        .arg("--disable-background-timer-throttling")
        .arg("--disable-renderer-backgrounding")
        .arg("--disable-backgrounding-occluded-windows")
        .arg("--disable-ipc-flooding-protection")
        .arg("--disable-dev-shm-usage")
        .arg("--no-sandbox")
        .arg("--user-agent=Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/127.0.6533.100 Safari/537.36");

    if let Ok(chrome_path) = std::env::var("CHROME_BIN").or_else(|_| std::env::var("CHROME_PATH")) {
        let trimmed = chrome_path.trim();
        if !trimmed.is_empty() {
            info!("Using custom Chrome binary: {}", trimmed);
            builder = builder.chrome_executable(trimmed);
        }
    }

    let config = builder.build().map_err(|e| {
        anyhow::anyhow!(
            "Failed to find Chrome or Chromium executable.\n\
             Please install Chrome or Chromium:\n\
               - Chromium: sudo apt install chromium\n\
               - Google Chrome: https://www.google.com/chrome/\n\
             Or specify the binary path using CHROME_BIN=/path/to/chrome in .env.\n\
             (Underlying error: {})",
            e
        )
    })?;

    let (browser, mut handler) = Browser::launch(config).await?;

    // Spawn the CDP event loop handler in the background
    let handle = tokio::task::spawn(async move {
        while let Some(_event) = handler.next().await {
            // CDP events are processed here; we just need to keep draining them
        }
    });

    info!("Chrome launched successfully");
    Ok((browser, handle))
}
