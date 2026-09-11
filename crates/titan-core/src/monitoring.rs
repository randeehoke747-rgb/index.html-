use std::{
    collections::HashMap,
    time::{SystemTime, UNIX_EPOCH},
};

use crate::{
    AgentHeartbeat, AgentStatus, AssignAgentRequest, AuditEvent, AuditEventKind,
    CloseSessionRequest, CommandEvent, CommandSafetyInspector, DiscordCommandKind,
    DiscordCommandRequest, DiscordContext, DiscordDispatch, HunterClone, HunterConfig,
    HunterSession, MessageAuthorKind, MessageRelayRequest, MonitorSnapshot, MonitorStatus,
    SessionCreateRequest, SessionState, TranscriptEntry,
};

#[derive(Clone, Debug)]
pub struct MonitorService {
    config: HunterConfig,
    inspector: CommandSafetyInspector,
    agents: HashMap<String, HunterClone>,
    sessions: HashMap<String, HunterSession>,
    audit_events: Vec<AuditEvent>,
    command_events: Vec<CommandEvent>,
}

impl MonitorService {
    pub fn new(config: HunterConfig) -> Self {
        Self {
            config,
            inspector: CommandSafetyInspector::new(),
            agents: HashMap::new(),
            sessions: HashMap::new(),
            audit_events: Vec::new(),
            command_events: Vec::new(),
        }
    }

    pub fn from_snapshot(snapshot: MonitorSnapshot) -> Self {
        let mut service = Self::new(snapshot.config);
        service.agents = snapshot
            .agents
            .into_iter()
            .map(|agent| (agent.name.clone(), agent))
            .collect();
        service.sessions = snapshot
            .sessions
            .into_iter()
            .map(|session| (session.session_id.clone(), session))
            .collect();
        service.audit_events = snapshot.audit_events;
        service.command_events = snapshot.command_events;
        service
    }

    pub fn snapshot(&self) -> MonitorSnapshot {
        MonitorSnapshot {
            config: self.config.clone(),
            agents: self.agents.values().cloned().collect(),
            sessions: self.sessions.values().cloned().collect(),
            audit_events: self.audit_events.clone(),
            command_events: self.command_events.clone(),
        }
    }

    pub fn replace_config(&mut self, config: HunterConfig) {
        self.config = config;
    }

    pub fn register_or_update_agent(&mut self, heartbeat: AgentHeartbeat) {
        let agent_name = heartbeat.agent_name;
        let entry = self.agents.entry(agent_name.clone()).or_insert_with(|| {
            HunterClone::new(
                agent_name.clone(),
                heartbeat.kind.clone(),
                heartbeat.capabilities.clone(),
            )
        });
        entry.recover(
            heartbeat.kind,
            heartbeat.capabilities,
            heartbeat.assigned_session_id,
        );
    }

    pub fn mark_agent_failure(&mut self, agent_name: &str) -> bool {
        if let Some(agent) = self.agents.get_mut(agent_name) {
            agent.mark_failure();
            self.audit_events.push(AuditEvent {
                event_id: format!("agent-failure-{agent_name}-{}", self.audit_events.len() + 1),
                session_id: None,
                agent_name: Some(agent_name.to_owned()),
                kind: AuditEventKind::AgentFailure,
                detail: format!("Hunter clone {agent_name} reported repeated failure."),
                recorded_at_epoch_ms: now_epoch_ms(),
            });
            return true;
        }

        false
    }

