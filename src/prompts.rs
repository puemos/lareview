use serde_json::Value;

pub fn render(name: &str, data: &Value) -> Result<String, handlebars::RenderError> {
    let template = match name {
        "generate_tasks" => include_str!("generate_tasks.hbs"),
        "compact_learnings" => include_str!("compact_learnings.hbs"),
        _ => {
            return Err(handlebars::RenderError::from(
                handlebars::RenderErrorReason::NestedError(Box::new(std::io::Error::new(
                    std::io::ErrorKind::NotFound,
                    format!("unknown template: {}", name),
                ))),
            ));
        }
    };
    let handlebars = handlebars::Handlebars::new();
    handlebars.render_template(template, data)
}
