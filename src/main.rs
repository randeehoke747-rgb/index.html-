use axum::{
    extract::{Path, State},
    http::{header, HeaderMap, StatusCode},
    routing::{get, post},
    Json, Router,
};
use serde_json::json;
use std::{net::SocketAddr, path::PathBuf, sync::Arc};
use titan_core::{
    AgentCapability, AgentHeartbeat, AgentKind, AssignAgentRequest, AuditEvent,
    CloseSessionRequest, CommandEvent, DiscordCommandRequest, HunterConfig, HunterSession,
    MessageRelayRequest, MonitorService, MonitorSnapshot, MonitorStatus, SessionCreateRequest,
    Vulnerability,
};
use tokio::sync::RwLock;
use tracing::info;

#[derive(Clone)]
struct AppState {
    monitor: Arc<RwLock<MonitorService>>,
    operator_api_token: Option<String>,
    discord_bot_token: Option<String>,
    discord_ingest_token: Option<String>,
    monitor_state_path: Option<PathBuf>,
}

#[derive(Clone)]
struct RuntimeConfig {
    operator_api_token: Option<String>,
    discord_bot_token: Option<String>,
    discord_ingest_token: Option<String>,
    monitor_state_path: Option<PathBuf>,
    strict_startup: bool,
}

fn env_token(name: &str) -> Option<String> {
    std::env::var(name)
        .ok()
        .map(|value| value.trim().to_owned())
        .filter(|value| !value.is_empty())
}

fn path_from_env(name: &str) -> Option<PathBuf> {
    env_token(name).map(PathBuf::from)
}

fn runtime_config_from_env() -> RuntimeConfig {
    RuntimeConfig {
        operator_api_token: env_token("OPERATOR_API_TOKEN"),
        discord_bot_token: env_token("DISCORD_BOT_TOKEN"),
        discord_ingest_token: env_token("DISCORD_INGEST_TOKEN"),
        monitor_state_path: path_from_env("MONITOR_STATE_PATH"),
        strict_startup: strict_startup_from_env(),
    }
}

fn strict_startup_from_env() -> bool {
    matches!(
        std::env::var("STRICT_STARTUP")
            .ok()
            .as_deref()
            .map(str::trim),
        Some("1" | "true" | "TRUE" | "yes" | "YES")
    )
}

fn parse_csv_env(name: &str) -> Vec<String> {
    std::env::var(name)
        .ok()
        .map(|value| {
            value
                .split(',')
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .map(ToOwned::to_owned)
                .collect()
        })
        .unwrap_or_default()
}

fn hunter_config_from_env() -> HunterConfig {
    HunterConfig {
        hunter_identity: env_token("HUNTER_IDENTITY").unwrap_or_else(|| "hunter-prime".into()),
        allowed_guilds: parse_csv_env("ALLOWED_GUILDS"),
        allowed_channels: parse_csv_env("ALLOWED_CHANNELS"),
        minimum_active_agents: std::env::var("MINIMUM_ACTIVE_AGENTS")
            .ok()
            .and_then(|value| value.parse::<usize>().ok())
            .unwrap_or(2),
    }
}

fn configuration_errors(config: &HunterConfig, runtime_config: &RuntimeConfig) -> Vec<String> {
    let mut errors = Vec::new();
    if config.hunter_identity.trim().is_empty() {
        errors.push("HUNTER_IDENTITY is not configured.".to_owned());
    }
    if runtime_config.discord_bot_token.is_none() {
        errors.push("DISCORD_BOT_TOKEN is not configured.".to_owned());
    }
    if runtime_config.operator_api_token.is_none() {
        errors.push("OPERATOR_API_TOKEN is not configured.".to_owned());
    }
    if runtime_config.discord_ingest_token.is_none() {
        errors.push("DISCORD_INGEST_TOKEN is not configured.".to_owned());
    }
    errors
}

fn expected_bearer_token(headers: &HeaderMap) -> Option<&str> {
    let value = headers.get(header::AUTHORIZATION)?.to_str().ok()?.trim();
    let mut parts = value.splitn(2, char::is_whitespace);
    let scheme = parts.next()?;
    if !scheme.eq_ignore_ascii_case("bearer") {
        return None;
    }
    let token = parts.next()?.trim();
    if token.is_empty() {
        None
    } else {
        Some(token)
    }
}

