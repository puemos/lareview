//! Prompt management system for LaReview
//! Handles template-based prompt generation using Handlebars templates

use handlebars::Handlebars;
use once_cell::sync::Lazy;
use serde_json::Value;
use std::collections::HashMap;

/// Static registry of available prompts
static PROMPT_REGISTRY: Lazy<HashMap<&'static str, &'static str>> = Lazy::new(|| {
    let mut m = HashMap::new();
    m.insert("generate_tasks", include_str!("generate_tasks.hbs"));
    // m.insert("other_prompt", include_str!("other_prompt.hbs"));
    m
});

/// Render a prompt template by name with the provided context
/// This function uses Handlebars templating to substitute variables in the prompt
pub fn render(name: &str, ctx: &Value) -> anyhow::Result<String> {
    let template = PROMPT_REGISTRY
        .get(name)
        .ok_or_else(|| anyhow::anyhow!("unknown prompt '{name}'"))?;

    let mut hb = Handlebars::new();
    hb.set_strict_mode(true); // fail if a variable is missing

    hb.render_template(template, ctx)
        .map_err(|e| anyhow::anyhow!("rendering prompt '{name}' failed: {e}"))
}
