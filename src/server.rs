// Copyright (C) 2026 salty919
// SPDX-License-Identifier: GPL-3.0-only

//! Loopback-only, read-only REST API for an already running Codex Info UI.
//!
//! This module deliberately knows nothing about Slint, Codex app-server,
//! SQLite, or local session files. The UI thread copies a whitelisted immutable
//! snapshot into [`ApiSnapshotPublisher`]; HTTP handlers only read that copy.

use crate::security;
use axum::extract::State;
use axum::http::{
    header::{CACHE_CONTROL, CONNECTION, CONTENT_TYPE},
    HeaderValue, StatusCode,
};
use axum::response::{IntoResponse, Response};
use axum::routing::get;
use axum::{Json, Router};
use serde::Serialize;
use std::collections::HashSet;
use std::env;
use std::fmt;
use std::net::{IpAddr, SocketAddr, TcpListener};
use std::sync::{mpsc, Arc, RwLock};
use std::thread::{self, JoinHandle};
use std::time::Duration;
use tokio::net::TcpListener as TokioTcpListener;
use tokio::sync::oneshot;

/// Environment variable that opt-ins to the local REST listener.
pub const API_LISTEN_ENV: &str = "CODEX_INFO_API_LISTEN";
pub const API_VERSION: &str = "v1";
/// Maximum number of model rows accepted at the public boundary.
pub const MAX_PUBLIC_MODELS: usize = 3;
/// History is retained locally for three months. Keep the wire representation
/// bounded even when a busy account has one sample per minute for that period.
pub const MAX_PUBLIC_HISTORY_PERIODS: usize = 128;
pub const MAX_PUBLIC_HISTORY_SAMPLES: usize = 100_000;
pub const MAX_PUBLIC_THREADS: usize = 256;
const API_START_TIMEOUT: Duration = Duration::from_secs(1);
const MAX_PUBLIC_UNIX_SECONDS: i64 = 253_402_300_799; // 9999-12-31T23:59:59Z
const MAX_PUBLIC_ID_SCALARS: usize = 512;
const MAX_PUBLIC_HISTORY_LABEL_SCALARS: usize = 512;

/// The public availability of the monitor data. No error detail is exported.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum PublicState {
    #[default]
    Initializing,
    Ready,
    AuthRequired,
    Error,
}

/// Quota values safe for the intranet monitoring client.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct PublicQuota {
    pub remaining_percent: f64,
    pub reset_at: i64,
    pub window_seconds: i64,
    pub monthly: bool,
}

/// Per-model usage values. `input_tokens` excludes cached input tokens.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct PublicModelUsage {
    pub name: String,
    pub input_tokens: u64,
    pub cached_input_tokens: u64,
    pub output_tokens: u64,
}

