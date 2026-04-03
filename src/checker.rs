use crate::result::{QueryResult, QueryStatus};
use crate::sites::SiteData;
use regex::Regex;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::{mpsc, Semaphore};

const USER_AGENT: &str = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/131.0.0.0 Safari/537.36";

const WAF_SIGNATURES: &[&str] = &[
    "Attention Required! | Cloudflare",
    "cf-browser-verification",
    "Please Wait... | Cloudflare",
    "Just a moment...",
    "Checking your browser",
    "Pardon Our Interruption",
    "Access denied | ",
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
    let mut client_builder = reqwest::Client::builder()
        .timeout(Duration::from_secs(config.timeout_secs))
        .user_agent(USER_AGENT)
        .danger_accept_invalid_certs(false);

    let mut client_no_redir_builder = reqwest::Client::builder()
        .timeout(Duration::from_secs(config.timeout_secs))
        .user_agent(USER_AGENT)
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
    let (result_tx, mut result_rx) = mpsc::channel::<QueryResult>(200);

    let mut spawned = 0usize;

    for (name, site) in sites.iter() {
        if !config.include_nsfw && site.is_nsfw.unwrap_or(false) {
            continue;
        }

        if let Some(regex_str) = &site.regex_check {
            if let Ok(re) = Regex::new(regex_str) {
                if !re.is_match(username) {
                    let _ = result_tx
                        .send(QueryResult {
                            site_name: name.clone(),
                            url_main: site.url_main.clone(),
                            site_url: site.url.replace("{}", username),
                            status: QueryStatus::Illegal,
                            response_time_ms: None,
                            context: Some("Invalid username format for this site".into()),
                        })
                        .await;
                    spawned += 1;
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
            let result = check_site(&name, &site, &username, &c, &cnr).await;
            let _ = rtx.send(result).await;
        });

        spawned += 1;
    }

    drop(result_tx);

    let mut count = 0usize;
    while let Some(mut result) = result_rx.recv().await {
        count += 1;
        // Tag with progress info via context (hacky but avoids changing the struct)
        let progress = format!("{}/{}", count, spawned);
        if result.context.is_none() {
            result.context = Some(progress);
        }
        if tx.send(result).await.is_err() {
            break;
        }
    }
}

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

    let mut request = active_client.request(method, &probe_url);

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
            QueryResult {
                site_name: name.to_string(),
                url_main: site.url_main.clone(),
                site_url: url,
                status: QueryStatus::Unknown,
                response_time_ms: Some(elapsed),
                context: Some(format!("Error: {}", e)),
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
