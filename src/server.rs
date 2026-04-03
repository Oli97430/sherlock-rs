use crate::checker::{self, CheckConfig};
use crate::export;
use crate::result::QueryResult;
use crate::sites::{self, SiteData};
use axum::extract::{Query, State};
use axum::http::header;
use axum::response::sse::{Event, KeepAlive, Sse};
use axum::response::{Html, IntoResponse};
use axum::routing::{get, post};
use axum::{Json, Router};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::convert::Infallible;
use std::sync::Arc;
use tokio::sync::RwLock;
use tokio_stream::wrappers::ReceiverStream;

const FRONTEND_HTML: &str = include_str!("../frontend/index.html");

pub struct AppState {
    pub sites: RwLock<Option<HashMap<String, SiteData>>>,
    pub last_results: RwLock<Vec<QueryResult>>,
    pub last_username: RwLock<String>,
    pub load_error: RwLock<Option<String>>,
}

impl AppState {
    pub fn new() -> Self {
        Self {
            sites: RwLock::new(None),
            last_results: RwLock::new(Vec::new()),
            last_username: RwLock::new(String::new()),
            load_error: RwLock::new(None),
        }
    }
}

pub fn create_router(state: Arc<AppState>) -> Router {
    Router::new()
        .route("/", get(index_handler))
        .route("/api/status", get(status_handler))
        .route("/api/search", get(search_handler))
        .route("/api/export/csv", get(export_csv_handler))
        .route("/api/export/txt", get(export_txt_handler))
        .route("/api/update-db", post(update_db_handler))
        .with_state(state)
}

async fn index_handler() -> Html<&'static str> {
    Html(FRONTEND_HTML)
}

#[derive(Serialize)]
struct StatusResponse {
    ready: bool,
    sites_count: usize,
    error: Option<String>,
}

async fn status_handler(State(state): State<Arc<AppState>>) -> Json<StatusResponse> {
    let sites = state.sites.read().await;
    let error = state.load_error.read().await;
    Json(StatusResponse {
        ready: sites.is_some(),
        sites_count: sites.as_ref().map(|s| s.len()).unwrap_or(0),
        error: error.clone(),
    })
}

#[derive(Deserialize)]
struct SearchParams {
    username: String,
    timeout: Option<u64>,
    nsfw: Option<bool>,
    proxy: Option<String>,
}

#[derive(Serialize)]
struct SseResultData {
    site_name: String,
    url_main: String,
    site_url: String,
    status: String,
    response_time_ms: Option<u64>,
    checked: usize,
    total: usize,
}

async fn search_handler(
    State(state): State<Arc<AppState>>,
    Query(params): Query<SearchParams>,
) -> Sse<impl tokio_stream::Stream<Item = Result<Event, Infallible>>> {
    let (sse_tx, sse_rx) = tokio::sync::mpsc::channel::<Result<Event, Infallible>>(200);

    let sites_guard = state.sites.read().await;
    let sites = sites_guard.clone().unwrap_or_default();
    drop(sites_guard);

    // Filter out NSFW if needed
    let include_nsfw = params.nsfw.unwrap_or(false);
    let total: usize = sites
        .values()
        .filter(|s| include_nsfw || !s.is_nsfw.unwrap_or(false))
        .count();

    // Clear previous results
    {
        let mut results = state.last_results.write().await;
        results.clear();
        let mut uname = state.last_username.write().await;
        *uname = params.username.clone();
    }

    let username = params.username.clone();
    let config = CheckConfig {
        timeout_secs: params.timeout.unwrap_or(30),
        include_nsfw,
        proxy: params.proxy.clone(),
    };

    let state_clone = state.clone();

    tokio::spawn(async move {
        let (checker_tx, mut checker_rx) = tokio::sync::mpsc::channel::<QueryResult>(200);

        let checker_handle = tokio::spawn(async move {
            checker::check_username(&username, &sites, &config, checker_tx).await;
        });

        let mut checked: usize = 0;

        while let Some(result) = checker_rx.recv().await {
            checked += 1;

            let event_data = SseResultData {
                site_name: result.site_name.clone(),
                url_main: result.url_main.clone(),
                site_url: result.site_url.clone(),
                status: result.status.as_str().to_string(),
                response_time_ms: result.response_time_ms,
                checked,
                total,
            };

            state_clone.last_results.write().await.push(result);

            let json = serde_json::to_string(&event_data).unwrap_or_default();
            let event = Event::default().event("result").data(json);

            if sse_tx.send(Ok(event)).await.is_err() {
                break;
            }
        }

        let _ = checker_handle.await;

        // Send completion event
        let found = state_clone
            .last_results
            .read()
            .await
            .iter()
            .filter(|r| r.status == crate::result::QueryStatus::Claimed)
            .count();

        let done_json = serde_json::to_string(&serde_json::json!({
            "total_found": found,
            "total_checked": checked,
        }))
        .unwrap_or_default();

        let _ = sse_tx
            .send(Ok(Event::default().event("done").data(done_json)))
            .await;
    });

    Sse::new(ReceiverStream::new(sse_rx)).keep_alive(KeepAlive::default())
}

async fn export_csv_handler(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    let results = state.last_results.read().await;
    let csv_data = export::to_csv(&results);
    (
        [
            (header::CONTENT_TYPE, "text/csv; charset=utf-8"),
            (
                header::CONTENT_DISPOSITION,
                "attachment; filename=\"sherlock_results.csv\"",
            ),
        ],
        csv_data,
    )
}

async fn export_txt_handler(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    let results = state.last_results.read().await;
    let username = state.last_username.read().await;
    let txt_data = export::to_txt(&username, &results);
    (
        [
            (header::CONTENT_TYPE, "text/plain; charset=utf-8"),
            (
                header::CONTENT_DISPOSITION,
                "attachment; filename=\"sherlock_results.txt\"",
            ),
        ],
        txt_data,
    )
}

#[derive(Serialize)]
struct UpdateResponse {
    success: bool,
    sites_count: usize,
    error: Option<String>,
}

async fn update_db_handler(State(state): State<Arc<AppState>>) -> Json<UpdateResponse> {
    match sites::download_sites().await {
        Ok(new_sites) => {
            let count = new_sites.len();
            *state.sites.write().await = Some(new_sites);
            *state.load_error.write().await = None;
            Json(UpdateResponse {
                success: true,
                sites_count: count,
                error: None,
            })
        }
        Err(e) => Json(UpdateResponse {
            success: false,
            sites_count: 0,
            error: Some(e.to_string()),
        }),
    }
}