/// Detailed per-model usage for `/v1/details`. The legacy `/v1/status`
/// response intentionally continues to use [`PublicModelUsage`] so adding
/// pricing fields cannot change its strict schema.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct PublicDetailedModelUsage {
    pub name: String,
    pub input_tokens: u64,
    pub cached_input_tokens: u64,
    pub output_tokens: u64,
    pub input_dollars: f64,
    pub cached_input_dollars: f64,
    pub output_dollars: f64,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct PublicHistoryPeriod {
    pub id: String,
    pub start_at: i64,
    pub end_at: i64,
    pub label: String,
    pub current: bool,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct PublicHistorySample {
    pub timestamp: i64,
    pub reset_at: i64,
    /// `null` means the local session backfill had no quota observation.
    pub remaining_percent: Option<f64>,
    pub sol_dollars: f64,
    pub terra_dollars: f64,
    pub luna_dollars: f64,
    pub sol_tokens: u64,
    pub terra_tokens: u64,
    pub luna_tokens: u64,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct PublicThread {
    pub id: String,
    pub title: String,
    pub parent_thread_id: Option<String>,
    pub model: String,
    pub model_label: String,
    pub total_tokens: Option<u64>,
    pub context_usage_tokens: Option<u64>,
    pub context_window_tokens: Option<u64>,
    pub created_at: Option<i64>,
    pub last_user_message_at: Option<i64>,
    pub is_subagent: bool,
    pub depth: Option<i32>,
}

/// Immutable data that may cross the REST trust boundary.
///
/// Do not add account email, authentication URLs, filesystem locations, raw
/// backend errors, or secrets to this type.
#[derive(Clone, Debug, Default, PartialEq, Serialize)]
pub struct PublicSnapshot {
    pub state: PublicState,
    pub observed_at: Option<i64>,
    pub authenticated: bool,
    pub plan_label: Option<String>,
    pub quota: Option<PublicQuota>,
    pub models: Vec<PublicModelUsage>,
    pub active_thread_count: u64,
}

/// The additive `/v1/details` document. The scalar fields intentionally mirror
/// the legacy status document while model rows carry the additional pricing
/// columns needed by the Windows client.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct PublicDetails {
    pub state: PublicState,
    pub observed_at: Option<i64>,
    pub authenticated: bool,
    pub plan_label: Option<String>,
    pub quota: Option<PublicQuota>,
    pub models: Vec<PublicDetailedModelUsage>,
    pub active_thread_count: u64,
    pub history_periods: Vec<PublicHistoryPeriod>,
    pub history_samples: Vec<PublicHistorySample>,
    pub threads: Vec<PublicThread>,
    pub estimated_cost_label: String,
}

impl Default for PublicDetails {
    fn default() -> Self {
        Self {
            state: PublicState::Initializing,
            observed_at: None,
            authenticated: false,
            plan_label: None,
            quota: None,
            models: Vec::new(),
            active_thread_count: 0,
            history_periods: Vec::new(),
            history_samples: Vec::new(),
            threads: Vec::new(),
            estimated_cost_label: "概算 —".to_owned(),
        }
    }
}

impl PublicDetails {
    /// Build an additive document from a legacy snapshot for callers that have
    /// not opted into detailed publication yet. The resulting detail rows are
    /// still schema-valid; pricing is zero because it was not supplied.
    fn from_snapshot(snapshot: &PublicSnapshot) -> Self {
        Self {
            state: snapshot.state,
            observed_at: snapshot.observed_at,
            authenticated: snapshot.authenticated,
            plan_label: snapshot.plan_label.clone(),
            quota: snapshot.quota.clone(),
            models: snapshot
                .models
                .iter()
                .map(|model| PublicDetailedModelUsage {
                    name: model.name.clone(),
                    input_tokens: model.input_tokens,
                    cached_input_tokens: model.cached_input_tokens,
                    output_tokens: model.output_tokens,
                    input_dollars: 0.0,
                    cached_input_dollars: 0.0,
                    output_dollars: 0.0,
                })
                .collect(),
            active_thread_count: snapshot.active_thread_count,
            estimated_cost_label: "概算 —".to_owned(),
            ..Self::default()
        }
    }

    fn status_snapshot(&self) -> PublicSnapshot {
        PublicSnapshot {
            state: self.state,
            observed_at: self.observed_at,
            authenticated: self.authenticated,
            plan_label: self.plan_label.clone(),
            quota: self.quota.clone(),
            models: self
                .models
                .iter()
                .map(|model| PublicModelUsage {
                    name: model.name.clone(),
                    input_tokens: model.input_tokens,
                    cached_input_tokens: model.cached_input_tokens,
                    output_tokens: model.output_tokens,
                })
                .collect(),
            active_thread_count: self.active_thread_count,
        }
    }

    fn validate(&self) -> Result<(), ApiSnapshotError> {
        self.status_snapshot().validate()?;
        if self.history_periods.len() > MAX_PUBLIC_HISTORY_PERIODS {
            return Err(ApiSnapshotError::ListTooLong);
        }
        if self.history_samples.len() > MAX_PUBLIC_HISTORY_SAMPLES {
            return Err(ApiSnapshotError::ListTooLong);
        }
        if self.threads.len() > MAX_PUBLIC_THREADS {
            return Err(ApiSnapshotError::ListTooLong);
        }
        if self.models.len() > MAX_PUBLIC_MODELS {
            return Err(ApiSnapshotError::ListTooLong);
        }

        for model in &self.models {
            if !valid_non_negative_rate(model.input_dollars)
                || !valid_non_negative_rate(model.cached_input_dollars)
                || !valid_non_negative_rate(model.output_dollars)
            {
                return Err(ApiSnapshotError::InvalidModel);
            }
        }

        let mut period_ids = HashSet::with_capacity(self.history_periods.len());
        let mut current_periods = 0usize;
        for period in &self.history_periods {
            if !valid_text(&period.id, MAX_PUBLIC_ID_SCALARS)
                || !period_ids.insert(period.id.as_str())
                || !valid_timestamp(period.start_at)
                || !valid_timestamp(period.end_at)
                || period.end_at < period.start_at
                || !valid_text(&period.label, MAX_PUBLIC_HISTORY_LABEL_SCALARS)
                || period.label.is_empty()
            {
                return Err(ApiSnapshotError::InvalidHistoryPeriod);
            }
            if period.current {
                current_periods = current_periods.saturating_add(1);
            }
        }
        if current_periods > 1 {
            return Err(ApiSnapshotError::InvalidHistoryPeriod);
        }

        for sample in &self.history_samples {
            if !valid_timestamp(sample.timestamp)
                || !valid_timestamp(sample.reset_at)
                || sample
                    .remaining_percent
                    .is_some_and(|value| !value.is_finite() || !(0.0..=100.0).contains(&value))
                || !valid_non_negative_rate(sample.sol_dollars)
                || !valid_non_negative_rate(sample.terra_dollars)
                || !valid_non_negative_rate(sample.luna_dollars)
            {
                return Err(ApiSnapshotError::InvalidHistorySample);
            }
        }

        let mut thread_ids = HashSet::with_capacity(self.threads.len());
        for thread in &self.threads {
            if !valid_text(&thread.id, MAX_PUBLIC_ID_SCALARS)
                || !thread_ids.insert(thread.id.as_str())
                || !valid_text(&thread.title, security::MAX_THREAD_TITLE_SCALARS)
                || thread.title.is_empty()
                || !valid_text(&thread.model, security::MAX_MODEL_SCALARS)
                || thread.model.is_empty()
                || !valid_text(
                    &thread.model_label,
                    security::MAX_ACCOUNT_ACTIVITY_LABEL_SCALARS,
                )
                || thread.model_label.is_empty()
                || !thread
                    .parent_thread_id
                    .as_deref()
                    .is_none_or(|id| valid_text(id, MAX_PUBLIC_ID_SCALARS) && !id.is_empty())
                || !thread.created_at.is_none_or(valid_timestamp)
                || !thread.last_user_message_at.is_none_or(valid_timestamp)
                || !thread.depth.is_none_or(|depth| (0..=1024).contains(&depth))
            {
                return Err(ApiSnapshotError::InvalidThread);
            }
        }
        if !valid_text(&self.estimated_cost_label, security::MAX_STATUS_SCALARS)
            || self.estimated_cost_label.is_empty()
        {
            return Err(ApiSnapshotError::InvalidLabel);
        }
        Ok(())
    }
}

fn valid_timestamp(value: i64) -> bool {
    (1..=MAX_PUBLIC_UNIX_SECONDS).contains(&value)
}

fn valid_non_negative_rate(value: f64) -> bool {
    value.is_finite() && value >= 0.0
}

fn valid_text(value: &str, max_scalars: usize) -> bool {
    !value.is_empty()
        && value.chars().count() <= max_scalars
        && !value.chars().any(char::is_control)
}

impl PublicSnapshot {
    fn validate(&self) -> Result<(), ApiSnapshotError> {
        if self
            .observed_at
            .is_some_and(|timestamp| !valid_timestamp(timestamp))
        {
            return Err(ApiSnapshotError::InvalidObservedAt);
        }
        if let Some(quota) = self.quota.as_ref() {
            if !quota.remaining_percent.is_finite()
                || !(0.0..=100.0).contains(&quota.remaining_percent)
                || !valid_timestamp(quota.reset_at)
                || quota.window_seconds <= 0
            {
                return Err(ApiSnapshotError::InvalidQuota);
            }
        }
        if self.models.len() > MAX_PUBLIC_MODELS {
            return Err(ApiSnapshotError::ListTooLong);
        }
        let mut model_names = HashSet::with_capacity(self.models.len());
        if self.models.iter().any(|model| {
            !matches!(model.name.as_str(), "SOL" | "TERRA" | "LUNA")
                || !model_names.insert(model.name.as_str())
        }) {
            return Err(ApiSnapshotError::InvalidModel);
        }
        if self
            .plan_label
            .as_deref()
            .is_some_and(|label| !valid_text(label, security::MAX_PLAN_SCALARS))
        {
            return Err(ApiSnapshotError::InvalidLabel);
        }
        Ok(())
    }
}

/// A redacted validation error for data that would leave the process.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ApiSnapshotError {
    InvalidObservedAt,
    InvalidQuota,
    InvalidModel,
    InvalidLabel,
    InvalidHistoryPeriod,
    InvalidHistorySample,
    InvalidThread,
    ListTooLong,
}

