//! GitHub Copilot CLI's native ACP server over stdio.

crate::define_standard_acp_agent!(
    CopilotAgent,
    "copilot",
    "GitHub Copilot",
    "assets/icons/copilot.svg",
    "copilot",
    ["--acp", "--stdio"]
);

#[cfg(test)]
mod tests {
    use crate::infra::acp::agent_trait::AgentRegistry;

    #[test]
    fn copilot_is_registered_with_native_stdio_transport() {
        let candidate = AgentRegistry::default()
            .get_agent_candidate("copilot")
            .expect("Copilot should be a built-in agent even when not installed");
        assert_eq!(candidate.label, "GitHub Copilot");
        assert_eq!(candidate.args, ["--acp", "--stdio"]);
    }
}
