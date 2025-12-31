use crate::infra::diagram::models::*;
use crate::infra::diagram::renderers::DiagramRenderer;
use std::fmt::Write;

pub struct MermaidRenderer;

impl DiagramRenderer for MermaidRenderer {
    fn render(&self, diagram: &Diagram) -> Result<String> {
        match diagram {
            Diagram::Flow(f) => render_flow_mermaid(f),
            Diagram::Sequence(s) => render_sequence_mermaid(s),
            Diagram::State(s) => render_state_mermaid(s),
            Diagram::Entity(e) => render_entity_mermaid(e),
        }
    }
}

fn render_flow_mermaid(flow: &FlowDiagram) -> Result<String> {
    let mut out = String::new();
    let direction = match flow.direction {
        Direction::LeftToRight => "LR",
        Direction::TopToBottom => "TB",
        Direction::RightToLeft => "RL",
        Direction::BottomToTop => "BT",
    };
    writeln!(&mut out, "flowchart {direction}").map_err(render_err)?;

    for node in &flow.nodes {
        let (open, close) = mermaid_shape_for_node(node.kind);
        writeln!(
            &mut out,
            "    {}{}\"{}\"{}",
            safe_id(&node.id),
            open,
            escape_mermaid(&node.label),
            close
        )
        .map_err(render_err)?;

        if let Some(color) = &node.style.color {
            writeln!(
                &mut out,
                "    style {} fill:{}",
                safe_id(&node.id),
                color.hex
            )
            .map_err(render_err)?;
        }
    }

    for group in &flow.groups {
        writeln!(
            &mut out,
            "    subgraph {}[\"{}\"]",
            safe_id(&group.id),
            escape_mermaid(group.label.as_deref().unwrap_or(&group.id))
        )
        .map_err(render_err)?;
        for member in &group.members {
            writeln!(&mut out, "        {}", safe_id(member)).map_err(render_err)?;
        }
        writeln!(&mut out, "    end").map_err(render_err)?;
    }

    for edge in &flow.edges {
        let arrow = if edge.dashed { "-.->" } else { "-->" };
        let label = edge.label.as_deref().unwrap_or("");
        if label.is_empty() {
            writeln!(
                &mut out,
                "    {} {} {}",
                safe_id(&edge.from),
                arrow,
                safe_id(&edge.to)
            )
            .map_err(render_err)?;
        } else {
            writeln!(
                &mut out,
                "    {} {}|\"{}\"|{}",
                safe_id(&edge.from),
                arrow,
                escape_mermaid(label),
                safe_id(&edge.to)
            )
            .map_err(render_err)?;
        }
    }

    Ok(out)
}

fn render_sequence_mermaid(seq: &SequenceDiagram) -> Result<String> {
    let mut out = String::new();
    writeln!(&mut out, "sequenceDiagram").map_err(render_err)?;

    for actor in &seq.actors {
        writeln!(
            &mut out,
            "    participant {} as {}",
            safe_id(&actor.id),
            escape_mermaid(&actor.label)
        )
        .map_err(render_err)?;
    }

    for msg in &seq.messages {
        render_sequence_msg_mermaid(&mut out, msg, 4)?;
    }

    Ok(out)
}

fn render_sequence_msg_mermaid(out: &mut String, msg: &Message, indent: usize) -> Result<()> {
    let pad = " ".repeat(indent);
    match msg {
        Message::Call {
            from,
            to,
            label,
            is_async,
            ..
        } => {
            let arrow = if *is_async { "->>" } else { "->" };
            writeln!(
                out,
                "{pad}{from}{arrow}{to}: {label}",
                pad = pad,
                from = safe_id(from),
                arrow = arrow,
                to = safe_id(to),
                label = escape_mermaid(label)
            )
            .map_err(render_err)?;
        }
        Message::Return { from, to, label } => {
            let text = label.as_deref().unwrap_or("");
            writeln!(
                out,
                "{pad}{from}-->>{to}: {label}",
                pad = pad,
                from = safe_id(from),
                to = safe_id(to),
                label = escape_mermaid(text)
            )
            .map_err(render_err)?;
        }
        Message::Note { target, text } => match target {
            NoteTarget::Single(actor) => writeln!(
                out,
                "{pad}Note right of {actor}: {text}",
                pad = pad,
                actor = safe_id(actor),
                text = escape_mermaid(text)
            )
            .map_err(render_err)?,
            NoteTarget::Over(actors) => {
                let list = actors
                    .iter()
                    .map(|a| safe_id(a))
                    .collect::<Vec<_>>()
                    .join(",");
                writeln!(
                    out,
                    "{pad}Note over {list}: {text}",
                    pad = pad,
                    list = list,
                    text = escape_mermaid(text)
                )
                .map_err(render_err)?;
            }
        },
        Message::Fragment { fragment } => {
            let mut branches = fragment.branches.iter();
            if let Some(first) = branches.next() {
                let header = fragment_keyword(fragment.kind);
                let label = first
                    .label
                    .as_deref()
                    .or(fragment.label.as_deref())
                    .unwrap_or(fragment_title(fragment.kind));
                writeln!(
                    out,
                    "{pad}{header} {label}",
                    pad = pad,
                    label = escape_mermaid(label)
                )
                .map_err(render_err)?;
                for m in &first.messages {
                    render_sequence_msg_mermaid(out, m, indent + 4)?;
                }
                for branch in branches {
                    let else_kw = fragment_branch_keyword(fragment.kind);
                    if let Some(lbl) = branch.label.as_deref() {
                        writeln!(
                            out,
                            "{pad}{else_kw} {label}",
                            pad = pad,
                            else_kw = else_kw,
                            label = escape_mermaid(lbl)
                        )
                        .map_err(render_err)?;
                    } else {
                        writeln!(out, "{pad}{else_kw}", pad = pad, else_kw = else_kw)
                            .map_err(render_err)?;
                    }
                    for m in &branch.messages {
                        render_sequence_msg_mermaid(out, m, indent + 4)?;
                    }
                }
                writeln!(out, "{pad}end", pad = pad).map_err(render_err)?;
            }
        }
        Message::Activate { actor } => {
            writeln!(out, "{pad}activate {}", safe_id(actor)).map_err(render_err)?;
        }
        Message::Deactivate { actor } => {
            writeln!(out, "{pad}deactivate {}", safe_id(actor)).map_err(render_err)?;
        }
    }
    Ok(())
}