impl fmt::Display for ApiSnapshotError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let message = match self {
            Self::InvalidObservedAt => "public snapshot has an invalid observation time",
            Self::InvalidQuota => "public snapshot has an invalid quota",
            Self::InvalidModel => "public snapshot has an invalid model",
            Self::InvalidLabel => "public snapshot has an invalid label",
            Self::InvalidHistoryPeriod => "public snapshot has an invalid history period",
            Self::InvalidHistorySample => "public snapshot has an invalid history sample",
            Self::InvalidThread => "public snapshot has an invalid thread",
            Self::ListTooLong => "public snapshot has too many rows",
        };
        formatter.write_str(message)
    }
}

impl std::error::Error for ApiSnapshotError {}

#[derive(Clone, Debug, Default, PartialEq)]
struct PublishedSnapshot {
    status: PublicSnapshot,
    details: PublicDetails,
}

type SharedSnapshot = Arc<RwLock<PublishedSnapshot>>;

/// Cloneable one-way publication handle held by the UI thread.
#[derive(Clone)]
pub struct ApiSnapshotPublisher {
    snapshot: SharedSnapshot,
}

impl ApiSnapshotPublisher {
    /// Replaces the entire public snapshot only after its finite whitelist has
    /// been validated. The previous snapshot remains available on failure.
    pub fn publish(&self, snapshot: PublicSnapshot) -> Result<(), ApiSnapshotError> {
        snapshot.validate()?;
        let details = PublicDetails::from_snapshot(&snapshot);
        details.validate()?;
        let mut current = self
            .snapshot
            .write()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        *current = PublishedSnapshot {
            status: snapshot,
            details,
        };
        Ok(())
    }

