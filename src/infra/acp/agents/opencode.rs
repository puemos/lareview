//! OpenCode ACP agent implementation

crate::define_standard_acp_agent!(
    OpenCodeAgent,
    "opencode",
    "OpenCode",
    "assets/icons/opencode.svg",
    "opencode",
    ["--experimental-acp"]
);
