use crate::infra::diagram::models::*;
use serde::Deserialize;

/// Parse JSON text into a Diagram.
pub fn parse_json(input: &str) -> Result<Diagram> {
    let trimmed = input.trim();
    if trimmed.is_empty() {
        return Err(Error::EmptyDiagram);
    }

    let candidates = diagram_json_candidates(trimmed);
    let mut last_err: Option<serde_json::Error> = None;

    for candidate in &candidates {
        match parse_diagram_lenient(candidate) {
            Ok(parsed) => return validate_diagram(parsed),
            Err(err) => last_err = Some(err),
        }
    }

    for candidate in &candidates {
        if let Some(parsed) = repair_diagram_from_value(candidate) {
            return validate_diagram(parsed);
        }
    }

    for candidate in &candidates {
        if let Some(parsed) = coerce_diagram_from_value(candidate) {
            return validate_diagram(parsed);
        }
    }

    Err(Error::ParseError(format!(
        "JSON parse error: {}",
        last_err
            .map(|err| err.to_string())
            .unwrap_or_else(|| "unable to parse diagram".to_string())
    )))
}

fn parse_diagram_lenient(input: &str) -> std::result::Result<Diagram, serde_json::Error> {
    let mut de = serde_json::Deserializer::from_str(input);
    Diagram::deserialize(&mut de)
}

fn validate_diagram(diagram: Diagram) -> Result<Diagram> {
    match &diagram {
        Diagram::Flow(flow) => flow.validate()?,
        Diagram::Sequence(seq) => seq.validate()?,
        Diagram::State(state) => {
            if state.states.is_empty() {
                return Err(Error::EmptyDiagram);
            }
        }
        Diagram::Entity(entity) => {
            if entity.entities.is_empty() {
                return Err(Error::EmptyDiagram);
            }
        }
    }

    Ok(diagram)
}

fn diagram_json_candidates(input: &str) -> Vec<String> {
    let mut candidates = vec![input.to_string()];

    if let Some(slice) = find_first_json_slice(input) {
        let trimmed = slice.trim();
        if trimmed != input {
            candidates.push(trimmed.to_string());
        }
    }

    if let Some(repaired) = repair_json(input) {
        if !candidates.iter().any(|existing| existing == &repaired) {
            candidates.push(repaired.clone());
        }

        if let Some(slice) = find_first_json_slice(&repaired) {
            let trimmed = slice.trim();
            if !candidates.iter().any(|existing| existing == trimmed) {
                candidates.push(trimmed.to_string());
            }
        }
    }

    candidates
}

fn repair_diagram_from_value(candidate: &str) -> Option<Diagram> {
    let value: serde_json::Value = serde_json::from_str(candidate).ok()?;
    let repaired = repair_diagram_value(value)?;
    serde_json::from_value::<Diagram>(repaired).ok()
}

fn repair_diagram_value(value: serde_json::Value) -> Option<serde_json::Value> {
    let mut diagram = if value.get("type").is_some() {
        value
    } else if let Some(diagram_type) = infer_diagram_type(&value) {
        serde_json::json!({
            "type": diagram_type,
            "data": value
        })
    } else if let Some(data) = value.get("data").cloned() {
        let diagram_type = infer_diagram_type(&data)?;
        serde_json::json!({
            "type": diagram_type,
            "data": data
        })
    } else {
        return None;
    };

    if diagram.get("type").and_then(|v| v.as_str()) == Some("sequence")
        && let Some(data) = diagram.get_mut("data")
    {
        repair_sequence_messages(data);
    }

    Some(diagram)
}

fn repair_sequence_messages(data: &mut serde_json::Value) {
    let Some(messages) = data.get_mut("messages").and_then(|v| v.as_array_mut()) else {
        return;
    };

    for message in messages {
        repair_sequence_message(message);
    }
}

