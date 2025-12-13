//! Agent trait definitions

use super::agent_discovery::AgentCandidate;

/// Trait that each agent should implement to provide metadata
pub trait AcpAgent {
    /// The unique ID of the agent (e.g., "codex", "qwen", "gemini")
    #[allow(dead_code)]
    fn id(&self) -> &'static str;

    /// Human-readable display name for the agent
    #[allow(dead_code)]
    fn display_name(&self) -> &'static str;

    /// Get the agent candidate
    fn candidate(&self) -> AgentCandidate;

    /// Check if the agent is available
    #[allow(dead_code)]
    fn is_available(&self) -> bool;
}

/// Agent registry that collects all available agents
pub struct AgentRegistry {
    agents: Vec<Box<dyn AcpAgent>>,
}

impl AgentRegistry {
    pub fn new() -> Self {
        Self { agents: Vec::new() }
    }

    pub fn register_agent(&mut self, agent: Box<dyn AcpAgent>) {
        self.agents.push(agent);
    }

    pub fn get_agents(&self) -> &[Box<dyn AcpAgent>] {
        &self.agents
    }

    #[allow(dead_code)]
    pub fn get_agent_by_id(&self, id: &str) -> Option<&dyn AcpAgent> {
        self.agents
            .iter()
            .find(|agent| agent.id() == id)
            .map(|agent| agent.as_ref())
    }

    #[allow(dead_code)]
    pub fn get_agent_candidate(&self, id: &str) -> Option<AgentCandidate> {
        self.get_agent_by_id(id).map(|agent| agent.candidate())
    }
}

impl Default for AgentRegistry {
    fn default() -> Self {
        let mut registry = Self::new();

        // Register all known agents
        registry.register_agent(Box::new(super::agents::claude::ClaudeAgent));
        registry.register_agent(Box::new(super::agents::codex::CodexAgent));
        registry.register_agent(Box::new(super::agents::gemini::GeminiAgent));
        registry.register_agent(Box::new(super::agents::grok::GrokAgent));
        registry.register_agent(Box::new(super::agents::mistral::MistralAgent));
        registry.register_agent(Box::new(super::agents::qwen::QwenAgent));

        registry
    }
}
