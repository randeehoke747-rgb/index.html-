use crate::{DiscordContext, HunterConfig, Vulnerability, VulnerabilityLevel};

#[derive(Clone, Debug, Default)]
pub struct CommandSafetyInspector;

impl CommandSafetyInspector {
    pub fn new() -> Self {
        Self
    }

    pub fn inspect_context(
        &self,
        context: &DiscordContext,
        config: &HunterConfig,
    ) -> Vec<Vulnerability> {
        let mut findings = Vec::new();

        if !config.allowed_guilds.is_empty() && !config.allowed_guilds.contains(&context.guild_id) {
            findings.push(Vulnerability::new(
                "guild_not_allowlisted",
                "Discord guild is not allowlisted for Hunter session intake.",
                VulnerabilityLevel::Critical,
            ));
        }

        if !config.allowed_channels.is_empty()
            && !config.allowed_channels.contains(&context.channel_id)
        {
            findings.push(Vulnerability::new(
                "channel_not_allowlisted",
                "Discord channel is not allowlisted for Hunter session intake.",
                VulnerabilityLevel::High,
            ));
        }

        findings
    }

    pub fn inspect_message(&self, body: &str) -> Vec<Vulnerability> {
        let mut findings = Vec::new();
        if body.trim().is_empty() {
            findings.push(Vulnerability::new(
                "empty_message",
                "Empty relay messages are rejected.",
                VulnerabilityLevel::Medium,
            ));
        }
        if body.len() > 4_000 {
            findings.push(Vulnerability::new(
                "message_too_large",
                "Relay messages above 4000 characters are rejected.",
                VulnerabilityLevel::High,
            ));
        }
        findings
    }
}
