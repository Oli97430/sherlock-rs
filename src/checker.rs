use crate::result::{QueryResult, QueryStatus};
use crate::sites::SiteData;
use rand::seq::SliceRandom;
use regex::Regex;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::{mpsc, Semaphore};
use tokio::time::sleep;

// ── 25 real browser User-Agents ─────────────────────────────────────────────
const USER_AGENTS: &[&str] = &[
    // Chrome Windows
    "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/131.0.0.0 Safari/537.36",
    "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/130.0.0.0 Safari/537.36",
    "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/129.0.0.0 Safari/537.36",
    "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/128.0.0.0 Safari/537.36",
    // Chrome macOS
    "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/131.0.0.0 Safari/537.36",
    "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/130.0.0.0 Safari/537.36",
    "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/129.0.0.0 Safari/537.36",
    // Chrome Linux
    "Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/131.0.0.0 Safari/537.36",
    "Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/130.0.0.0 Safari/537.36",
    // Firefox Windows
    "Mozilla/5.0 (Windows NT 10.0; Win64; x64; rv:133.0) Gecko/20100101 Firefox/133.0",
    "Mozilla/5.0 (Windows NT 10.0; Win64; x64; rv:132.0) Gecko/20100101 Firefox/132.0",
    "Mozilla/5.0 (Windows NT 10.0; Win64; x64; rv:131.0) Gecko/20100101 Firefox/131.0",
    "Mozilla/5.0 (Windows NT 10.0; Win64; x64; rv:130.0) Gecko/20100101 Firefox/130.0",
    // Firefox macOS
    "Mozilla/5.0 (Macintosh; Intel Mac OS X 10.15; rv:133.0) Gecko/20100101 Firefox/133.0",
    // Firefox Linux
    "Mozilla/5.0 (X11; Linux x86_64; rv:133.0) Gecko/20100101 Firefox/133.0",
    // Edge Windows
    "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/131.0.0.0 Safari/537.36 Edg/131.0.0.0",
    "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/130.0.0.0 Safari/537.36 Edg/130.0.0.0",
    // Edge macOS
    "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/131.0.0.0 Safari/537.36 Edg/131.0.0.0",
    // Safari macOS
    "Mozilla/5.0 (Macintosh; Intel Mac OS X 14_1) AppleWebKit/605.1.15 (KHTML, like Gecko) Version/17.1 Safari/605.1.15",
    "Mozilla/5.0 (Macintosh; Intel Mac OS X 14_0) AppleWebKit/605.1.15 (KHTML, like Gecko) Version/17.0 Safari/605.1.15",
    // Safari iOS
    "Mozilla/5.0 (iPhone; CPU iPhone OS 17_2 like Mac OS X) AppleWebKit/605.1.15 (KHTML, like Gecko) Version/17.2 Mobile/15E148 Safari/604.1",
    // Chrome Android
    "Mozilla/5.0 (Linux; Android 14; Pixel 8) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/131.0.0.0 Mobile Safari/537.36",
    "Mozilla/5.0 (Linux; Android 13; SM-S918B) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/131.0.0.0 Mobile Safari/537.36",
    // Opera
    "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/131.0.0.0 Safari/537.36 OPR/116.0.0.0",
    // Brave (same UA as Chrome, different internals)
    "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/131.0.0.0 Safari/537.36",
];

fn random_ua() -> &'static str {
    USER_AGENTS
        .choose(&mut rand::thread_rng())
        .copied()
        .unwrap_or(USER_AGENTS[0])
}

// ── WAF signatures ────────────────────────────────────────────────────────────
const WAF_SIGNATURES: &[&str] = &[
    "Attention Required! | Cloudflare",
    "cf-browser-verification",
    "Please Wait... | Cloudflare",
    "Just a moment...",
    "Checking your browser",
    "Pardon Our Interruption",
    "Access denied | ",
    "_cf_chl_opt",
];

