use serde_json::Value;

pub fn render(_name: &str, data: &Value) -> Result<String, handlebars::RenderError> {
    let template = include_str!("generate_tasks.hbs");
    let handlebars = handlebars::Handlebars::new();
    handlebars.render_template(template, data)
}
