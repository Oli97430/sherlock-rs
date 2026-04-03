use anyhow::Result;
use serde::Deserialize;
use std::collections::HashMap;
use std::path::PathBuf;

const DATA_URL: &str = "https://raw.githubusercontent.com/sherlock-project/sherlock/master/sherlock_project/resources/data.json";

#[derive(Deserialize, Clone, Debug)]
#[serde(untagged)]
pub enum ErrorMsg {
    Single(String),
    Multiple(Vec<String>),
}

impl ErrorMsg {
    pub fn as_vec(&self) -> Vec<&str> {
        match self {
            ErrorMsg::Single(s) => vec![s.as_str()],
            ErrorMsg::Multiple(v) => v.iter().map(|s| s.as_str()).collect(),
        }
    }
}

#[derive(Deserialize, Clone, Debug)]
#[serde(untagged)]
pub enum ErrorCode {
    Single(u16),
    Multiple(Vec<u16>),
}

impl ErrorCode {
    pub fn matches(&self, code: u16) -> bool {
        match self {
            ErrorCode::Single(c) => *c == code,
            ErrorCode::Multiple(codes) => codes.contains(&code),
        }
    }
}

#[derive(Deserialize, Clone, Debug)]
pub struct SiteData {
    #[serde(rename = "errorMsg")]
    pub error_msg: Option<ErrorMsg>,
    #[serde(rename = "errorType")]
    pub error_type: String,
    #[serde(rename = "errorCode")]
    pub error_code: Option<ErrorCode>,
    #[serde(rename = "errorUrl")]
    pub error_url: Option<String>,
    pub url: String,
    #[serde(rename = "urlMain")]
    pub url_main: String,
    #[serde(rename = "urlProbe")]
    pub url_probe: Option<String>,
    pub username_claimed: Option<String>,
    pub username_unclaimed: Option<String>,
    #[serde(rename = "regexCheck")]
    pub regex_check: Option<String>,
    #[serde(rename = "isNSFW")]
    pub is_nsfw: Option<bool>,
    pub headers: Option<HashMap<String, String>>,
    pub request_method: Option<String>,
    pub request_payload: Option<serde_json::Value>,
}

fn data_dir() -> PathBuf {
    dirs::data_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("sherlock-rs")
}

pub async fn load_sites() -> Result<HashMap<String, SiteData>> {
    let path = data_dir().join("data.json");
    if path.exists() {
        let json = tokio::fs::read_to_string(&path).await?;
        return parse_sites(&json);
    }
    download_sites().await
}

pub async fn download_sites() -> Result<HashMap<String, SiteData>> {
    let dir = data_dir();
    tokio::fs::create_dir_all(&dir).await?;

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(30))
        .build()?;

    let json = client.get(DATA_URL).send().await?.text().await?;
    tokio::fs::write(dir.join("data.json"), &json).await?;
    parse_sites(&json)
}

fn parse_sites(json: &str) -> Result<HashMap<String, SiteData>> {
    let raw: HashMap<String, serde_json::Value> = serde_json::from_str(json)?;
    let sites = raw
        .into_iter()
        .filter(|(k, _)| k != "$schema")
        .filter_map(|(k, v)| serde_json::from_value(v).ok().map(|s| (k, s)))
        .collect();
    Ok(sites)
}