    pub fn create_session(
        &mut self,
        request: SessionCreateRequest,
    ) -> Result<HunterSession, Vec<crate::Vulnerability>> {
        let findings = self
            .inspector
            .inspect_context(&request.discord_context, &self.config);
        if !findings.is_empty() {
            return Err(findings);
        }

        let now = now_epoch_ms();
        let session_id = request.session_id.unwrap_or_else(|| {
            format!(
                "hunter-{}-{}",
                request
                    .discord_context
                    .thread_id
                    .clone()
                    .unwrap_or_else(|| request.discord_context.channel_id.clone()),
                self.sessions.len() + 1
            )
        });

        let session = HunterSession {
            session_id: session_id.clone(),
            clone_name: request.clone_name,
            requested_agent: request.requested_agent,
            assigned_agent: None,
            state: SessionState::WaitingForAgent,
            discord_context: request.discord_context,
            transcript: vec![TranscriptEntry {
                author: session_author(&session_id),
                author_kind: MessageAuthorKind::System,
                body: request.initial_prompt,
                recorded_at_epoch_ms: now,
            }],
            findings: Vec::new(),
            created_at_epoch_ms: now,
            last_activity_epoch_ms: now,
            closed_by: None,
            close_reason: None,
        };

        self.sessions.insert(session_id.clone(), session.clone());
        self.audit_events.push(AuditEvent {
            event_id: format!("session-created-{session_id}"),
            session_id: Some(session_id.clone()),
            agent_name: None,
            kind: AuditEventKind::SessionCreated,
            detail: format!(
                "Hunter clone {} opened session for Discord user {}.",
                session.clone_name, session.discord_context.user_id
            ),
            recorded_at_epoch_ms: now,
        });
        self.record_command(
            Some(session_id.clone()),
            "spawn_clone",
            "accepted",
            "Created Hunter clone session from Discord intake.",
        );

        Ok(session)
    }

    pub fn assign_agent(
        &mut self,
        session_id: &str,
        request: AssignAgentRequest,
    ) -> Option<HunterSession> {
        let agent_name = request.agent_name.clone();
        {
            let agent = self.agents.get_mut(&agent_name)?;
            agent.assign(session_id.to_owned());
        }
        let session = self.sessions.get_mut(session_id)?;
        session.assigned_agent = Some(agent_name.clone());
        session.state = SessionState::Active;
        session.last_activity_epoch_ms = now_epoch_ms();
        let session_clone = session.clone();
        self.audit_events.push(AuditEvent {
            event_id: format!("session-assigned-{session_id}"),
            session_id: Some(session_id.to_owned()),
            agent_name: Some(agent_name.clone()),
            kind: AuditEventKind::SessionAssigned,
            detail: format!(
                "Operator {} assigned {} to session {}.",
                request.operator_id, agent_name, session_id
            ),
            recorded_at_epoch_ms: now_epoch_ms(),
        });
        self.record_command(
            Some(session_id.to_owned()),
            "assign_agent",
            "accepted",
            &format!("Assigned {} to session {}.", request.agent_name, session_id),
        );
        Some(session_clone)
    }

    pub fn relay_message(
        &mut self,
        session_id: &str,
        request: MessageRelayRequest,
    ) -> Result<HunterSession, Vec<crate::Vulnerability>> {
        let findings = self.inspector.inspect_message(&request.body);
        if !findings.is_empty() {
            return Err(findings);
        }
        let session = self.sessions.get_mut(session_id).ok_or_else(Vec::new)?;
        let now = now_epoch_ms();
        session.transcript.push(TranscriptEntry {
            author: request.author,
            author_kind: request.author_kind,
            body: request.body,
            recorded_at_epoch_ms: now,
        });
        if session.state != SessionState::Closed {
            session.state = if session.assigned_agent.is_some() {
                SessionState::Active
            } else {
                SessionState::WaitingForAgent
            };
        }
        session.last_activity_epoch_ms = now;
        let assigned_agent = session.assigned_agent.clone();
        let transcript_len = session.transcript.len();
        let session_clone = session.clone();
        self.audit_events.push(AuditEvent {
            event_id: format!("message-relayed-{session_id}-{transcript_len}"),
            session_id: Some(session_id.to_owned()),
            agent_name: assigned_agent,
            kind: AuditEventKind::MessageRelayed,
            detail: "Relayed message into Hunter session transcript.".into(),
            recorded_at_epoch_ms: now,
        });
        self.record_command(
            Some(session_id.to_owned()),
            "relay_message",
            "accepted",
            "Relayed message to Hunter session transcript.",
        );
        Ok(session_clone)
    }

