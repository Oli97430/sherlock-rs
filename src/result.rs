use serde::Serialize;

#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum QueryStatus {
    Claimed,
    Available,
    Unknown,
    Illegal,
    Waf,
}

impl QueryStatus {
    pub fn as_str(&self) -> &str {
        match self {
            QueryStatus::Claimed => "claimed",
            QueryStatus::Available => "available",
            QueryStatus::Unknown => "unknown",
            QueryStatus::Illegal => "illegal",
            QueryStatus::Waf => "waf",
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct QueryResult {
    pub username: String,
    pub site_name: String,
    pub url_main: String,
    pub site_url: String,
    pub status: QueryStatus,
    pub response_time_ms: Option<u64>,
    pub context: Option<String>,
}
