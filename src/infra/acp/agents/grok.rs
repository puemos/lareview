//! Grok Build ACP agent implementation.

crate::define_standard_acp_agent!(
    GrokAgent,
    "grok",
    "Grok",
    "assets/icons/grok.svg",
    "grok",
    ["--no-auto-update", "agent", "stdio"]
);

#[cfg(test)]
mod tests {
    use super::*;
    use crate::infra::acp::agent_trait::AcpAgent;

    #[test]
    fn grok_uses_the_installed_native_acp_harness() {
        let agent = GrokAgent;
        let candidate = agent.candidate();

        assert_eq!(agent.id(), "grok");
        assert_eq!(agent.display_name(), "Grok");
        assert_eq!(candidate.id, "grok");
        assert_eq!(candidate.logo.as_deref(), Some("assets/icons/grok.svg"));
        assert_eq!(candidate.args, vec!["--no-auto-update", "agent", "stdio"]);
        assert_eq!(candidate.available, candidate.command.is_some());
    }
}
