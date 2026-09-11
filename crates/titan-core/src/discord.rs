use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct DiscordContext {
    pub guild_id: String,
    pub channel_id: String,
    pub thread_id: Option<String>,
    pub user_id: String,
    pub message_id: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum DiscordCommandKind {
    SpawnClone,
    AssignAgent,
    RelayMessage,
    CloseSession,
    SessionStatus,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct DiscordCommandRequest {
    pub command: DiscordCommandKind,
    pub context: DiscordContext,
    pub session_id: Option<String>,
    pub clone_name: Option<String>,
    pub requested_agent: Option<String>,
    pub agent_name: Option<String>,
    pub content: Option<String>,
    pub operator_id: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct DiscordDispatch {
    pub acknowledged: bool,
    pub session_id: Option<String>,
    pub response: String,
}
