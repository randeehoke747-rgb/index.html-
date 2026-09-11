use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum AgentKind {
    Hunter,
    Strategist,
    Researcher,
    Builder,
    Sentinel,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum AgentCapability {
    SessionIntake,
    MessageRelay,
    TaskExecution,
    KnowledgeRetrieval,
    Moderation,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum AgentStatus {
    Idle,
    Assigned,
    Degraded,
    Offline,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct HunterClone {
    pub name: String,
    pub kind: AgentKind,
    pub capabilities: Vec<AgentCapability>,
    pub status: AgentStatus,
    pub consecutive_failures: u32,
    pub assigned_session_id: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct AgentHeartbeat {
    pub agent_name: String,
    pub kind: AgentKind,
    #[serde(default)]
    pub capabilities: Vec<AgentCapability>,
    pub assigned_session_id: Option<String>,
}

impl HunterClone {
    pub fn new(
        name: impl Into<String>,
        kind: AgentKind,
        capabilities: Vec<AgentCapability>,
    ) -> Self {
        Self {
            name: name.into(),
            kind,
            capabilities,
            status: AgentStatus::Idle,
            consecutive_failures: 0,
            assigned_session_id: None,
        }
    }

    pub fn assign(&mut self, session_id: impl Into<String>) {
        self.status = AgentStatus::Assigned;
        self.assigned_session_id = Some(session_id.into());
        self.consecutive_failures = 0;
    }

    pub fn release(&mut self) {
        self.status = AgentStatus::Idle;
        self.assigned_session_id = None;
    }

    pub fn mark_failure(&mut self) {
        self.consecutive_failures = self.consecutive_failures.saturating_add(1);
        self.assigned_session_id = None;
        self.status = if self.consecutive_failures >= 3 {
            AgentStatus::Offline
        } else {
            AgentStatus::Degraded
        };
    }

    pub fn recover(
        &mut self,
        kind: AgentKind,
        capabilities: Vec<AgentCapability>,
        assigned_session_id: Option<String>,
    ) {
        self.kind = kind;
        self.capabilities = capabilities;
        self.consecutive_failures = 0;
        if let Some(session_id) = assigned_session_id {
            self.assign(session_id);
        } else {
            self.release();
        }
    }

    pub fn is_available(&self) -> bool {
        matches!(
            self.status,
            AgentStatus::Idle | AgentStatus::Assigned | AgentStatus::Degraded
        ) && self.consecutive_failures < 3
    }
}