fn secure_token_eq(expected: &str, provided: &str) -> bool {
    let expected = expected.as_bytes();
    let provided = provided.as_bytes();
    let mut diff = expected.len() ^ provided.len();
    let max_len = expected.len().max(provided.len());

    for index in 0..max_len {
        let expected_byte = expected.get(index).copied().unwrap_or_default();
        let provided_byte = provided.get(index).copied().unwrap_or_default();
        diff |= usize::from(expected_byte ^ provided_byte);
    }

    diff == 0
}

fn is_authorized(headers: &HeaderMap, expected_token: Option<&str>) -> bool {
    match expected_token {
        Some(expected_token) => expected_bearer_token(headers)
            .map(|provided_token| secure_token_eq(expected_token, provided_token))
            .unwrap_or(false),
        None => false,
    }
}

#[cfg(test)]
mod tests {
    use super::{expected_bearer_token, is_authorized, secure_token_eq};
    use axum::http::{header, HeaderMap, HeaderValue};

    fn headers_with_authorization(value: &str) -> HeaderMap {
        let mut headers = HeaderMap::new();
        headers.insert(
            header::AUTHORIZATION,
            HeaderValue::from_str(value).expect("valid header"),
        );
        headers
    }

    #[test]
    fn bearer_token_parsing_accepts_standard_bearer_header() {
        let header = format!("{} {}", "Bearer", "operator-token");
        let headers = headers_with_authorization(&header);
        assert_eq!(expected_bearer_token(&headers), Some("operator-token"));
    }

    #[test]
    fn bearer_token_parsing_accepts_case_insensitive_scheme_and_extra_spaces() {
        let headers = headers_with_authorization("   bearer    ingest-token   ");
        assert_eq!(expected_bearer_token(&headers), Some("ingest-token"));
    }

    #[test]
    fn bearer_token_parsing_rejects_missing_or_empty_token() {
        let headers = headers_with_authorization("Bearer   ");
        assert_eq!(expected_bearer_token(&headers), None);
    }

    #[test]
    fn bearer_token_parsing_rejects_non_bearer_scheme() {
        let headers = headers_with_authorization("Basic abc123");
        assert_eq!(expected_bearer_token(&headers), None);
    }

    #[test]
    fn secure_token_comparison_matches_identical_tokens_only() {
        assert!(secure_token_eq("same-token", "same-token"));
        assert!(!secure_token_eq("same-token", "different-token"));
        assert!(!secure_token_eq("same-token", "same-token-extra"));
    }

    #[test]
    fn authorization_requires_matching_expected_bearer_token() {
        let header = format!("{} {}", "Bearer", "secret-token");
        let headers = headers_with_authorization(&header);
        assert!(is_authorized(&headers, Some("secret-token")));
        assert!(!is_authorized(&headers, Some("other-token")));
        assert!(!is_authorized(&headers, None));
    }
}

async fn persist_snapshot(path: &PathBuf, snapshot: &MonitorSnapshot) -> Result<(), StatusCode> {
    if let Some(parent) = path.parent() {
        tokio::fs::create_dir_all(parent)
            .await
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    }
    let body =
        serde_json::to_vec_pretty(snapshot).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    tokio::fs::write(path, body)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)
}

async fn health(State(state): State<AppState>) -> Json<serde_json::Value> {
    let monitor = state.monitor.read().await;
    Json(json!({
        "status": "ok",
        "service": "hunter-clone-control-plane",
        "ready": monitor.is_ready(state.discord_bot_token.is_some()),
        "active_agents": monitor.active_agents(),
        "available_agents": monitor.available_agents(),
    }))
}

async fn ready(State(state): State<AppState>) -> Result<Json<serde_json::Value>, StatusCode> {
    let monitor = state.monitor.read().await;
    if monitor.is_ready(state.discord_bot_token.is_some()) {
        Ok(Json(json!({
            "status": "ready"
        })))
    } else {
        Err(StatusCode::SERVICE_UNAVAILABLE)
    }
}