    /// Atomically publishes the status and all additive details. The status
    /// projection is derived from the same document, so `/status` and
    /// `/details` can never observe different account/quota generations.
    pub fn publish_details(&self, details: PublicDetails) -> Result<(), ApiSnapshotError> {
        details.validate()?;
        let status = details.status_snapshot();
        let mut current = self
            .snapshot
            .write()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        *current = PublishedSnapshot { status, details };
        Ok(())
    }
}

/// A listener configuration that can never represent a LAN or public bind.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ApiServerConfig {
    listen_addr: SocketAddr,
}

impl ApiServerConfig {
    pub fn new(listen_addr: SocketAddr) -> Result<Self, ApiServerError> {
        if !is_loopback(listen_addr.ip()) {
            return Err(ApiServerError::NonLoopbackAddress);
        }
        Ok(Self { listen_addr })
    }

    /// Parses the opt-in listener. An unset variable keeps the API disabled.
    pub fn from_environment() -> Result<Option<Self>, ApiServerError> {
        let Some(value) = env::var_os(API_LISTEN_ENV) else {
            return Ok(None);
        };
        let value = value
            .to_str()
            .ok_or(ApiServerError::InvalidListenConfiguration)?;
        let address = value
            .parse::<SocketAddr>()
            .map_err(|_| ApiServerError::InvalidListenConfiguration)?;
        Self::new(address).map(Some)
    }

    pub const fn listen_addr(self) -> SocketAddr {
        self.listen_addr
    }
}

fn is_loopback(address: IpAddr) -> bool {
    address.is_loopback()
}

/// Redacted errors for starting the optional API listener.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ApiServerError {
    InvalidListenConfiguration,
    NonLoopbackAddress,
    BindFailed,
    RuntimeFailed,
    WorkerStartFailed,
}

impl fmt::Display for ApiServerError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let message = match self {
            Self::InvalidListenConfiguration => "API listen configuration is invalid",
            Self::NonLoopbackAddress => "API listener must use a loopback address",
            Self::BindFailed => "API listener could not bind safely",
            Self::RuntimeFailed => "API runtime could not start",
            Self::WorkerStartFailed => "API worker could not start",
        };
        formatter.write_str(message)
    }
}

impl std::error::Error for ApiServerError {}

/// An optional API worker. Dropping it closes the listener and joins its
/// thread; it owns no Codex child process and never accesses UI state.
pub struct ApiServer {
    publisher: ApiSnapshotPublisher,
    local_addr: SocketAddr,
    shutdown: Option<oneshot::Sender<()>>,
    worker: Option<JoinHandle<()>>,
}

impl ApiServer {
    pub fn from_environment() -> Result<Option<Self>, ApiServerError> {
        ApiServerConfig::from_environment()?
            .map(Self::start)
            .transpose()
    }