    pub fn close_session(
        &mut self,
        session_id: &str,
        request: CloseSessionRequest,
    ) -> Option<HunterSession> {
        let session = self.sessions.get_mut(session_id)?;
        let assigned_agent = session.assigned_agent.clone();
        let operator_id = request.operator_id.clone();
        session.state = SessionState::Closed;
        session.closed_by = Some(operator_id.clone());
        session.close_reason = request.reason.clone();
        session.last_activity_epoch_ms = now_epoch_ms();
        let session_clone = session.clone();
        if let Some(agent_name) = &assigned_agent {
            if let Some(agent) = self.agents.get_mut(agent_name) {
                agent.release();
            }
        }
        self.audit_events.push(AuditEvent {
            event_id: format!("session-closed-{session_id}"),
            session_id: Some(session_id.to_owned()),
            agent_name: assigned_agent,
            kind: AuditEventKind::SessionClosed,
            detail: format!("Operator {} closed session {}.", operator_id, session_id),
            recorded_at_epoch_ms: now_epoch_ms(),
        });
        self.record_command(
            Some(session_id.to_owned()),
            "close_session",
            "accepted",
            &format!("Closed Hunter session {}.", session_id),
        );
        Some(session_clone)
    }

    pub fn handle_discord_command(
        &mut self,
        request: DiscordCommandRequest,
    ) -> Result<DiscordDispatch, Vec<crate::Vulnerability>> {
        match request.command {
            DiscordCommandKind::SpawnClone => {
                let session = self.create_session(SessionCreateRequest {
                    session_id: request.session_id,
                    clone_name: request
                        .clone_name
                        .unwrap_or_else(|| self.config.hunter_identity.clone()),
                    requested_agent: request.requested_agent,
                    discord_context: request.context,
                    initial_prompt: request
                        .content
                        .unwrap_or_else(|| "Hunter clone session started.".into()),
                })?;
                Ok(DiscordDispatch {
                    acknowledged: true,
                    session_id: Some(session.session_id.clone()),
                    response: format!(
                        "Hunter clone {} spawned session {}.",
                        session.clone_name, session.session_id
                    ),
                })
            }
            DiscordCommandKind::AssignAgent => {
                let session_id = request.session_id.unwrap_or_default();
                let session = self
                    .assign_agent(
                        &session_id,
                        AssignAgentRequest {
                            agent_name: request.agent_name.unwrap_or_default(),
                            operator_id: request
                                .operator_id
                                .unwrap_or_else(|| request.context.user_id.clone()),
                        },
                    )
                    .ok_or_else(Vec::new)?;
                Ok(DiscordDispatch {
                    acknowledged: true,
                    session_id: Some(session.session_id.clone()),
                    response: format!(
                        "Assigned {:?} to session {}.",
                        session.assigned_agent, session.session_id
                    ),
                })
            }
            DiscordCommandKind::RelayMessage => {
                let session_id = request.session_id.unwrap_or_default();
                let session = self.relay_message(
                    &session_id,
                    MessageRelayRequest {
                        author: request.context.user_id,
                        author_kind: MessageAuthorKind::DiscordUser,
                        body: request.content.unwrap_or_default(),
                    },
                )?;
                Ok(DiscordDispatch {
                    acknowledged: true,
                    session_id: Some(session.session_id.clone()),
                    response: format!("Relayed message into session {}.", session.session_id),
                })
            }
            DiscordCommandKind::CloseSession => {
                let session_id = request.session_id.unwrap_or_default();
                let session = self
                    .close_session(
                        &session_id,
                        CloseSessionRequest {
                            operator_id: request
                                .operator_id
                                .unwrap_or_else(|| request.context.user_id.clone()),
                            reason: request.content,
                        },
                    )
                    .ok_or_else(Vec::new)?;
                Ok(DiscordDispatch {
                    acknowledged: true,
                    session_id: Some(session.session_id.clone()),
                    response: format!("Closed session {}.", session.session_id),
                })
            }
            DiscordCommandKind::SessionStatus => {
                let session_id = request.session_id.unwrap_or_default();
                let session = self
                    .sessions
                    .get(&session_id)
                    .cloned()
                    .ok_or_else(Vec::new)?;
                Ok(DiscordDispatch {
                    acknowledged: true,
                    session_id: Some(session.session_id.clone()),
                    response: format!(
                        "Session {} is {:?} with assigned agent {:?}.",
                        session.session_id, session.state, session.assigned_agent
                    ),
                })
            }
        }
    }