async fn status(State(state): State<AppState>) -> Json<serde_json::Value> {
    let monitor = state.monitor.read().await;
    let monitor_status: MonitorStatus = monitor.status(state.discord_bot_token.is_some());
    let config = hunter_config_from_env();
    let runtime_config = RuntimeConfig {
        operator_api_token: state.operator_api_token.clone(),
        discord_bot_token: state.discord_bot_token.clone(),
        discord_ingest_token: state.discord_ingest_token.clone(),
        monitor_state_path: state.monitor_state_path.clone(),
        strict_startup: strict_startup_from_env(),
    };
    Json(json!({
        "monitor": monitor_status,
        "operator_auth_configured": state.operator_api_token.is_some(),
        "discord_ingest_auth_configured": state.discord_ingest_token.is_some(),
        "persistence_enabled": state.monitor_state_path.is_some(),
        "strict_startup": runtime_config.strict_startup,
        "configuration_errors": configuration_errors(&config, &runtime_config),
    }))
}

async fn history(State(state): State<AppState>) -> Json<Vec<AuditEvent>> {
    let monitor = state.monitor.read().await;
    Json(monitor.audit_history().to_vec())
}

async fn commands(State(state): State<AppState>) -> Json<Vec<CommandEvent>> {
    let monitor = state.monitor.read().await;
    Json(monitor.command_events().to_vec())
}

async fn sessions(State(state): State<AppState>) -> Json<Vec<HunterSession>> {
    let monitor = state.monitor.read().await;
    Json(monitor.sessions())
}

async fn create_session(
    State(state): State<AppState>,
    Json(payload): Json<SessionCreateRequest>,
) -> Result<(StatusCode, Json<HunterSession>), (StatusCode, Json<Vec<Vulnerability>>)> {
    let mut monitor = state.monitor.write().await;
    let session = monitor
        .create_session(payload)
        .map_err(|findings| (StatusCode::FORBIDDEN, Json(findings)))?;
    let snapshot = state
        .monitor_state_path
        .as_ref()
        .map(|_| monitor.snapshot());
    drop(monitor);
    if let (Some(path), Some(snapshot)) = (&state.monitor_state_path, snapshot.as_ref()) {
        persist_snapshot(path, snapshot)
            .await
            .map_err(|status| (status, Json(Vec::new())))?;
    }
    Ok((StatusCode::ACCEPTED, Json(session)))
}

async fn assign_session(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(session_id): Path<String>,
    Json(payload): Json<AssignAgentRequest>,
) -> Result<Json<HunterSession>, StatusCode> {
    if !is_authorized(&headers, state.operator_api_token.as_deref()) {
        return Err(StatusCode::UNAUTHORIZED);
    }
    let mut monitor = state.monitor.write().await;
    let session = monitor
        .assign_agent(&session_id, payload)
        .ok_or(StatusCode::NOT_FOUND)?;
    let snapshot = state
        .monitor_state_path
        .as_ref()
        .map(|_| monitor.snapshot());
    drop(monitor);
    if let (Some(path), Some(snapshot)) = (&state.monitor_state_path, snapshot.as_ref()) {
        persist_snapshot(path, snapshot).await?;
    }
    Ok(Json(session))
}

async fn relay_session_message(
    State(state): State<AppState>,
    Path(session_id): Path<String>,
    Json(payload): Json<MessageRelayRequest>,
) -> Result<Json<HunterSession>, (StatusCode, Json<Vec<Vulnerability>>)> {
    let mut monitor = state.monitor.write().await;
    let session = monitor
        .relay_message(&session_id, payload)
        .map_err(|findings| {
            let status = if findings.is_empty() {
                StatusCode::NOT_FOUND
            } else {
                StatusCode::BAD_REQUEST
            };
            (status, Json(findings))
        })?;
    let snapshot = state
        .monitor_state_path
        .as_ref()
        .map(|_| monitor.snapshot());
    drop(monitor);
    if let (Some(path), Some(snapshot)) = (&state.monitor_state_path, snapshot.as_ref()) {
        persist_snapshot(path, snapshot)
            .await
            .map_err(|status| (status, Json(Vec::new())))?;
    }
    Ok(Json(session))
}

