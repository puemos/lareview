//! Qwen ACP agent implementation

crate::define_standard_acp_agent!(
    QwenAgent,
    "qwen",
    "Qwen Code",
    "assets/icons/qwen.svg",
    "qwen",
    ["--experimental-acp"]
);