    pub fn sessions(&self) -> Vec<HunterSession> {
        self.sessions.values().cloned().collect()
    }

    pub fn audit_history(&self) -> &[AuditEvent] {
        &self.audit_events
    }

    pub fn command_events(&self) -> &[CommandEvent] {
        &self.command_events
    }

    pub fn status(&self, discord_bot_configured: bool) -> MonitorStatus {
        let tracked_sessions = self.sessions.len();
        let active_sessions = self
            .sessions
            .values()
            .filter(|session| {
                matches!(
                    session.state,
                    SessionState::Active | SessionState::WaitingForAgent
                )
            })
            .count();
        let closed_sessions = self
            .sessions
            .values()
            .filter(|session| session.state == SessionState::Closed)
            .count();

        MonitorStatus {
            hunter_identity: self.config.hunter_identity.clone(),
            active_agents: self.active_agents(),
            available_agents: self.available_agents(),
            minimum_active_agents: self.config.minimum_active_agents,
            ready: self.is_ready(discord_bot_configured),
            discord_bot_configured,
            tracked_sessions,
            active_sessions,
            closed_sessions,
            recorded_commands: self.command_events.len(),
        }
    }

    pub fn is_ready(&self, discord_bot_configured: bool) -> bool {
        discord_bot_configured && self.active_agents() >= self.config.minimum_active_agents
    }

    pub fn active_agents(&self) -> usize {
        self.agents
            .values()
            .filter(|agent| agent.is_available())
            .count()
    }

    pub fn available_agents(&self) -> usize {
        self.agents
            .values()
            .filter(|agent| agent.status == AgentStatus::Idle)
            .count()
    }

    pub fn discord_context_allowed(&self, context: &DiscordContext) -> bool {
        self.inspector
            .inspect_context(context, &self.config)
            .is_empty()
    }

    fn record_command(
        &mut self,
        session_id: Option<String>,
        command_name: &str,
        outcome: &str,
        detail: &str,
    ) {
        self.command_events.push(CommandEvent {
            event_id: format!("command-{command_name}-{}", self.command_events.len() + 1),
            session_id,
            command_name: command_name.to_owned(),
            outcome: outcome.to_owned(),
            detail: detail.to_owned(),
            recorded_at_epoch_ms: now_epoch_ms(),
        });
    }
}

fn now_epoch_ms() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis()
}

fn session_author(session_id: &str) -> String {
    format!("session-bootstrap:{session_id}")
}

#[cfg(test)]
mod tests {
    use crate::{
        AgentCapability, AgentHeartbeat, AgentKind, AssignAgentRequest, CloseSessionRequest,
        DiscordCommandKind, DiscordCommandRequest, DiscordContext, HunterConfig, MessageAuthorKind,
        MessageRelayRequest, MonitorService, SessionCreateRequest, SessionState,
    };

    fn config() -> HunterConfig {
        HunterConfig {
            hunter_identity: "hunter-prime".into(),
            allowed_guilds: vec!["guild-1".into()],
            allowed_channels: vec!["channel-1".into()],
            minimum_active_agents: 1,
        }
    }

    fn context() -> DiscordContext {
        DiscordContext {
            guild_id: "guild-1".into(),
            channel_id: "channel-1".into(),
            thread_id: Some("thread-9".into()),
            user_id: "discord-user".into(),
            message_id: Some("message-1".into()),
        }
    }

    #[test]
    fn command_routing_spawns_and_relays() {
        let mut service = MonitorService::new(config());
        let dispatch = service
            .handle_discord_command(DiscordCommandRequest {
                command: DiscordCommandKind::SpawnClone,
                context: context(),
                session_id: Some("session-1".into()),
                clone_name: Some("hunter-copy".into()),
                requested_agent: Some("builder-1".into()),
                agent_name: None,
                content: Some("Need build help".into()),
                operator_id: None,
            })
            .expect("spawn should succeed");
        assert_eq!(dispatch.session_id.as_deref(), Some("session-1"));

        let relay = service
            .handle_discord_command(DiscordCommandRequest {
                command: DiscordCommandKind::RelayMessage,
                context: context(),
                session_id: Some("session-1".into()),
                clone_name: None,
                requested_agent: None,
                agent_name: None,
                content: Some("Run the titan task".into()),
                operator_id: None,
            })
            .expect("relay should succeed");
        assert!(relay.response.contains("session-1"));
        assert_eq!(service.sessions()[0].transcript.len(), 2);
    }

