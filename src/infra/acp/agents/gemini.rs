//! Gemini ACP agent implementation

crate::define_standard_acp_agent!(
    GeminiAgent,
    "gemini",
    "Gemini",
    "assets/icons/gemini.svg",
    "gemini",
    ["--experimental-acp"]
);