async fn close_session(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(session_id): Path<String>,
    Json(payload): Json<CloseSessionRequest>,
) -> Result<Json<HunterSession>, StatusCode> {
    if !is_authorized(&headers, state.operator_api_token.as_deref()) {
        return Err(StatusCode::UNAUTHORIZED);
    }
    let mut monitor = state.monitor.write().await;
    let session = monitor
        .close_session(&session_id, payload)
        .ok_or(StatusCode::NOT_FOUND)?;
    let snapshot = state
        .monitor_state_path
        .as_ref()
        .map(|_| monitor.snapshot());
    drop(monitor);
    if let (Some(path), Some(snapshot)) = (&state.monitor_state_path, snapshot.as_ref()) {
        persist_snapshot(path, snapshot).await?;
    }
    Ok(Json(session))
}

async fn register_heartbeat(
    State(state): State<AppState>,
    Json(payload): Json<AgentHeartbeat>,
) -> StatusCode {
    let mut monitor = state.monitor.write().await;
    monitor.register_or_update_agent(payload);
    let snapshot = state
        .monitor_state_path
        .as_ref()
        .map(|_| monitor.snapshot());
    drop(monitor);
    if let (Some(path), Some(snapshot)) = (&state.monitor_state_path, snapshot.as_ref()) {
        if persist_snapshot(path, snapshot).await.is_err() {
            return StatusCode::INTERNAL_SERVER_ERROR;
        }
    }
    StatusCode::ACCEPTED
}

async fn report_agent_failure(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(agent_name): Path<String>,
) -> StatusCode {
    if !is_authorized(&headers, state.operator_api_token.as_deref()) {
        return StatusCode::UNAUTHORIZED;
    }
    let mut monitor = state.monitor.write().await;
    if monitor.mark_agent_failure(&agent_name) {
        let snapshot = state
            .monitor_state_path
            .as_ref()
            .map(|_| monitor.snapshot());
        drop(monitor);
        if let (Some(path), Some(snapshot)) = (&state.monitor_state_path, snapshot.as_ref()) {
            if persist_snapshot(path, snapshot).await.is_err() {
                return StatusCode::INTERNAL_SERVER_ERROR;
            }
        }
        StatusCode::ACCEPTED
    } else {
        StatusCode::NOT_FOUND
    }
}

async fn dispatch_discord_command(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(payload): Json<DiscordCommandRequest>,
) -> Result<Json<titan_core::DiscordDispatch>, (StatusCode, Json<Vec<Vulnerability>>)> {
    if !is_authorized(&headers, state.discord_ingest_token.as_deref()) {
        return Err((StatusCode::UNAUTHORIZED, Json(Vec::new())));
    }
    let mut monitor = state.monitor.write().await;
    let dispatch = monitor.handle_discord_command(payload).map_err(|findings| {
        let status = if findings.is_empty() {
            StatusCode::NOT_FOUND
        } else if findings.iter().any(|finding| finding.code.contains("allowlisted")) {
            StatusCode::FORBIDDEN
        } else {
            StatusCode::BAD_REQUEST
        };
        (status, Json(findings))
    })?;
    let snapshot = state
        .monitor_state_path
        .as_ref()
        .map(|_| monitor.snapshot());
    drop(monitor);
    if let (Some(path), Some(snapshot)) = (&state.monitor_state_path, snapshot.as_ref()) {
        persist_snapshot(path, snapshot)
            .await
            .map_err(|status| (status, Json(Vec::new())))?;
    }
    Ok(Json(dispatch))
}

fn port_from_env() -> u16 {
    std::env::var("PORT")
        .ok()
        .and_then(|value| value.parse::<u16>().ok())
        .unwrap_or(8080)
}

fn parse_kind(value: &str) -> AgentKind {
    match value.trim().to_ascii_lowercase().as_str() {
        "strategist" => AgentKind::Strategist,
        "researcher" => AgentKind::Researcher,
        "builder" => AgentKind::Builder,
        "sentinel" => AgentKind::Sentinel,
        _ => AgentKind::Hunter,
    }
}

