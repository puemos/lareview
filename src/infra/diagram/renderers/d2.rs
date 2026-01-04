use crate::infra::diagram::models::*;
use crate::infra::diagram::renderers::DiagramRenderer;
use std::fmt::Write;

pub struct D2Renderer;

impl DiagramRenderer for D2Renderer {
    fn render(&self, diagram: &Diagram) -> Result<String> {
        match diagram {
            Diagram::Flow(f) => render_flow_d2(f),
            Diagram::Sequence(s) => render_sequence_d2(s),
            Diagram::State(s) => render_state_d2(s),
            Diagram::Entity(e) => render_entity_d2(e),
        }
    }
}

fn render_flow_d2(flow: &FlowDiagram) -> Result<String> {
    let mut out = String::new();

    if let Some(title) = &flow.metadata.title {
        writeln!(&mut out, "# {title}").map_err(render_err)?;
    }

    let direction = match flow.direction {
        Direction::LeftToRight => "right",
        Direction::TopToBottom => "down",
        Direction::RightToLeft => "left",
        Direction::BottomToTop => "up",
    };
    writeln!(&mut out, "direction: {direction}").map_err(render_err)?;

    for node in &flow.nodes {
        write!(
            &mut out,
            "{}: {{ shape: {}; label: \"{}\"",
            node.id,
            d2_shape_for_node(node.kind),
            escape_d2(&node.label)
        )
        .map_err(render_err)?;

        if let Some(color) = &node.style.color {
            write!(&mut out, "; style.fill: \"{}\"", color.hex).map_err(render_err)?;
        }
        if let Some(tooltip) = &node.tooltip {
            write!(&mut out, "; tooltip: \"{}\"", escape_d2(tooltip)).map_err(render_err)?;
        }
        writeln!(&mut out, " }}").map_err(render_err)?;
    }

    for group in &flow.groups {
        writeln!(
            &mut out,
            "{}: {{ label: \"{}\"",
            group.id,
            escape_d2(group.label.as_deref().unwrap_or(&group.id))
        )
        .map_err(render_err)?;
        for member in &group.members {
            writeln!(&mut out, "  {member}").map_err(render_err)?;
        }
        writeln!(&mut out, "}}").map_err(render_err)?;
    }

    for edge in &flow.edges {
        let arrow = if edge.dashed { "--" } else { "-" };
        write!(&mut out, "{} {}> {}", edge.from, arrow, edge.to).map_err(render_err)?;
        if let Some(label) = &edge.label {
            write!(&mut out, ": \"{}\"", escape_d2(label)).map_err(render_err)?;
        }
        writeln!(&mut out).map_err(render_err)?;
    }

    Ok(out)
}

fn render_sequence_d2(seq: &SequenceDiagram) -> Result<String> {
    let mut out = String::new();
    writeln!(&mut out, "shape: sequence_diagram").map_err(render_err)?;

    for actor in &seq.actors {
        writeln!(&mut out, "{}", actor.id).map_err(render_err)?;
    }

    for msg in &seq.messages {
        render_sequence_msg_d2(&mut out, msg, 0)?;
    }

    Ok(out)
}

fn render_sequence_msg_d2(out: &mut String, msg: &Message, indent: usize) -> Result<()> {
    let pad = " ".repeat(indent);
    match msg {
        Message::Call {
            from,
            to,
            label,
            is_async: _,
            ..
        } => {
            writeln!(
                out,
                r#"{pad}{from} -> {to}: "{label}""#,
                pad = pad,
                from = from,
                to = to,
                label = escape_d2(label)
            )
            .map_err(render_err)?;
        }
        Message::Return { from, to, label } => {
            let text = label.as_deref().unwrap_or("");
            writeln!(
                out,
                r#"{pad}{from} <- {to}: "{text}""#,
                pad = pad,
                from = from,
                to = to,
                text = escape_d2(text)
            )
            .map_err(render_err)?;
        }
        Message::Note { target, text } => match target {
            NoteTarget::Single(actor) => {
                writeln!(
                    out,
                    r#"{pad}{actor}: {{ note: "{text}" }}"#,
                    pad = pad,
                    actor = actor,
                    text = escape_d2(text)
                )
                .map_err(render_err)?;
            }
            NoteTarget::Over(actors) => {
                if let Some(first) = actors.first() {
                    writeln!(
                        out,
                        r#"{pad}{actor}: {{ note: "{text}" }}"#,
                        pad = pad,
                        actor = first,
                        text = escape_d2(text)
                    )
                    .map_err(render_err)?;
                }
            }
        },
        Message::Fragment { fragment } => {
            let kw = fragment_keyword(fragment.kind);
            writeln!(out, "{pad}{}: {{", kw).map_err(render_err)?;
            for (i, branch) in fragment.branches.iter().enumerate() {
                let branch_pad = " ".repeat(indent + 2);
                if i > 0 {
                    let branch_kw = fragment_branch_keyword(fragment.kind);
                    if let Some(label) = branch.label.as_deref() {
                        writeln!(
                            out,
                            r#"{pad}{branch_kw} "{label}": {{"#,
                            pad = pad,
                            branch_kw = branch_kw,
                            label = escape_d2(label)
                        )
                        .map_err(render_err)?;
                    } else {
                        writeln!(
                            out,
                            "{pad}{branch_kw}: {{",
                            pad = pad,
                            branch_kw = branch_kw
                        )
                        .map_err(render_err)?;
                    }
                } else if let Some(label) = branch.label.as_deref() {
                    writeln!(
                        out,
                        r#"{branch_pad}{label}: {{"#,
                        branch_pad = branch_pad,
                        label = escape_d2(label)
                    )
                    .map_err(render_err)?;
                } else {
                    writeln!(out, "{}: {{", fragment_title(fragment.kind)).map_err(render_err)?;
                }
                for m in &branch.messages {
                    render_sequence_msg_d2(out, m, indent + 4)?;
                }
                writeln!(out, "{}}}", branch_pad).map_err(render_err)?;
            }
            writeln!(out, "{}}}", pad).map_err(render_err)?;
        }
        Message::Activate { actor } => {
            writeln!(out, "{pad}activate {actor}", pad = pad, actor = actor).map_err(render_err)?;
        }
        Message::Deactivate { actor } => {
            writeln!(out, "{pad}deactivate {actor}", pad = pad, actor = actor)
                .map_err(render_err)?;
        }
    }
    Ok(())
}