pub struct CheckConfig {
    pub timeout_secs: u64,
    pub include_nsfw: bool,
    pub proxy: Option<String>,
}

pub async fn check_username(
    username: &str,
    sites: &HashMap<String, SiteData>,
    config: &CheckConfig,
    tx: mpsc::Sender<QueryResult>,
) {
    let base_ua = random_ua();

    let mut client_builder = reqwest::Client::builder()
        .timeout(Duration::from_secs(config.timeout_secs))
        .user_agent(base_ua)
        .danger_accept_invalid_certs(false);

    let mut client_no_redir_builder = reqwest::Client::builder()
        .timeout(Duration::from_secs(config.timeout_secs))
        .user_agent(base_ua)
        .redirect(reqwest::redirect::Policy::none())
        .danger_accept_invalid_certs(false);

    if let Some(proxy_url) = &config.proxy {
        if !proxy_url.is_empty() {
            if let Ok(proxy) = reqwest::Proxy::all(proxy_url) {
                client_builder = client_builder.proxy(proxy.clone());
                client_no_redir_builder = client_no_redir_builder.proxy(proxy);
            }
        }
    }

    let client = client_builder.build().unwrap_or_default();
    let client_no_redir = client_no_redir_builder.build().unwrap_or_default();

    let semaphore = Arc::new(Semaphore::new(20));
    let (result_tx, mut result_rx) = mpsc::channel::<QueryResult>(300);

    for (name, site) in sites.iter() {
        if !config.include_nsfw && site.is_nsfw.unwrap_or(false) {
            continue;
        }

        if let Some(regex_str) = &site.regex_check {
            if let Ok(re) = Regex::new(regex_str) {
                if !re.is_match(username) {
                    let _ = result_tx
                        .send(QueryResult {
                            username: username.to_string(),
                            site_name: name.clone(),
                            url_main: site.url_main.clone(),
                            site_url: site.url.replace("{}", username),
                            status: QueryStatus::Illegal,
                            response_time_ms: None,
                            context: Some("Invalid username format for this site".into()),
                        })
                        .await;
                    continue;
                }
            }
        }

        let name = name.clone();
        let site = site.clone();
        let username = username.to_string();
        let c = client.clone();
        let cnr = client_no_redir.clone();
        let sem = semaphore.clone();
        let rtx = result_tx.clone();

        tokio::spawn(async move {
            let _permit = sem.acquire().await.unwrap();
            let result = check_site_with_retry(&name, &site, &username, &c, &cnr).await;
            let _ = rtx.send(result).await;
        });
    }

    drop(result_tx);

    while let Some(result) = result_rx.recv().await {
        if tx.send(result).await.is_err() {
            break;
        }
    }
}

// ── Retry wrapper: up to 3 attempts with exponential backoff ─────────────────
async fn check_site_with_retry(
    name: &str,
    site: &SiteData,
    username: &str,
    client: &reqwest::Client,
    client_no_redir: &reqwest::Client,
) -> QueryResult {
    const MAX_ATTEMPTS: u32 = 3;
    let mut last: Option<QueryResult> = None;

    for attempt in 0..MAX_ATTEMPTS {
        if attempt > 0 {
            // Exponential backoff: 500ms → 1000ms
            sleep(Duration::from_millis(500 * (1u64 << (attempt - 1)))).await;
        }

        let result = check_site(name, site, username, client, client_no_redir).await;

        let is_network_error = matches!(result.status, QueryStatus::Unknown)
            && result
                .context
                .as_deref()
                .map(|c| c.starts_with("NET:"))
                .unwrap_or(false);

        if !is_network_error {
            return result;
        }

        last = Some(result);
    }

    // All retries exhausted — clean context for display
    let mut final_result = last.unwrap();
    if let Some(ctx) = final_result.context.as_mut() {
        if let Some(stripped) = ctx.strip_prefix("NET: ") {
            *ctx = format!("{} (after {} retries)", stripped, MAX_ATTEMPTS - 1);
        }
    }
    final_result
}