    #[test]
    fn clone_lifecycle_tracks_assignment_and_close() {
        let mut service = MonitorService::new(config());
        service.register_or_update_agent(AgentHeartbeat {
            agent_name: "builder-1".into(),
            kind: AgentKind::Builder,
            capabilities: vec![AgentCapability::TaskExecution],
            assigned_session_id: None,
        });
        service
            .create_session(SessionCreateRequest {
                session_id: Some("session-2".into()),
                clone_name: "hunter-copy".into(),
                requested_agent: Some("builder-1".into()),
                discord_context: context(),
                initial_prompt: "Handle task".into(),
            })
            .expect("session should be created");

        let session = service
            .assign_agent(
                "session-2",
                AssignAgentRequest {
                    agent_name: "builder-1".into(),
                    operator_id: "operator-1".into(),
                },
            )
            .expect("assignment should succeed");
        assert_eq!(session.state, SessionState::Active);

        let closed = service
            .close_session(
                "session-2",
                CloseSessionRequest {
                    operator_id: "operator-1".into(),
                    reason: Some("done".into()),
                },
            )
            .expect("close should succeed");
        assert_eq!(closed.state, SessionState::Closed);
        assert_eq!(service.available_agents(), 1);
    }

    #[test]
    fn heartbeat_failover_marks_agent_offline_after_repeated_failure() {
        let mut service = MonitorService::new(config());
        service.register_or_update_agent(AgentHeartbeat {
            agent_name: "sentinel-1".into(),
            kind: AgentKind::Sentinel,
            capabilities: vec![AgentCapability::Moderation],
            assigned_session_id: None,
        });
        assert_eq!(service.active_agents(), 1);
        assert!(service.mark_agent_failure("sentinel-1"));
        assert!(service.mark_agent_failure("sentinel-1"));
        assert!(service.mark_agent_failure("sentinel-1"));
        assert_eq!(service.active_agents(), 0);
    }

    #[test]
    fn allowlists_protect_session_intake() {
        let mut service = MonitorService::new(config());
        let mut blocked_context = context();
        blocked_context.channel_id = "blocked-channel".into();

        let error = service
            .create_session(SessionCreateRequest {
                session_id: Some("session-3".into()),
                clone_name: "hunter-copy".into(),
                requested_agent: None,
                discord_context: blocked_context,
                initial_prompt: "hello".into(),
            })
            .expect_err("blocked channel should fail");
        assert!(error
            .iter()
            .any(|finding| finding.code == "channel_not_allowlisted"));
    }

    #[test]
    fn snapshot_round_trip_preserves_sessions() {
        let mut service = MonitorService::new(config());
        service.register_or_update_agent(AgentHeartbeat {
            agent_name: "researcher-1".into(),
            kind: AgentKind::Researcher,
            capabilities: vec![AgentCapability::KnowledgeRetrieval],
            assigned_session_id: None,
        });
        service
            .create_session(SessionCreateRequest {
                session_id: Some("session-4".into()),
                clone_name: "hunter-copy".into(),
                requested_agent: Some("researcher-1".into()),
                discord_context: context(),
                initial_prompt: "research this".into(),
            })
            .expect("session should be created");
        service
            .relay_message(
                "session-4",
                MessageRelayRequest {
                    author: "discord-user".into(),
                    author_kind: MessageAuthorKind::DiscordUser,
                    body: "Follow up".into(),
                },
            )
            .expect("relay should work");

        let restored = MonitorService::from_snapshot(service.snapshot());

        assert_eq!(restored.sessions().len(), 1);
        assert_eq!(restored.command_events().len(), 2);
    }
}