    pub fn start(config: ApiServerConfig) -> Result<Self, ApiServerError> {
        let listener =
            TcpListener::bind(config.listen_addr).map_err(|_| ApiServerError::BindFailed)?;
        listener
            .set_nonblocking(true)
            .map_err(|_| ApiServerError::BindFailed)?;
        let local_addr = listener
            .local_addr()
            .map_err(|_| ApiServerError::BindFailed)?;
        let publisher = ApiSnapshotPublisher {
            snapshot: Arc::new(RwLock::new(PublishedSnapshot::default())),
        };
        let snapshot = Arc::clone(&publisher.snapshot);
        let (shutdown, shutdown_receiver) = oneshot::channel();
        let (started, started_receiver) = mpsc::sync_channel(1);
        let worker = thread::Builder::new()
            .name("codex-info-api".into())
            .spawn(move || {
                let runtime = match tokio::runtime::Builder::new_current_thread()
                    .enable_io()
                    .build()
                {
                    Ok(runtime) => runtime,
                    Err(_) => {
                        let _ = started.send(Err(ApiServerError::RuntimeFailed));
                        return;
                    }
                };
                runtime.block_on(async move {
                    let listener = match TokioTcpListener::from_std(listener) {
                        Ok(listener) => listener,
                        Err(_) => {
                            let _ = started.send(Err(ApiServerError::RuntimeFailed));
                            return;
                        }
                    };
                    let app = router(snapshot);
                    if started.send(Ok(())).is_err() {
                        return;
                    }
                    // Snapshot responses are idempotent GETs. On shutdown we
                    // stop accepting immediately, so an unavailable Windows
                    // client simply reconnects to the next server instance.
                    tokio::select! {
                        _ = axum::serve(listener, app) => {}
                        _ = shutdown_receiver => {}
                    }
                });
            })
            .map_err(|_| ApiServerError::WorkerStartFailed)?;

        match started_receiver.recv_timeout(API_START_TIMEOUT) {
            Ok(Ok(())) => Ok(Self {
                publisher,
                local_addr,
                shutdown: Some(shutdown),
                worker: Some(worker),
            }),
            Ok(Err(error)) => {
                let _ = shutdown.send(());
                let _ = worker.join();
                Err(error)
            }
            Err(_) => {
                let _ = shutdown.send(());
                let _ = worker.join();
                Err(ApiServerError::WorkerStartFailed)
            }
        }
    }

    pub fn publisher(&self) -> ApiSnapshotPublisher {
        self.publisher.clone()
    }

    pub const fn local_addr(&self) -> SocketAddr {
        self.local_addr
    }

    /// Stops the worker and releases the loopback port. Calling it more than
    /// once is harmless.
    pub fn shutdown(&mut self) {
        if let Some(shutdown) = self.shutdown.take() {
            let _ = shutdown.send(());
        }
        if let Some(worker) = self.worker.take() {
            let _ = worker.join();
        }
    }
}

impl Drop for ApiServer {
    fn drop(&mut self) {
        self.shutdown();
    }
}

#[derive(Serialize)]
struct HealthResponse {
    api_version: &'static str,
    service: &'static str,
}

#[derive(Serialize)]
struct StatusResponse {
    api_version: &'static str,
    #[serde(flatten)]
    snapshot: PublicSnapshot,
}

#[derive(Serialize)]
struct DetailsResponse {
    api_version: &'static str,
    #[serde(flatten)]
    details: PublicDetails,
}

#[derive(Serialize)]
struct ErrorResponse {
    api_version: &'static str,
    error: &'static str,
}

fn router(snapshot: SharedSnapshot) -> Router {
    Router::new()
        .route("/v1/health", get(health).fallback(method_not_allowed))
        .route("/v1/status", get(status).fallback(method_not_allowed))
        .route("/v1/details", get(details).fallback(method_not_allowed))
        .fallback(not_found)
        .with_state(snapshot)
}

async fn health() -> Response {
    json_response(
        StatusCode::OK,
        HealthResponse {
            api_version: API_VERSION,
            service: "codex-info",
        },
    )
}

async fn status(State(snapshot): State<SharedSnapshot>) -> Response {
    let snapshot = snapshot
        .read()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
        .status
        .clone();
    json_response(
        StatusCode::OK,
        StatusResponse {
            api_version: API_VERSION,
            snapshot,
        },
    )
}

async fn details(State(snapshot): State<SharedSnapshot>) -> Response {
    let details = snapshot
        .read()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
        .details
        .clone();
    json_response(
        StatusCode::OK,
        DetailsResponse {
            api_version: API_VERSION,
            details,
        },
    )
}

async fn method_not_allowed() -> Response {
    json_response(
        StatusCode::METHOD_NOT_ALLOWED,
        ErrorResponse {
            api_version: API_VERSION,
            error: "method_not_allowed",
        },
    )
}

