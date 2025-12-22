//! Kimi ACP agent implementation

crate::define_standard_acp_agent!(
    KimiAgent,
    "kimi",
    "Kimi",
    "assets/icons/kimi.svg",
    "kimi",
    ["--experimental-acp"]
);
