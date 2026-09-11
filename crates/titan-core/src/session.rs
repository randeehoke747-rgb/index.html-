use serde::{Deserialize, Serialize};

use crate::{DiscordContext, Vulnerability};

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum MessageAuthorKind {
    DiscordUser,
    HunterClone,
    TitanAgent,
    Operator,
    System,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct TranscriptEntry {
    pub author: String,
    pub author_kind: MessageAuthorKind,
    pub body: String,
    pub recorded_at_epoch_ms: u128,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum SessionState {
    Pending,
    Active,
    WaitingForAgent,
    Closed,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct HunterSession {
    pub session_id: String,
    pub clone_name: String,
    pub requested_agent: Option<String>,
    pub assigned_agent: Option<String>,
    pub state: SessionState,
    pub discord_context: DiscordContext,
    pub transcript: Vec<TranscriptEntry>,
    pub findings: Vec<Vulnerability>,
    pub created_at_epoch_ms: u128,
    pub last_activity_epoch_ms: u128,
    pub closed_by: Option<String>,
    pub close_reason: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct SessionCreateRequest {
    pub session_id: Option<String>,
    pub clone_name: String,
    pub requested_agent: Option<String>,
    pub discord_context: DiscordContext,
    pub initial_prompt: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct AssignAgentRequest {
    pub agent_name: String,
    pub operator_id: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct MessageRelayRequest {
    pub author: String,
    pub author_kind: MessageAuthorKind,
    pub body: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct CloseSessionRequest {
    pub operator_id: String,
    pub reason: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct HunterConfig {
    pub hunter_identity: String,
    pub allowed_guilds: Vec<String>,
    pub allowed_channels: Vec<String>,
    pub minimum_active_agents: usize,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct MonitorStatus {
    pub hunter_identity: String,
    pub active_agents: usize,
    pub available_agents: usize,
    pub minimum_active_agents: usize,
    pub ready: bool,
    pub discord_bot_configured: bool,
    pub tracked_sessions: usize,
    pub active_sessions: usize,
    pub closed_sessions: usize,
    pub recorded_commands: usize,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct MonitorSnapshot {
    pub config: HunterConfig,
    pub agents: Vec<crate::HunterClone>,
    pub sessions: Vec<HunterSession>,
    pub audit_events: Vec<AuditEvent>,
    pub command_events: Vec<CommandEvent>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum AuditEventKind {
    SessionCreated,
    SessionAssigned,
    MessageRelayed,
    SessionClosed,
    AgentFailure,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct AuditEvent {
    pub event_id: String,
    pub session_id: Option<String>,
    pub agent_name: Option<String>,
    pub kind: AuditEventKind,
    pub detail: String,
    pub recorded_at_epoch_ms: u128,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct CommandEvent {
    pub event_id: String,
    pub session_id: Option<String>,
    pub command_name: String,
    pub outcome: String,
    pub detail: String,
    pub recorded_at_epoch_ms: u128,
}