async fn not_found() -> Response {
    json_response(
        StatusCode::NOT_FOUND,
        ErrorResponse {
            api_version: API_VERSION,
            error: "not_found",
        },
    )
}

fn json_response<T>(status: StatusCode, value: T) -> Response
where
    T: Serialize,
{
    let mut response = (status, Json(value)).into_response();
    let headers = response.headers_mut();
    headers.insert(CACHE_CONTROL, HeaderValue::from_static("no-store"));
    headers.insert(
        CONTENT_TYPE,
        HeaderValue::from_static("application/json; charset=utf-8"),
    );
    headers.insert(CONNECTION, HeaderValue::from_static("close"));
    response
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::Value;
    use std::io::{Read, Write};
    use std::net::TcpStream;
    use std::sync::{Mutex, OnceLock};

    fn environment_lock() -> &'static Mutex<()> {
        static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
        LOCK.get_or_init(|| Mutex::new(()))
    }

    fn api_server_test_lock() -> &'static Mutex<()> {
        static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
        LOCK.get_or_init(|| Mutex::new(()))
    }

    fn loopback_config() -> ApiServerConfig {
        ApiServerConfig::new(SocketAddr::from(([127, 0, 0, 1], 0))).unwrap()
    }

    fn wire_request(address: SocketAddr, request: &str) -> String {
        let mut stream = TcpStream::connect(address).unwrap();
        stream
            .set_read_timeout(Some(Duration::from_secs(2)))
            .unwrap();
        stream.write_all(request.as_bytes()).unwrap();
        stream.flush().unwrap();
        let mut response = String::new();
        stream.read_to_string(&mut response).unwrap();
        response
    }

    fn body(response: &str) -> Value {
        let (_, body) = response.split_once("\r\n\r\n").unwrap();
        serde_json::from_str(body).unwrap()
    }

    fn detailed_fixture() -> PublicDetails {
        PublicDetails {
            state: PublicState::Ready,
            observed_at: Some(1_780_000_000),
            authenticated: true,
            plan_label: Some("Pro".into()),
            quota: Some(PublicQuota {
                remaining_percent: 98.0,
                reset_at: 1_780_400_000,
                window_seconds: 604_800,
                monthly: false,
            }),
            models: vec![PublicDetailedModelUsage {
                name: "SOL".into(),
                input_tokens: 900,
                cached_input_tokens: 300,
                output_tokens: 400,
                input_dollars: 0.0045,
                cached_input_dollars: 0.00015,
                output_dollars: 0.012,
            }],
            active_thread_count: 1,
            history_periods: vec![PublicHistoryPeriod {
                id: "1780400000".into(),
                start_at: 1_779_395_200,
                end_at: 1_780_400_000,
                label: "2026/06/01 — 2026/06/08".into(),
                current: true,
            }],
            history_samples: vec![PublicHistorySample {
                timestamp: 1_780_000_000,
                reset_at: 1_780_400_000,
                remaining_percent: None,
                sol_dollars: 0.01665,
                terra_dollars: 0.0,
                luna_dollars: 0.0,
                sol_tokens: 1_600,
                terra_tokens: 0,
                luna_tokens: 0,
            }],
            threads: vec![PublicThread {
                id: "thread-1".into(),
                title: "安全な読み取り確認".into(),
                parent_thread_id: None,
                model: "gpt-5.6-sol".into(),
                model_label: "gpt-5.6-sol".into(),
                total_tokens: Some(1_600),
                context_usage_tokens: Some(1_200),
                context_window_tokens: Some(258_400),
                created_at: Some(1_779_999_000),
                last_user_message_at: Some(1_779_999_900),
                is_subagent: false,
                depth: None,
            }],
            estimated_cost_label: "概算 $1".into(),
        }
    }

    #[test]
    fn environment_is_disabled_when_listen_is_unset() {
        let _guard = environment_lock().lock().unwrap();
        let previous = env::var_os(API_LISTEN_ENV);
        env::remove_var(API_LISTEN_ENV);
        assert_eq!(ApiServerConfig::from_environment().unwrap(), None);
        match previous {
            Some(value) => env::set_var(API_LISTEN_ENV, value),
            None => env::remove_var(API_LISTEN_ENV),
        }
    }

    #[test]
    fn configuration_rejects_non_loopback_or_non_numeric_addresses() {
        assert_eq!(
            ApiServerConfig::new("0.0.0.0:8787".parse().unwrap()),
            Err(ApiServerError::NonLoopbackAddress)
        );
        assert_eq!(
            ApiServerConfig::new("192.168.1.7:8787".parse().unwrap()),
            Err(ApiServerError::NonLoopbackAddress)
        );
        assert_eq!(
            ApiServerConfig::new("[::]:8787".parse().unwrap()),
            Err(ApiServerError::NonLoopbackAddress)
        );
        assert!(ApiServerConfig::new("[::1]:8787".parse().unwrap()).is_ok());
        let _guard = environment_lock().lock().unwrap();
        let previous = env::var_os(API_LISTEN_ENV);
        env::set_var(API_LISTEN_ENV, "localhost:8787");
        assert_eq!(
            ApiServerConfig::from_environment(),
            Err(ApiServerError::InvalidListenConfiguration)
        );
        match previous {
            Some(value) => env::set_var(API_LISTEN_ENV, value),
            None => env::remove_var(API_LISTEN_ENV),
        }
    }

    #[test]
    fn health_status_errors_and_snapshot_are_json_no_store() {
        // All API tests use an ephemeral loopback port. Serialize their server
        // lifetimes so another test cannot claim this test's just-released
        // port between shutdown and the explicit rebind assertion.
        let _guard = api_server_test_lock()
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        let mut server = ApiServer::start(loopback_config()).unwrap();
        let publisher = server.publisher();
        publisher
            .publish(PublicSnapshot {
                state: PublicState::Ready,
                observed_at: Some(1_780_000_000),
                authenticated: true,
                plan_label: Some("Pro".into()),
                quota: Some(PublicQuota {
                    remaining_percent: 98.0,
                    reset_at: 1_780_400_000,
                    window_seconds: 604_800,
                    monthly: false,
                }),
                models: vec![PublicModelUsage {
                    name: "SOL".into(),
                    input_tokens: 1_200,
                    cached_input_tokens: 300,
                    output_tokens: 400,
                }],
                active_thread_count: 1,
            })
            .unwrap();

        let health = wire_request(
            server.local_addr(),
            "GET /v1/health HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\n\r\n",
        );
        assert!(health.starts_with("HTTP/1.1 200"), "response: {health:?}");
        assert!(health.contains("cache-control: no-store"));
        assert_eq!(body(&health)["api_version"], "v1");

        let status = wire_request(
            server.local_addr(),
            "GET /v1/status HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\n\r\n",
        );
        assert!(status.starts_with("HTTP/1.1 200"));
        assert!(status.contains("cache-control: no-store"));
        let status_body = body(&status);
        assert_eq!(status_body["state"], "ready");
        assert_eq!(status_body["quota"]["remaining_percent"], 98.0);
        assert_eq!(status_body["models"][0]["input_tokens"], 1200);
        let status_keys = status_body.as_object().unwrap().keys().collect::<Vec<_>>();
        assert_eq!(
            status_keys,
            vec![
                "active_thread_count",
                "api_version",
                "authenticated",
                "models",
                "observed_at",
                "plan_label",
                "quota",
                "state",
            ]
        );
        assert!(status_body.get("email").is_none());
        assert!(status_body.get("auth_url").is_none());
        assert!(status_body.get("error_detail").is_none());

        let missing = wire_request(
            server.local_addr(),
            "GET /v1/missing HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\n\r\n",
        );
        assert!(missing.starts_with("HTTP/1.1 404"));
        assert_eq!(body(&missing)["error"], "not_found");

        let wrong_method = wire_request(
            server.local_addr(),
            "POST /v1/status HTTP/1.1\r\nHost: localhost\r\nContent-Length: 0\r\nConnection: close\r\n\r\n",
        );
        assert!(wrong_method.starts_with("HTTP/1.1 405"));
        assert_eq!(body(&wrong_method)["error"], "method_not_allowed");

        let wrong_details_method = wire_request(
            server.local_addr(),
            "POST /v1/details HTTP/1.1\r\nHost: localhost\r\nContent-Length: 0\r\nConnection: close\r\n\r\n",
        );
        assert!(wrong_details_method.starts_with("HTTP/1.1 405"));
        server.shutdown();
    }

    #[test]
    fn details_endpoint_publishes_additive_fields_without_status_drift() {
        let _guard = api_server_test_lock()
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        let mut server = ApiServer::start(loopback_config()).unwrap();
        server
            .publisher()
            .publish_details(detailed_fixture())
            .unwrap();

        let details = wire_request(
            server.local_addr(),
            "GET /v1/details HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\n\r\n",
        );
        assert!(details.starts_with("HTTP/1.1 200"));
        assert!(details.contains("cache-control: no-store"));
        let details_body = body(&details);
        assert_eq!(details_body["api_version"], "v1");
        assert_eq!(details_body["models"][0]["input_dollars"], 0.0045);
        assert!(details_body["history_samples"][0]["remaining_percent"].is_null());
        assert_eq!(details_body["history_periods"][0]["id"], "1780400000");
        assert_eq!(details_body["threads"][0]["context_window_tokens"], 258400);
        assert!(details_body.get("email").is_none());
        assert!(details_body.get("auth_url").is_none());
        assert!(details_body.get("error_detail").is_none());

        let status = wire_request(
            server.local_addr(),
            "GET /v1/status HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\n\r\n",
        );
        assert_eq!(body(&status)["models"][0].as_object().unwrap().len(), 4);
        assert!(body(&status)["history_periods"].is_null());
        server.shutdown();
    }

    #[test]
    fn invalid_publication_keeps_the_last_snapshot() {
        let _guard = api_server_test_lock()
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        let mut server = ApiServer::start(loopback_config()).unwrap();
        let publisher = server.publisher();
        let initial = PublicSnapshot {
            state: PublicState::AuthRequired,
            ..PublicSnapshot::default()
        };
        publisher.publish(initial).unwrap();
        let invalid = PublicSnapshot {
            quota: Some(PublicQuota {
                remaining_percent: 101.0,
                reset_at: 1,
                window_seconds: 1,
                monthly: false,
            }),
            ..PublicSnapshot::default()
        };
        assert_eq!(
            publisher.publish(invalid),
            Err(ApiSnapshotError::InvalidQuota)
        );
        let status = wire_request(
            server.local_addr(),
            "GET /v1/status HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\n\r\n",
        );
        assert_eq!(body(&status)["state"], "auth_required");
        server.shutdown();
    }

    #[test]
    fn invalid_details_keep_the_last_atomic_generation() {
        let _guard = api_server_test_lock()
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        let mut server = ApiServer::start(loopback_config()).unwrap();
        let publisher = server.publisher();
        publisher.publish_details(detailed_fixture()).unwrap();
        let mut invalid = detailed_fixture();
        invalid.models[0].output_dollars = f64::NAN;
        assert_eq!(
            publisher.publish_details(invalid),
            Err(ApiSnapshotError::InvalidModel)
        );
        let details = wire_request(
            server.local_addr(),
            "GET /v1/details HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\n\r\n",
        );
        assert_eq!(body(&details)["estimated_cost_label"], "概算 $1");
        server.shutdown();
    }

    #[test]
    fn detail_validation_is_bounded_for_times_rates_lists_models_and_text() {
        let mut invalid = detailed_fixture();
        invalid.history_samples[0].timestamp = 0;
        assert_eq!(
            invalid.validate(),
            Err(ApiSnapshotError::InvalidHistorySample)
        );

        let mut invalid = detailed_fixture();
        invalid.history_samples[0].sol_dollars = f64::INFINITY;
        assert_eq!(
            invalid.validate(),
            Err(ApiSnapshotError::InvalidHistorySample)
        );

        let mut invalid = detailed_fixture();
        invalid.history_periods[0].label = "x".repeat(513);
        assert_eq!(
            invalid.validate(),
            Err(ApiSnapshotError::InvalidHistoryPeriod)
        );

        let mut invalid = detailed_fixture();
        invalid.threads[0].title = "x".repeat(513);
        assert_eq!(invalid.validate(), Err(ApiSnapshotError::InvalidThread));

        let mut invalid = detailed_fixture();
        invalid.models = vec![invalid.models[0].clone(); MAX_PUBLIC_MODELS + 1];
        assert_eq!(invalid.validate(), Err(ApiSnapshotError::ListTooLong));

        let mut invalid = detailed_fixture();
        invalid.models[0].name = "OTHER".into();
        assert_eq!(invalid.validate(), Err(ApiSnapshotError::InvalidModel));
    }

    #[test]
    fn shutdown_releases_the_bound_port() {
        let _guard = api_server_test_lock()
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        let mut server = ApiServer::start(loopback_config()).unwrap();
        let address = server.local_addr();
        let conflicting = ApiServerConfig::new(address).unwrap();
        assert_eq!(
            ApiServer::start(conflicting).err(),
            Some(ApiServerError::BindFailed)
        );
        server.shutdown();
        server.shutdown();
        let rebound = TcpListener::bind(address);
        assert!(rebound.is_ok());
    }
}