fn render_state_d2(state: &StateDiagram) -> Result<String> {
    let mut out = String::new();
    writeln!(&mut out, "direction: down").map_err(render_err)?;

    for s in &state.states {
        render_state_node_d2(&mut out, s, 0)?;
    }

    writeln!(&mut out, "(*): {{ shape: circle; label: \"\" }}").map_err(render_err)?;
    writeln!(&mut out, "(*) -> {}: \"\"", state.initial).map_err(render_err)?;

    for f in &state.finals {
        writeln!(&mut out, "{}: {{ shape: double_circle; label: \"\" }}", f).map_err(render_err)?;
    }

    for t in &state.transitions {
        let label = match (&t.guard, &t.action) {
            (Some(g), Some(a)) => format!("{} [{}] / {}", t.trigger, g, a),
            (Some(g), None) => format!("{} [{}]", t.trigger, g),
            (None, Some(a)) => format!("{} / {}", t.trigger, a),
            (None, None) => t.trigger.clone(),
        };
        writeln!(
            &mut out,
            "{} -> {}: \"{}\"",
            t.from,
            t.to,
            escape_d2(&label)
        )
        .map_err(render_err)?;
    }

    Ok(out)
}

fn render_state_node_d2(out: &mut String, state: &State, indent: usize) -> Result<()> {
    let pad = " ".repeat(indent);
    if state.substates.is_empty() {
        writeln!(out, "{pad}{}: \"{}\"", state.id, escape_d2(&state.label)).map_err(render_err)?;
    } else {
        writeln!(out, "{pad}{}: {{", state.id).map_err(render_err)?;
        writeln!(out, "{pad}  label: \"{}\"", escape_d2(&state.label)).map_err(render_err)?;
        for sub in &state.substates {
            render_state_node_d2(out, sub, indent + 2)?;
        }
        writeln!(out, "{pad}}}").map_err(render_err)?;
    }
    Ok(())
}

fn render_entity_d2(entity: &EntityDiagram) -> Result<String> {
    let mut out = String::new();
    writeln!(&mut out, "direction: right").map_err(render_err)?;
    for ent in &entity.entities {
        writeln!(&mut out, "{}: {{ shape: sql_table", ent.id).map_err(render_err)?;
        for attr in &ent.attributes {
            writeln!(
                &mut out,
                "  {}: {}{}{}",
                escape_d2(&attr.name),
                if attr.is_key { "PK " } else { "" },
                escape_d2(&attr.type_name),
                if attr.nullable { "?" } else { "" }
            )
            .map_err(render_err)?;
        }
        writeln!(&mut out, "}}").map_err(render_err)?;
    }
    for rel in &entity.relationships {
        writeln!(
            &mut out,
            "{} -> {}: \"{}\"",
            rel.from,
            rel.to,
            escape_d2(&rel.label)
        )
        .map_err(render_err)?;
    }
    Ok(out)
}

fn render_err(e: std::fmt::Error) -> Error {
    Error::RenderError(e.to_string())
}

fn escape_d2(s: &str) -> String {
    s.replace('"', "\\\"")
}

fn d2_shape_for_node(kind: NodeKind) -> &'static str {
    match kind {
        NodeKind::System | NodeKind::Service | NodeKind::Generic => "rectangle",
        NodeKind::Database => "cylinder",
        NodeKind::Queue => "queue",
        NodeKind::User => "person",
        NodeKind::File => "document",
        NodeKind::Lambda => "hexagon",
        NodeKind::Cache => "rectangle",
        NodeKind::LoadBalancer => "diamond",
    }
}

fn fragment_title(kind: FragmentKind) -> &'static str {
    match kind {
        FragmentKind::Alt => "Alternative",
        FragmentKind::Opt => "Optional",
        FragmentKind::Loop => "Loop",
        FragmentKind::Par => "Parallel",
        FragmentKind::Break => "Break",
        FragmentKind::Critical => "Critical",
    }
}

fn fragment_keyword(kind: FragmentKind) -> &'static str {
    match kind {
        FragmentKind::Alt => "alt",
        FragmentKind::Opt => "opt",
        FragmentKind::Loop => "loop",
        FragmentKind::Par => "par",
        FragmentKind::Break => "break",
        FragmentKind::Critical => "critical",
    }
}

fn fragment_branch_keyword(kind: FragmentKind) -> &'static str {
    match kind {
        FragmentKind::Par => "and",
        FragmentKind::Critical => "option",
        _ => "else",
    }
}