fn parse_capability(value: &str) -> Option<AgentCapability> {
    match value.trim().to_ascii_lowercase().as_str() {
        "session_intake" => Some(AgentCapability::SessionIntake),
        "message_relay" => Some(AgentCapability::MessageRelay),
        "task_execution" => Some(AgentCapability::TaskExecution),
        "knowledge_retrieval" => Some(AgentCapability::KnowledgeRetrieval),
        "moderation" => Some(AgentCapability::Moderation),
        _ => None,
    }
}

fn seed_agents(monitor: &mut MonitorService) {
    let definitions = std::env::var("HUNTER_CLONES").unwrap_or_else(|_| {
        "hunter-prime:hunter:session_intake|message_relay;strategist-1:strategist:knowledge_retrieval;builder-1:builder:task_execution".into()
    });

    for definition in definitions.split(';').map(str::trim).filter(|value| !value.is_empty()) {
        let mut parts = definition.split(':');
        let Some(name) = parts.next().map(str::trim).filter(|value| !value.is_empty()) else {
            continue;
        };
        let kind = parts.next().map(parse_kind).unwrap_or(AgentKind::Hunter);
        let capabilities = parts
            .next()
            .map(|value| value.split('|').filter_map(parse_capability).collect())
            .unwrap_or_else(Vec::new);
        monitor.register_or_update_agent(AgentHeartbeat {
            agent_name: name.to_owned(),
            kind,
            capabilities,
            assigned_session_id: None,
        });
    }
}

async fn monitor_from_env(runtime_config: &RuntimeConfig) -> MonitorService {
    let config = hunter_config_from_env();
    let loaded_snapshot = if let Some(path) = &runtime_config.monitor_state_path {
        tokio::fs::read(path)
            .await
            .ok()
            .and_then(|bytes| serde_json::from_slice::<MonitorSnapshot>(&bytes).ok())
    } else {
        None
    };

    if let Some(snapshot) = loaded_snapshot {
        let mut monitor = MonitorService::from_snapshot(snapshot);
        monitor.replace_config(config);
        seed_agents(&mut monitor);
        monitor
    } else {
        let mut monitor = MonitorService::new(config);
        seed_agents(&mut monitor);
        monitor
    }
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(std::env::var("RUST_LOG").unwrap_or_else(|_| "info".into()))
        .init();

    let runtime_config = runtime_config_from_env();
    let hunter_config = hunter_config_from_env();
    let config_errors = configuration_errors(&hunter_config, &runtime_config);
    if runtime_config.strict_startup && !config_errors.is_empty() {
        panic!(
            "strict startup configuration errors: {}",
            config_errors.join(" ")
        );
    }
    let app_state = AppState {
        monitor: Arc::new(RwLock::new(monitor_from_env(&runtime_config).await)),
        operator_api_token: runtime_config.operator_api_token,
        discord_bot_token: runtime_config.discord_bot_token,
        discord_ingest_token: runtime_config.discord_ingest_token,
        monitor_state_path: runtime_config.monitor_state_path,
    };

    let app = Router::new()
        .route("/", get(health))
        .route("/health", get(health))
        .route("/ready", get(ready))
        .route("/status", get(status))
        .route("/history", get(history))
        .route("/commands", get(commands))
        .route("/sessions", get(sessions).post(create_session))
        .route("/sessions/:session_id/assign", post(assign_session))
        .route("/sessions/:session_id/messages", post(relay_session_message))
        .route("/sessions/:session_id/close", post(close_session))
        .route("/agents/heartbeat", post(register_heartbeat))
        .route("/agents/:agent_name/failure", post(report_agent_failure))
        .route("/discord/commands", post(dispatch_discord_command))
        .with_state(app_state);

    let port = port_from_env();
    let addr = SocketAddr::from(([0, 0, 0, 0], port));

    info!("Hunter clone control plane listening on {}", addr);

    let listener = tokio::net::TcpListener::bind(addr)
        .await
        .unwrap_or_else(|error| panic!("failed to bind port {port}: {error}"));

    axum::serve(listener, app)
        .await
        .expect("Hunter clone server stopped");
}
