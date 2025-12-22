//! Mistral ACP agent implementation

crate::define_standard_acp_agent!(
    MistralAgent,
    "mistral",
    "Mistral",
    "assets/icons/mistral.svg",
    "mistral",
    ["--experimental-acp"]
);