// ── Core request function ─────────────────────────────────────────────────────
async fn check_site(
    name: &str,
    site: &SiteData,
    username: &str,
    client: &reqwest::Client,
    client_no_redir: &reqwest::Client,
) -> QueryResult {
    let url = site.url.replace("{}", username);
    let probe_url = site
        .url_probe
        .as_ref()
        .map(|u| u.replace("{}", username))
        .unwrap_or_else(|| url.clone());

    let active_client = if site.error_type == "response_url" {
        client_no_redir
    } else {
        client
    };

    let method = match site.request_method.as_deref() {
        Some("POST") => reqwest::Method::POST,
        Some("HEAD") => reqwest::Method::HEAD,
        Some("PUT") => reqwest::Method::PUT,
        _ => reqwest::Method::GET,
    };

    let start = Instant::now();

    // Override UA per request for rotation
    let mut request = active_client
        .request(method, &probe_url)
        .header(reqwest::header::USER_AGENT, random_ua());

    if let Some(headers) = &site.headers {
        for (k, v) in headers {
            request = request.header(k.as_str(), v.as_str());
        }
    }

    if let Some(payload) = &site.request_payload {
        let payload_str = serde_json::to_string(payload)
            .unwrap_or_default()
            .replace("{}", username);
        request = request
            .header("Content-Type", "application/json")
            .body(payload_str);
    }

    match request.send().await {
        Ok(response) => {
            let elapsed = start.elapsed().as_millis() as u64;
            let status_code = response.status().as_u16();
            let body = response.text().await.unwrap_or_default();
            let status = determine_status(site, status_code, &body);

            QueryResult {
                username: username.to_string(),
                site_name: name.to_string(),
                url_main: site.url_main.clone(),
                site_url: url,
                status,
                response_time_ms: Some(elapsed),
                context: None,
            }
        }
        Err(e) => {
            let elapsed = start.elapsed().as_millis() as u64;
            // Tag network errors so retry logic can identify them
            let prefix = if e.is_timeout() || e.is_connect() {
                "NET: "
            } else {
                "Error: "
            };
            QueryResult {
                username: username.to_string(),
                site_name: name.to_string(),
                url_main: site.url_main.clone(),
                site_url: url,
                status: QueryStatus::Unknown,
                response_time_ms: Some(elapsed),
                context: Some(format!("{}{}", prefix, e)),
            }
        }
    }
}

fn detect_waf(body: &str) -> bool {
    let lower = body.to_lowercase();
    WAF_SIGNATURES
        .iter()
        .any(|sig| lower.contains(&sig.to_lowercase()))
}

fn determine_status(site: &SiteData, status_code: u16, body: &str) -> QueryStatus {
    if detect_waf(body) {
        return QueryStatus::Waf;
    }

    match site.error_type.as_str() {
        "status_code" => {
            let is_error = site
                .error_code
                .as_ref()
                .map(|ec| ec.matches(status_code))
                .unwrap_or(status_code == 404);

            if is_error {
                QueryStatus::Available
            } else if (200..300).contains(&status_code) {
                QueryStatus::Claimed
            } else {
                QueryStatus::Unknown
            }
        }
        "message" => {
            if let Some(error_msgs) = &site.error_msg {
                let has_error = error_msgs.as_vec().iter().any(|msg| body.contains(msg));
                if has_error {
                    QueryStatus::Available
                } else if (200..300).contains(&status_code) {
                    QueryStatus::Claimed
                } else {
                    QueryStatus::Unknown
                }
            } else {
                QueryStatus::Unknown
            }
        }
        "response_url" => {
            if (200..300).contains(&status_code) {
                QueryStatus::Claimed
            } else {
                QueryStatus::Available
            }
        }
        _ => QueryStatus::Unknown,
    }
}
