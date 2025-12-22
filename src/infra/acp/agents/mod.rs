//! ACP agents module - one file per agent

pub mod claude;
pub mod codex;
pub mod gemini;
pub mod kimi;
pub mod mistral;
pub mod opencode;
pub mod qwen;

pub use super::agent_trait::AgentRegistry;

#[macro_export]
macro_rules! define_standard_acp_agent {
    ($struct_name:ident, $id:expr, $label:expr, $logo:expr, $command:expr, $args:expr) => {
        pub struct $struct_name;

        impl $crate::infra::acp::agent_trait::AcpAgent for $struct_name {
            fn id(&self) -> &'static str {
                $id
            }

            fn display_name(&self) -> &'static str {
                $label
            }

            fn candidate(&self) -> $crate::infra::acp::agent_discovery::AgentCandidate {
                let command_path = $crate::infra::brew::find_bin($command)
                    .map(|path| path.to_string_lossy().to_string());
                let available = command_path.is_some();

                $crate::infra::acp::agent_discovery::AgentCandidate {
                    id: $id.to_string(),
                    label: $label.to_string(),
                    logo: Some($logo.to_string()),
                    command: command_path,
                    args: $args.iter().map(|s| s.to_string()).collect(),
                    available,
                }
            }

            fn is_available(&self) -> bool {
                $crate::infra::brew::find_bin($command).is_some()
            }
        }
    };
}