fn render_state_mermaid(state: &StateDiagram) -> Result<String> {
    let mut out = String::new();
    writeln!(&mut out, "stateDiagram-v2").map_err(render_err)?;
    writeln!(&mut out, "    [*] --> {}", safe_id(&state.initial)).map_err(render_err)?;
    for t in &state.transitions {
        let guard = t
            .guard
            .as_deref()
            .map(|g| format!("[{g}]"))
            .unwrap_or_default();
        let action = t
            .action
            .as_deref()
            .map(|a| format!(" / {a}"))
            .unwrap_or_default();
        writeln!(
            &mut out,
            "    {} --> {}: {}{}{}",
            safe_id(&t.from),
            safe_id(&t.to),
            escape_mermaid(&t.trigger),
            guard,
            action
        )
        .map_err(render_err)?;
    }
    for final_state in &state.finals {
        writeln!(&mut out, "    {} --> [*]", safe_id(final_state)).map_err(render_err)?;
    }
    Ok(out)
}

fn render_entity_mermaid(entity: &EntityDiagram) -> Result<String> {
    let mut out = String::new();
    writeln!(&mut out, "erDiagram").map_err(render_err)?;

    for ent in &entity.entities {
        writeln!(&mut out, "    {} {{", safe_id(&ent.id)).map_err(render_err)?;
        for attr in &ent.attributes {
            let nullable = if attr.nullable { "?" } else { "" };
            let key = if attr.is_key { "PK " } else { "" };
            writeln!(
                &mut out,
                "        {key}{name} {typ}{nullable}",
                key = key,
                name = escape_mermaid(&attr.name),
                typ = escape_mermaid(&attr.type_name),
                nullable = nullable
            )
            .map_err(render_err)?;
        }
        writeln!(&mut out, "    }}").map_err(render_err)?;
    }

    for rel in &entity.relationships {
        let card = match rel.cardinality {
            Cardinality::OneToOne => "||--||",
            Cardinality::OneToMany => "||--o{",
            Cardinality::ManyToOne => "}o--||",
            Cardinality::ManyToMany => "}o--o{",
        };
        writeln!(
            &mut out,
            "    {} {} {} : {}",
            safe_id(&rel.from),
            card,
            safe_id(&rel.to),
            escape_mermaid(&rel.label)
        )
        .map_err(render_err)?;
    }

    Ok(out)
}

fn render_err(e: std::fmt::Error) -> Error {
    Error::RenderError(e.to_string())
}

fn escape_mermaid(s: &str) -> String {
    s.replace('"', "#quot;")
}

fn safe_id(id: &str) -> String {
    id.replace(['-', '.', ' ', '/'], "_")
}

fn mermaid_shape_for_node(kind: NodeKind) -> (&'static str, &'static str) {
    match kind {
        NodeKind::System | NodeKind::Service | NodeKind::Generic => ("[", "]"),
        NodeKind::Database => ("[(", ")]"),
        NodeKind::Queue | NodeKind::User => ("([", "])"),
        NodeKind::File => ("[/", "/]"),
        NodeKind::Lambda => ("{{", "}}"),
        NodeKind::Cache => ("[[", "]]"),
        NodeKind::LoadBalancer => ("{", "}"),
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
