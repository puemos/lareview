//! Claude ACP agent implementation

crate::define_standard_acp_agent!(
    ClaudeAgent,
    "claude",
    "Claude",
    "assets/icons/claude.svg",
    "npx",
    ["@zed-industries/claude-code-acp"]
);