fn repair_sequence_message(message: &mut serde_json::Value) {
    let msg_type = message
        .get("type")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());

    if let Some(msg_type) = msg_type {
        if is_fragment_shorthand(&msg_type) {
            let data = message
                .get("data")
                .cloned()
                .unwrap_or_else(|| serde_json::Value::Object(serde_json::Map::new()));
            let fragment_value = data
                .get("fragment")
                .cloned()
                .unwrap_or_else(|| data.clone());
            let mut fragment_obj = fragment_value
                .as_object()
                .cloned()
                .unwrap_or_else(serde_json::Map::new);

            fragment_obj.insert("kind".to_string(), serde_json::Value::String(msg_type));
            fragment_obj
                .entry("branches".to_string())
                .or_insert_with(|| serde_json::Value::Array(Vec::new()));

            *message = serde_json::json!({
                "type": "fragment",
                "data": { "fragment": serde_json::Value::Object(fragment_obj) }
            });
        } else if msg_type == "fragment"
            && let Some(data) = message.get_mut("data")
            && data.get("fragment").is_none()
        {
            let fragment_value = std::mem::take(data);
            *data = serde_json::json!({ "fragment": fragment_value });
        }
    }

    let Some(branches) = message
        .pointer_mut("/data/fragment/branches")
        .and_then(|v| v.as_array_mut())
    else {
        return;
    };

    for branch in branches {
        let Some(messages) = branch.get_mut("messages").and_then(|v| v.as_array_mut()) else {
            continue;
        };
        for nested in messages {
            repair_sequence_message(nested);
        }
    }
}

fn is_fragment_shorthand(message_type: &str) -> bool {
    matches!(
        message_type,
        "alt" | "opt" | "loop" | "par" | "break" | "critical"
    )
}

fn coerce_diagram_from_value(candidate: &str) -> Option<Diagram> {
    let value: serde_json::Value = serde_json::from_str(candidate).ok()?;
    coerce_diagram_value(value)
}

fn coerce_diagram_value(value: serde_json::Value) -> Option<Diagram> {
    let object = value.as_object()?;
    if object.contains_key("type") {
        return None;
    }

    let data_value = object.get("data").cloned().unwrap_or_else(|| value.clone());
    let diagram_type = infer_diagram_type(&data_value)?;
    let wrapped = serde_json::json!({
        "type": diagram_type,
        "data": data_value
    });

    serde_json::from_value::<Diagram>(wrapped).ok()
}

fn infer_diagram_type(value: &serde_json::Value) -> Option<&'static str> {
    let object = value.as_object()?;
    if object.contains_key("actors") || object.contains_key("messages") {
        return Some("sequence");
    }
    if object.contains_key("nodes") || object.contains_key("edges") {
        return Some("flow");
    }
    if object.contains_key("states") || object.contains_key("transitions") {
        return Some("state");
    }
    if object.contains_key("entities") || object.contains_key("relationships") {
        return Some("entity");
    }
    None
}

fn find_first_json_slice(input: &str) -> Option<&str> {
    if let Some(slice) = extract_first_json_slice(input) {
        return Some(slice);
    }

    for (idx, ch) in input.char_indices() {
        if ch == '{'
            && let Some(slice) = extract_first_json_slice(&input[idx..])
        {
            return Some(slice);
        }
    }

    None
}

fn extract_first_json_slice(input: &str) -> Option<&str> {
    let mut iter = serde_json::Deserializer::from_str(input).into_iter::<serde_json::Value>();
    match iter.next()? {
        Ok(_) => Some(&input[..iter.byte_offset()]),
        Err(_) => None,
    }
}

fn repair_json(input: &str) -> Option<String> {
    let mut output = replace_smart_quotes(input);
    let mut changed = output != input;

    if !output.contains('"') && output.contains('\'') {
        output = output.replace('\'', "\"");
        changed = true;
    }

    let without_trailing = strip_trailing_commas(&output);
    if without_trailing != output {
        output = without_trailing;
        changed = true;
    }

    if changed { Some(output) } else { None }
}

fn replace_smart_quotes(input: &str) -> String {
    input
        .replace(['\u{201C}', '\u{201D}', '\u{201E}', '\u{201F}'], "\"")
        .replace(['\u{2018}', '\u{2019}'], "'")
}

fn strip_trailing_commas(input: &str) -> String {
    let chars: Vec<char> = input.chars().collect();
    let mut out = String::with_capacity(chars.len());
    let mut in_string = false;
    let mut escape = false;
    let mut idx = 0;

    while idx < chars.len() {
        let ch = chars[idx];
        if in_string {
            out.push(ch);
            if escape {
                escape = false;
            } else if ch == '\\' {
                escape = true;
            } else if ch == '"' {
                in_string = false;
            }
            idx += 1;
            continue;
        }

        if ch == '"' {
            in_string = true;
            out.push(ch);
            idx += 1;
            continue;
        }

        if ch == ',' {
            let mut lookahead = idx + 1;
            while lookahead < chars.len() && chars[lookahead].is_whitespace() {
                lookahead += 1;
            }
            if lookahead < chars.len() && (chars[lookahead] == '}' || chars[lookahead] == ']') {
                idx += 1;
                continue;
            }
        }

        out.push(ch);
        idx += 1;
    }

    out
}
