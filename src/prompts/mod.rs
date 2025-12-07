use handlebars::Handlebars;
use once_cell::sync::Lazy;
use serde_json::Value;
use std::collections::HashMap;

static PROMPT_REGISTRY: Lazy<HashMap<&'static str, &'static str>> = Lazy::new(|| {
    let mut m = HashMap::new();
    m.insert("generate_tasks", include_str!("generate_tasks.hbs"));
    // m.insert("other_prompt", include_str!("other_prompt.hbs"));
    m
});

/// Render a prompt by name using Handlebars.
///
/// Usage:
///     render("generate_tasks", json!({"id": "123"}))
///
pub fn render(name: &str, ctx: &Value) -> anyhow::Result<String> {
    let template = PROMPT_REGISTRY
        .get(name)
        .ok_or_else(|| anyhow::anyhow!("unknown prompt '{name}'"))?;

    let mut hb = Handlebars::new();
    hb.set_strict_mode(true); // fail if a variable is missing

    hb.render_template(template, ctx)
        .map_err(|e| anyhow::anyhow!("rendering prompt '{name}' failed: {e}"))
}
