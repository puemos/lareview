//! Diagram modeling and rendering utilities.
//!
//! This module defines a neutral diagram model (flow, sequence, state, entity),
//! rendering targets (D2 + Mermaid), validation, and JSON parsing for agents.

use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::fmt::{self, Write};

// =========================================================================
// Error Handling
// =========================================================================

/// Library error types.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Error {
    /// Edge references non-existent node.
    InvalidEdge { from: String, to: String },
    /// Group contains non-existent node.
    InvalidGroupMember { group: String, member: String },
    /// Duplicate node ID.
    DuplicateNode(String),
    /// Empty diagram.
    EmptyDiagram,
    /// Invalid ID format.
    InvalidId(String),
    /// Message references non-existent actor.
    InvalidMessage { from: String, to: String },
    /// Rendering error.
    RenderError(String),
    /// Unsupported node/actor kind
    UnsupportedKind(String),
    /// Diagram parsing failure
    ParseError(String),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::InvalidEdge { from, to } => {
                write!(f, "Edge references invalid nodes: {from} -> {to}")
            }
            Error::InvalidGroupMember { group, member } => {
                write!(f, "Group '{group}' contains invalid member '{member}'")
            }
            Error::DuplicateNode(id) => write!(f, "Duplicate node ID: {id}"),
            Error::EmptyDiagram => write!(f, "Diagram contains no elements"),
            Error::InvalidId(id) => write!(f, "Invalid ID format: {id}"),
            Error::InvalidMessage { from, to } => {
                write!(f, "Message references invalid actors: {from} -> {to}")
            }
            Error::RenderError(msg) => write!(f, "Rendering error: {msg}"),
            Error::UnsupportedKind(k) => write!(f, "Unsupported kind: {k}"),
            Error::ParseError(msg) => write!(f, "Diagram parse error: {msg}"),
        }
    }
}

impl std::error::Error for Error {}

/// Convenience result alias.
pub type Result<T> = std::result::Result<T, Error>;

// =========================================================================
// Core Types with Styling
// =========================================================================

/// Diagram variants.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", content = "data", rename_all = "snake_case")]
pub enum Diagram {
    Flow(FlowDiagram),
    Sequence(SequenceDiagram),
    State(StateDiagram),
    Entity(EntityDiagram),
}

/// Layout direction.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Direction {
    #[default]
    LeftToRight,
    TopToBottom,
    RightToLeft,
    BottomToTop,
}

impl Serialize for Direction {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let value = match self {
            Direction::LeftToRight => "LR",
            Direction::TopToBottom => "TB",
            Direction::RightToLeft => "RL",
            Direction::BottomToTop => "BT",
        };
        serializer.serialize_str(value)
    }
}

impl<'de> Deserialize<'de> for Direction {
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let raw = String::deserialize(deserializer)?;
        let normalized = raw.trim().to_ascii_lowercase().replace('-', "_");
        match normalized.as_str() {
            "lr" | "left_to_right" => Ok(Direction::LeftToRight),
            "tb" | "top_to_bottom" => Ok(Direction::TopToBottom),
            "rl" | "right_to_left" => Ok(Direction::RightToLeft),
            "bt" | "bottom_to_top" => Ok(Direction::BottomToTop),
            _ => Err(serde::de::Error::custom(format!(
                "invalid direction '{raw}'"
            ))),
        }
    }
}

/// Color specification.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Color {
    pub hex: String,
}

impl Color {
    pub fn new(hex: &str) -> Self {
        Self {
            hex: hex.to_string(),
        }
    }

    pub fn blue() -> Self {
        Self::new("#4A90E2")
    }
    pub fn green() -> Self {
        Self::new("#7ED321")
    }
    pub fn red() -> Self {
        Self::new("#D0021B")
    }
    pub fn orange() -> Self {
        Self::new("#F5A623")
    }
    pub fn purple() -> Self {
        Self::new("#9013FE")
    }
    pub fn gray() -> Self {
        Self::new("#9B9B9B")
    }
}

impl Serialize for Color {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&self.hex)
    }
}

impl<'de> Deserialize<'de> for Color {
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let raw = String::deserialize(deserializer)?;
        let trimmed = raw.trim();
        let lower = trimmed.to_ascii_lowercase();
        let mapped = match lower.as_str() {
            "blue" => Color::blue(),
            "green" => Color::green(),
            "red" => Color::red(),
            "orange" => Color::orange(),
            "purple" => Color::purple(),
            "gray" => Color::gray(),
            _ => Color::new(trimmed),
        };
        Ok(mapped)
    }
}

/// Visual styling for nodes.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct NodeStyle {
    pub color: Option<Color>,
    pub border_color: Option<Color>,
    pub border_width: Option<u8>,
    pub font_size: Option<u8>,
    pub bold: bool,
    pub italic: bool,
}

/// Edge styling.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct EdgeStyle {
    pub color: Option<Color>,
    pub thickness: Option<u8>,
    pub animated: bool,
}

// =========================================================================
// Flow Diagram
// =========================================================================

/// Flow diagram with nodes and edges.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FlowDiagram {
    #[serde(default)]
    pub direction: Direction,
    pub nodes: Vec<Node>,
    #[serde(default)]
    pub edges: Vec<Edge>,
    #[serde(default)]
    pub groups: Vec<Group>,
    #[serde(default)]
    pub metadata: Metadata,
}

/// Additional information for diagrams.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Metadata {
    pub title: Option<String>,
    pub description: Option<String>,
    pub author: Option<String>,
}

/// A node in a flow diagram.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Node {
    pub id: String,
    pub label: String,
    pub kind: NodeKind,
    #[serde(default)]
    pub style: NodeStyle,
    pub tooltip: Option<String>,
}

/// Node kinds used to drive shape mapping.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum NodeKind {
    System,
    Service,
    Database,
    Queue,
    User,
    File,
    Lambda,
    Cache,
    LoadBalancer,
    Generic,
}

impl NodeKind {
    fn from_str(raw: &str) -> Self {
        let normalized = raw.trim().to_ascii_lowercase().replace('-', "_");
        match normalized.as_str() {
            "system" => Self::System,
            "service" => Self::Service,
            "database" | "db" => Self::Database,
            "queue" => Self::Queue,
            "user" => Self::User,
            "file" => Self::File,
            "lambda" => Self::Lambda,
            "cache" => Self::Cache,
            "loadbalancer" | "load_balancer" | "lb" => Self::LoadBalancer,
            "generic" => Self::Generic,
            _ => Self::Generic,
        }
    }
}

impl<'de> Deserialize<'de> for NodeKind {
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let raw = String::deserialize(deserializer)?;
        Ok(NodeKind::from_str(&raw))
    }
}

/// A connection between nodes.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Edge {
    pub from: String,
    pub to: String,
    #[serde(default)]
    pub label: Option<String>,
    #[serde(default)]
    pub kind: EdgeKind,
    #[serde(default)]
    pub dashed: bool,
    #[serde(default)]
    pub style: EdgeStyle,
}

/// Edge kind for semantic tagging.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[derive(Default)]
pub enum EdgeKind {
    #[default]
    Call,
    Event,
    Data,
    Dependency,
}

/// Grouping of nodes.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Group {
    pub id: String,
    pub label: Option<String>,
    pub members: Vec<String>,
    #[serde(default)]
    pub style: NodeStyle,
}

// =========================================================================
// Sequence Diagram
// =========================================================================

/// Sequence diagram definition.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SequenceDiagram {
    pub actors: Vec<Actor>,
    #[serde(default)]
    pub messages: Vec<Message>,
    #[serde(default)]
    pub metadata: Metadata,
}

/// Actor definition for sequence diagrams.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Actor {
    pub id: String,
    pub label: String,
    pub kind: ActorKind,
    #[serde(default)]
    pub style: NodeStyle,
}

/// Actor kind used to style participants.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ActorKind {
    User,
    Service,
    Database,
    Queue,
    External,
    File,
    Generic,
}

impl ActorKind {
    fn from_str(raw: &str) -> Self {
        let normalized = raw.trim().to_ascii_lowercase().replace('-', "_");
        match normalized.as_str() {
            "user" => Self::User,
            "service" => Self::Service,
            "database" | "db" => Self::Database,
            "queue" => Self::Queue,
            "external" => Self::External,
            "file" => Self::File,
            "generic" => Self::Generic,
            _ => Self::Generic,
        }
    }
}

impl<'de> Deserialize<'de> for ActorKind {
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let raw = String::deserialize(deserializer)?;
        Ok(ActorKind::from_str(&raw))
    }
}

/// Sequence messages.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", content = "data", rename_all = "snake_case")]
pub enum Message {
    Call {
        from: String,
        to: String,
        label: String,
        #[serde(default)]
        is_async: bool,
        #[serde(default)]
        style: EdgeStyle,
    },
    Return {
        from: String,
        to: String,
        #[serde(default)]
        label: Option<String>,
    },
    Note {
        target: NoteTarget,
        text: String,
    },
    Fragment {
        fragment: Fragment,
    },
    Activate {
        actor: String,
    },
    Deactivate {
        actor: String,
    },
}

/// Note placement.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum NoteTarget {
    Single(String),
    Over(Vec<String>),
}

/// Fragment grouping (alt/opt/loop/par/etc.).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Fragment {
    pub kind: FragmentKind,
    #[serde(default)]
    pub label: Option<String>,
    pub branches: Vec<FragmentBranch>,
}

/// Branch inside a fragment.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FragmentBranch {
    #[serde(default)]
    pub label: Option<String>,
    #[serde(default)]
    pub messages: Vec<Message>,
}

/// Fragment kind.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FragmentKind {
    Alt,
    Opt,
    Loop,
    Par,
    Break,
    Critical,
}

// =========================================================================
// State Diagram
// =========================================================================

/// Simple state diagram.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct StateDiagram {
    pub states: Vec<State>,
    #[serde(default)]
    pub transitions: Vec<Transition>,
    pub initial: String,
    #[serde(default)]
    pub finals: Vec<String>,
}

/// A state in a state diagram.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct State {
    pub id: String,
    pub label: String,
    #[serde(default)]
    pub substates: Vec<State>,
}

/// Transition between states.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Transition {
    pub from: String,
    pub to: String,
    pub trigger: String,
    #[serde(default)]
    pub guard: Option<String>,
    #[serde(default)]
    pub action: Option<String>,
}

// =========================================================================
// Entity Diagram
// =========================================================================

/// Entity-relationship diagram.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EntityDiagram {
    pub entities: Vec<Entity>,
    #[serde(default)]
    pub relationships: Vec<Relationship>,
}

/// Table/entity.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Entity {
    pub id: String,
    pub label: String,
    pub attributes: Vec<Attribute>,
}

/// Attribute/column.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Attribute {
    pub name: String,
    pub type_name: String,
    pub is_key: bool,
    pub nullable: bool,
}

/// Relationship between entities.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Relationship {
    pub from: String,
    pub to: String,
    pub label: String,
    pub cardinality: Cardinality,
}

/// Relationship cardinality.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Cardinality {
    OneToOne,
    OneToMany,
    ManyToOne,
    ManyToMany,
}

// =========================================================================
// Builder Pattern (Flow only for now)
// =========================================================================

/// Builder for flow diagrams.
pub struct FlowDiagramBuilder {
    direction: Direction,
    nodes: Vec<Node>,
    edges: Vec<Edge>,
    groups: Vec<Group>,
    metadata: Metadata,
}

impl FlowDiagram {
    /// Create a builder for a flow diagram.
    pub fn builder() -> FlowDiagramBuilder {
        FlowDiagramBuilder {
            direction: Direction::TopToBottom,
            nodes: Vec::new(),
            edges: Vec::new(),
            groups: Vec::new(),
            metadata: Metadata::default(),
        }
    }
}

impl FlowDiagramBuilder {
    pub fn direction(mut self, direction: Direction) -> Self {
        self.direction = direction;
        self
    }

    pub fn title(mut self, title: impl Into<String>) -> Self {
        self.metadata.title = Some(title.into());
        self
    }

    pub fn node(mut self, id: impl Into<String>, label: impl Into<String>, kind: NodeKind) -> Self {
        self.nodes.push(Node {
            id: id.into(),
            label: label.into(),
            kind,
            style: NodeStyle::default(),
            tooltip: None,
        });
        self
    }

    pub fn node_styled(
        mut self,
        id: impl Into<String>,
        label: impl Into<String>,
        kind: NodeKind,
        style: NodeStyle,
        tooltip: Option<String>,
    ) -> Self {
        self.nodes.push(Node {
            id: id.into(),
            label: label.into(),
            kind,
            style,
            tooltip,
        });
        self
    }

    pub fn edge(
        mut self,
        from: impl Into<String>,
        to: impl Into<String>,
        label: impl Into<String>,
    ) -> Self {
        self.edges.push(Edge {
            from: from.into(),
            to: to.into(),
            label: Some(label.into()),
            kind: EdgeKind::Call,
            dashed: false,
            style: EdgeStyle::default(),
        });
        self
    }

    pub fn edge_styled(
        mut self,
        from: impl Into<String>,
        to: impl Into<String>,
        label: Option<String>,
        kind: EdgeKind,
        dashed: bool,
        style: EdgeStyle,
    ) -> Self {
        self.edges.push(Edge {
            from: from.into(),
            to: to.into(),
            label,
            kind,
            dashed,
            style,
        });
        self
    }

    pub fn group(
        mut self,
        id: impl Into<String>,
        label: impl Into<String>,
        members: Vec<String>,
    ) -> Self {
        self.groups.push(Group {
            id: id.into(),
            label: Some(label.into()),
            members,
            style: NodeStyle::default(),
        });
        self
    }

    pub fn build(self) -> Result<FlowDiagram> {
        let diagram = FlowDiagram {
            direction: self.direction,
            nodes: self.nodes,
            edges: self.edges,
            groups: self.groups,
            metadata: self.metadata,
        };
        diagram.validate()?;
        Ok(diagram)
    }
}

// =========================================================================
// Validation
// =========================================================================

impl FlowDiagram {
    /// Validate nodes, edges, and groups.
    pub fn validate(&self) -> Result<()> {
        if self.nodes.is_empty() {
            return Err(Error::EmptyDiagram);
        }

        let node_ids: HashSet<_> = self.nodes.iter().map(|n| &n.id).collect();

        // Check for duplicate IDs
        if node_ids.len() != self.nodes.len() {
            let mut seen = HashSet::new();
            for node in &self.nodes {
                if !seen.insert(&node.id) {
                    return Err(Error::DuplicateNode(node.id.clone()));
                }
            }
        }

        // Validate edges
        for edge in &self.edges {
            if !node_ids.contains(&edge.from) || !node_ids.contains(&edge.to) {
                return Err(Error::InvalidEdge {
                    from: edge.from.clone(),
                    to: edge.to.clone(),
                });
            }
        }

        // Validate groups
        for group in &self.groups {
            for member in &group.members {
                if !node_ids.contains(member) {
                    return Err(Error::InvalidGroupMember {
                        group: group.id.clone(),
                        member: member.clone(),
                    });
                }
            }
        }

        Ok(())
    }
}

impl SequenceDiagram {
    /// Validate actors and messages.
    pub fn validate(&self) -> Result<()> {
        if self.actors.is_empty() {
            return Err(Error::EmptyDiagram);
        }

        let actor_ids: HashSet<_> = self.actors.iter().map(|a| &a.id).collect();

        fn validate_message(msg: &Message, actor_ids: &HashSet<&String>) -> Result<()> {
            match msg {
                Message::Call { from, to, .. } | Message::Return { from, to, .. } => {
                    if !actor_ids.contains(from) || !actor_ids.contains(to) {
                        return Err(Error::InvalidMessage {
                            from: from.clone(),
                            to: to.clone(),
                        });
                    }
                }
                Message::Note { target, .. } => match target {
                    NoteTarget::Single(actor) => {
                        if !actor_ids.contains(actor) {
                            return Err(Error::InvalidMessage {
                                from: actor.clone(),
                                to: actor.clone(),
                            });
                        }
                    }
                    NoteTarget::Over(actors) => {
                        for actor in actors {
                            if !actor_ids.contains(actor) {
                                return Err(Error::InvalidMessage {
                                    from: actor.clone(),
                                    to: actor.clone(),
                                });
                            }
                        }
                    }
                },
                Message::Fragment { fragment } => {
                    for branch in &fragment.branches {
                        for m in &branch.messages {
                            validate_message(m, actor_ids)?;
                        }
                    }
                }
                Message::Activate { actor } | Message::Deactivate { actor } => {
                    if !actor_ids.contains(actor) {
                        return Err(Error::InvalidMessage {
                            from: actor.clone(),
                            to: actor.clone(),
                        });
                    }
                }
            }
            Ok(())
        }

        for msg in &self.messages {
            validate_message(msg, &actor_ids)?;
        }

        Ok(())
    }
}

// =========================================================================
// Rendering
// =========================================================================

/// Renderer trait.
pub trait Renderer {
    fn render(&self, diagram: &Diagram) -> Result<String>;
}

/// Render diagrams to D2.
pub struct D2Renderer;
/// Render diagrams to Mermaid.
pub struct MermaidRenderer;

impl Renderer for D2Renderer {
    fn render(&self, diagram: &Diagram) -> Result<String> {
        match diagram {
            Diagram::Flow(f) => render_flow_d2(f),
            Diagram::Sequence(s) => render_sequence_d2(s),
            Diagram::State(s) => render_state_d2(s),
            Diagram::Entity(e) => render_entity_d2(e),
        }
    }
}

impl Renderer for MermaidRenderer {
    fn render(&self, diagram: &Diagram) -> Result<String> {
        match diagram {
            Diagram::Flow(f) => render_flow_mermaid(f),
            Diagram::Sequence(s) => render_sequence_mermaid(s),
            Diagram::State(s) => render_state_mermaid(s),
            Diagram::Entity(e) => render_entity_mermaid(e),
        }
    }
}

// --- Flow -> D2 ---

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

// --- Flow -> Mermaid ---

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

// --- Sequence -> D2 ---

fn render_sequence_d2(seq: &SequenceDiagram) -> Result<String> {
    let mut out = String::new();
    writeln!(&mut out, "Flow: {{").map_err(render_err)?;
    writeln!(&mut out, "  shape: sequence_diagram").map_err(render_err)?;

    for actor in &seq.actors {
        writeln!(
            &mut out,
            "  {}: {{ label: \"{}\" }}",
            actor.id,
            escape_d2(&actor.label)
        )
        .map_err(render_err)?;
    }

    for msg in &seq.messages {
        render_sequence_msg_d2(&mut out, msg, 2)?;
    }

    writeln!(&mut out, "}}").map_err(render_err)?;
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
                r#"{pad}{from} -> {to}: \"{label}\""#,
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
                r#"{pad}{from} <- {to}: \"{text}\""#,
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
                    r#"{pad}{actor}: {{ note: \"{text}\" }}"#,
                    pad = pad,
                    actor = actor,
                    text = escape_d2(text)
                )
                .map_err(render_err)?;
            }
            NoteTarget::Over(actors) => {
                // D2 doesn't support spanning notes in sequence diagrams natively yet.
                // We attach the note to the first actor as a fallback to avoid creating
                // a phantom participant named "note over ...".
                if let Some(first) = actors.first() {
                    writeln!(
                        out,
                        r#"{pad}{actor}: {{ note: \"{text}\" }}"#,
                        pad = pad,
                        actor = first,
                        text = escape_d2(text)
                    )
                    .map_err(render_err)?;
                }
            }
        },
        Message::Fragment { fragment } => {
            let title = fragment
                .branches
                .first()
                .and_then(|branch| branch.label.as_deref())
                .or(fragment.label.as_deref())
                .unwrap_or(fragment_title(fragment.kind));
            writeln!(
                out,
                r#"{pad}{kw} \"{title}\""#,
                pad = pad,
                kw = fragment_keyword(fragment.kind),
                title = escape_d2(title)
            )
            .map_err(render_err)?;
            for (i, branch) in fragment.branches.iter().enumerate() {
                if i > 0 {
                    let branch_kw = fragment_branch_keyword(fragment.kind);
                    if let Some(label) = branch.label.as_deref() {
                        writeln!(
                            out,
                            r#"{pad}{branch_kw} \"{label}\""#,
                            pad = pad,
                            branch_kw = branch_kw,
                            label = escape_d2(label)
                        )
                        .map_err(render_err)?;
                    } else {
                        writeln!(out, "{pad}{branch_kw}", pad = pad, branch_kw = branch_kw)
                            .map_err(render_err)?;
                    }
                }
                for m in &branch.messages {
                    render_sequence_msg_d2(out, m, indent + 2)?;
                }
            }
            writeln!(out, "{pad}end", pad = pad).map_err(render_err)?;
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

// --- Sequence -> Mermaid ---

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

// --- State -> Mermaid ---

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

fn render_state_d2(state: &StateDiagram) -> Result<String> {
    let mut out = String::new();
    writeln!(&mut out, "direction: right").map_err(render_err)?;
    for s in &state.states {
        writeln!(
            &mut out,
            "{}: {{ label: \"{}\" }}",
            s.id,
            escape_d2(&s.label)
        )
        .map_err(render_err)?;
    }
    for t in &state.transitions {
        writeln!(
            &mut out,
            "{} -> {}: \"{}\"",
            t.from,
            t.to,
            escape_d2(&t.trigger)
        )
        .map_err(render_err)?;
    }
    Ok(out)
}

// --- Entity -> Mermaid ---

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

// =========================================================================
// JSON Parsing helpers
// =========================================================================

/// Parse JSON text into a Diagram.
pub fn parse_json(input: &str) -> Result<Diagram> {
    let trimmed = input.trim();
    if trimmed.is_empty() {
        return Err(Error::EmptyDiagram);
    }

    let diagram: Diagram = serde_json::from_str(trimmed)
        .map_err(|e| Error::ParseError(format!("JSON parse error: {e}")))?;

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

// =========================================================================
// Helpers
// =========================================================================

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

fn escape_d2(s: &str) -> String {
    s.replace('\\', "\\\\").replace('"', "\\\"")
}

fn escape_mermaid(s: &str) -> String {
    s.replace('"', "&quot;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('#', "&num;")
}

fn safe_id(id: &str) -> String {
    let mut out = String::with_capacity(id.len());
    let mut started = false;

    for c in id.chars() {
        if c.is_ascii_alphanumeric() || c == '_' {
            out.push(c);
            started = true;
        } else if started {
            out.push('_');
        }
    }

    if out.is_empty() || out.chars().next().unwrap().is_ascii_digit() {
        out.insert(0, '_');
    }

    out
}

fn render_err(e: impl std::fmt::Display) -> Error {
    Error::RenderError(e.to_string())
}

// =========================================================================
// Tests
// =========================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn parse_sequence_json(value: serde_json::Value) -> SequenceDiagram {
        match parse_json(&value.to_string()).expect("JSON produces diagram") {
            Diagram::Sequence(seq) => seq,
            other => panic!("Expected Sequence diagram, got {:?}", other),
        }
    }

    fn parse_flow_json(value: serde_json::Value) -> FlowDiagram {
        match parse_json(&value.to_string()).expect("JSON produces diagram") {
            Diagram::Flow(flow) => flow,
            other => panic!("Expected Flow diagram, got {:?}", other),
        }
    }

    fn render_sequence_outputs(seq: &SequenceDiagram) -> (String, String) {
        let d2 = D2Renderer
            .render(&Diagram::Sequence(seq.clone()))
            .expect("D2 render succeeds");
        let mermaid = MermaidRenderer
            .render(&Diagram::Sequence(seq.clone()))
            .expect("Mermaid render succeeds");
        (d2, mermaid)
    }

    fn count_lines_starting_with(text: &str, prefix: &str) -> usize {
        text.lines()
            .filter(|line| line.trim_start().starts_with(prefix))
            .count()
    }

    fn assert_contains_all(text: &str, parts: &[&str]) {
        for part in parts {
            assert!(
                text.contains(part),
                "Expected output to contain '{part}', got:\n{text}"
            );
        }
    }

    #[test]
    fn flow_builder_happy_path() {
        let flow = FlowDiagram::builder()
            .title("E-Commerce Architecture")
            .direction(Direction::LeftToRight)
            .node("client", "Web Client", NodeKind::User)
            .node("api", "API", NodeKind::Service)
            .node("db", "DB", NodeKind::Database)
            .edge("client", "api", "HTTPS")
            .edge("api", "db", "SQL")
            .build()
            .expect("valid diagram");

        let d2 = D2Renderer.render(&Diagram::Flow(flow.clone())).unwrap();
        let mermaid = MermaidRenderer.render(&Diagram::Flow(flow)).unwrap();

        assert!(d2.contains("client -> api"));
        assert!(mermaid.contains("flowchart LR"));
    }

    #[test]
    fn sequence_rendering() {
        let seq = SequenceDiagram {
            actors: vec![
                Actor {
                    id: "User".into(),
                    label: "User".into(),
                    kind: ActorKind::User,
                    style: NodeStyle::default(),
                },
                Actor {
                    id: "API".into(),
                    label: "API".into(),
                    kind: ActorKind::Service,
                    style: NodeStyle::default(),
                },
            ],
            messages: vec![
                Message::Call {
                    from: "User".into(),
                    to: "API".into(),
                    label: "POST /login".into(),
                    is_async: true,
                    style: EdgeStyle::default(),
                },
                Message::Return {
                    from: "API".into(),
                    to: "User".into(),
                    label: Some("200".into()),
                },
            ],
            metadata: Metadata::default(),
        };

        let d2 = D2Renderer.render(&Diagram::Sequence(seq.clone())).unwrap();
        let mermaid = MermaidRenderer.render(&Diagram::Sequence(seq)).unwrap();
        assert!(d2.contains("POST /login"));
        assert!(mermaid.contains("sequenceDiagram"));
    }

    #[test]
    fn entity_rendering() {
        let entity = EntityDiagram {
            entities: vec![Entity {
                id: "users".into(),
                label: "Users".into(),
                attributes: vec![
                    Attribute {
                        name: "id".into(),
                        type_name: "uuid".into(),
                        is_key: true,
                        nullable: false,
                    },
                    Attribute {
                        name: "email".into(),
                        type_name: "text".into(),
                        is_key: false,
                        nullable: false,
                    },
                ],
            }],
            relationships: vec![],
        };

        let d2 = D2Renderer.render(&Diagram::Entity(entity.clone())).unwrap();
        let mermaid = MermaidRenderer.render(&Diagram::Entity(entity)).unwrap();
        assert!(d2.contains("sql_table"));
        assert!(mermaid.contains("erDiagram"));
    }

    #[test]
    fn flow_json_complex_example() {
        let flow = parse_flow_json(json!({
            "type": "flow",
            "data": {
                "direction": "LR",
                "nodes": [
                    { "id": "bucket", "label": "IndexedDBBucket", "kind": "service" },
                    { "id": "meta", "label": "BucketMeta", "kind": "database" },
                    { "id": "measure", "label": "measureBucketUsage", "kind": "service" },
                    { "id": "estimate", "label": "getStorageEstimate", "kind": "service" },
                    { "id": "browser", "label": "Browser API", "kind": "generic" }
                ],
                "edges": [
                    { "from": "bucket", "to": "meta", "label": "trackUsage()", "style": { "color": "blue" } },
                    { "from": "bucket", "to": "measure", "label": "fallback measurement", "style": { "color": "orange" }, "dashed": true },
                    { "from": "measure", "to": "meta", "label": "update metadata" },
                    { "from": "estimate", "to": "browser", "label": "navigator.storage.estimate" },
                    { "from": "browser", "to": "estimate", "label": "quota/usage data" }
                ],
                "groups": [
                    { "id": "tracking", "label": "Storage Tracking", "members": ["bucket", "meta", "measure"] },
                    { "id": "estimation", "label": "Quota Estimation", "members": ["estimate", "browser"] }
                ]
            }
        }));
        let mermaid = MermaidRenderer.render(&Diagram::Flow(flow)).unwrap();
        assert_contains_all(
            &mermaid,
            &["flowchart LR", "IndexedDBBucket", "Storage Tracking"],
        );
        assert!(mermaid.contains("subgraph tracking[\"Storage Tracking\"]"));
    }

    #[test]
    fn flow_json_unknown_kind_falls_back() {
        let flow = parse_flow_json(json!({
            "type": "flow",
            "data": {
                "direction": "LR",
                "nodes": [
                    { "id": "mystery", "label": "Mystery", "kind": "widget" }
                ]
            }
        }));
        assert_eq!(flow.nodes[0].kind, NodeKind::Generic);
    }

    #[test]
    fn sequence_json_basic() {
        let seq = parse_sequence_json(json!({
            "type": "sequence",
            "data": {
                "actors": [
                    { "id": "user", "label": "User", "kind": "user" },
                    { "id": "api", "label": "API", "kind": "service" },
                    { "id": "db", "label": "DB", "kind": "database" }
                ],
                "messages": [
                    { "type": "call", "data": { "from": "user", "to": "api", "label": "POST /login" } },
                    { "type": "call", "data": { "from": "api", "to": "db", "label": "SELECT user" } },
                    { "type": "return", "data": { "from": "db", "to": "api", "label": "user record" } },
                    { "type": "call", "data": { "from": "api", "to": "user", "label": "200 OK" } }
                ]
            }
        }));
        assert_eq!(seq.actors.len(), 3);
        assert_eq!(seq.messages.len(), 4);
    }

    #[test]
    fn sequence_json_file_and_unknown_actor_kinds() {
        let seq = parse_sequence_json(json!({
            "type": "sequence",
            "data": {
                "actors": [
                    { "id": "file", "label": "File", "kind": "file" },
                    { "id": "svc", "label": "Service", "kind": "service" },
                    { "id": "mystery", "label": "Mystery", "kind": "widget" }
                ],
                "messages": []
            }
        }));
        assert_eq!(seq.actors[0].kind, ActorKind::File);
        assert_eq!(seq.actors[2].kind, ActorKind::Generic);
    }

    #[test]
    fn sequence_json_with_notes() {
        let seq = parse_sequence_json(json!({
            "type": "sequence",
            "data": {
                "actors": [
                    { "id": "user", "label": "User", "kind": "user" },
                    { "id": "api", "label": "API", "kind": "service" }
                ],
                "messages": [
                    { "type": "call", "data": { "from": "user", "to": "api", "label": "Request" } },
                    { "type": "note", "data": { "target": ["user", "api"], "text": "This is a note" } },
                    { "type": "call", "data": { "from": "api", "to": "user", "label": "Response" } }
                ]
            }
        }));
        assert_eq!(seq.messages.len(), 3);
    }

    #[test]
    fn sequence_json_alt_else_labels_render() {
        let seq = parse_sequence_json(json!({
            "type": "sequence",
            "data": {
                "actors": [
                    { "id": "user", "label": "User", "kind": "user" },
                    { "id": "api", "label": "API", "kind": "service" }
                ],
                "messages": [
                    {
                        "type": "fragment",
                        "data": {
                            "fragment": {
                                "kind": "alt",
                                "branches": [
                                    { "label": "success", "messages": [
                                        { "type": "call", "data": { "from": "user", "to": "api", "label": "Ping" } }
                                    ]},
                                    { "label": "failure", "messages": [
                                        { "type": "call", "data": { "from": "user", "to": "api", "label": "Retry" } }
                                    ]}
                                ]
                            }
                        }
                    }
                ]
            }
        }));
        let fragment = match &seq.messages[0] {
            Message::Fragment { fragment } => fragment,
            other => panic!("Expected Fragment message, got {:?}", other),
        };
        assert_eq!(fragment.kind, FragmentKind::Alt);
        assert_eq!(fragment.branches.len(), 2);
        assert_eq!(fragment.branches[0].label.as_deref(), Some("success"));
        assert_eq!(fragment.branches[1].label.as_deref(), Some("failure"));

        let (d2, mermaid) = render_sequence_outputs(&seq);
        assert_contains_all(&d2, &["alt \\\"success\\\"", "else \\\"failure\\\"", "end"]);
        assert_contains_all(
            &mermaid,
            &["sequenceDiagram", "alt success", "else failure", "end"],
        );
    }

    #[test]
    fn sequence_json_alt_multiple_else_branches() {
        let seq = parse_sequence_json(json!({
            "type": "sequence",
            "data": {
                "actors": [
                    { "id": "user", "label": "User", "kind": "user" },
                    { "id": "api", "label": "API", "kind": "service" }
                ],
                "messages": [
                    {
                        "type": "fragment",
                        "data": {
                            "fragment": {
                                "kind": "alt",
                                "branches": [
                                    { "label": "primary", "messages": [
                                        { "type": "call", "data": { "from": "user", "to": "api", "label": "A" } }
                                    ]},
                                    { "label": "secondary", "messages": [
                                        { "type": "call", "data": { "from": "user", "to": "api", "label": "B" } }
                                    ]},
                                    { "messages": [
                                        { "type": "call", "data": { "from": "user", "to": "api", "label": "C" } }
                                    ]}
                                ]
                            }
                        }
                    }
                ]
            }
        }));
        let (d2, mermaid) = render_sequence_outputs(&seq);

        assert_eq!(count_lines_starting_with(&d2, "else"), 2);
        assert_eq!(count_lines_starting_with(&mermaid, "else"), 2);
        assert!(!mermaid.contains("else Alternative"));
    }

    #[test]
    fn sequence_json_par_branches_render_and() {
        let seq = parse_sequence_json(json!({
            "type": "sequence",
            "data": {
                "actors": [
                    { "id": "user", "label": "User", "kind": "user" },
                    { "id": "api", "label": "API", "kind": "service" }
                ],
                "messages": [
                    {
                        "type": "fragment",
                        "data": {
                            "fragment": {
                                "kind": "par",
                                "branches": [
                                    { "label": "fast", "messages": [
                                        { "type": "call", "data": { "from": "user", "to": "api", "label": "A" } }
                                    ]},
                                    { "label": "slow", "messages": [
                                        { "type": "call", "data": { "from": "user", "to": "api", "label": "B" } }
                                    ]},
                                    { "messages": [
                                        { "type": "call", "data": { "from": "user", "to": "api", "label": "C" } }
                                    ]}
                                ]
                            }
                        }
                    }
                ]
            }
        }));
        let (d2, mermaid) = render_sequence_outputs(&seq);

        assert_contains_all(&d2, &["par \\\"fast\\\"", "and \\\"slow\\\"", "end"]);
        assert_contains_all(&mermaid, &["par fast", "and slow", "end"]);
        assert_eq!(count_lines_starting_with(&d2, "and"), 2);
        assert_eq!(count_lines_starting_with(&mermaid, "and"), 2);
    }

    #[test]
    fn sequence_json_critical_option_branches() {
        let seq = parse_sequence_json(json!({
            "type": "sequence",
            "data": {
                "actors": [
                    { "id": "user", "label": "User", "kind": "user" },
                    { "id": "api", "label": "API", "kind": "service" }
                ],
                "messages": [
                    {
                        "type": "fragment",
                        "data": {
                            "fragment": {
                                "kind": "critical",
                                "branches": [
                                    { "label": "transaction", "messages": [
                                        { "type": "call", "data": { "from": "user", "to": "api", "label": "begin" } }
                                    ]},
                                    { "label": "rollback", "messages": [
                                        { "type": "call", "data": { "from": "user", "to": "api", "label": "abort" } }
                                    ]}
                                ]
                            }
                        }
                    }
                ]
            }
        }));
        let (d2, mermaid) = render_sequence_outputs(&seq);

        assert_contains_all(
            &d2,
            &[
                "critical \\\"transaction\\\"",
                "option \\\"rollback\\\"",
                "end",
            ],
        );
        assert_contains_all(
            &mermaid,
            &["critical transaction", "option rollback", "end"],
        );
        assert_eq!(count_lines_starting_with(&d2, "option"), 1);
        assert_eq!(count_lines_starting_with(&mermaid, "option"), 1);
    }

    #[test]
    fn sequence_json_opt_loop_break_render() {
        let seq = parse_sequence_json(json!({
            "type": "sequence",
            "data": {
                "actors": [
                    { "id": "user", "label": "User", "kind": "user" },
                    { "id": "api", "label": "API", "kind": "service" }
                ],
                "messages": [
                    {
                        "type": "fragment",
                        "data": {
                            "fragment": {
                                "kind": "opt",
                                "branches": [
                                    { "label": "cached", "messages": [
                                        { "type": "call", "data": { "from": "user", "to": "api", "label": "hit" } }
                                    ]}
                                ]
                            }
                        }
                    },
                    {
                        "type": "fragment",
                        "data": {
                            "fragment": {
                                "kind": "loop",
                                "branches": [
                                    { "label": "retry", "messages": [
                                        { "type": "call", "data": { "from": "user", "to": "api", "label": "retry" } }
                                    ]}
                                ]
                            }
                        }
                    },
                    {
                        "type": "fragment",
                        "data": {
                            "fragment": {
                                "kind": "break",
                                "branches": [
                                    { "label": "fatal", "messages": [
                                        { "type": "call", "data": { "from": "user", "to": "api", "label": "abort" } }
                                    ]}
                                ]
                            }
                        }
                    }
                ]
            }
        }));
        let (d2, mermaid) = render_sequence_outputs(&seq);

        assert_contains_all(
            &d2,
            &[
                "opt \\\"cached\\\"",
                "loop \\\"retry\\\"",
                "break \\\"fatal\\\"",
            ],
        );
        assert_contains_all(&mermaid, &["opt cached", "loop retry", "break fatal"]);
        assert_eq!(count_lines_starting_with(&d2, "end"), 3);
        assert_eq!(count_lines_starting_with(&mermaid, "end"), 3);
    }

    #[test]
    fn sequence_json_complex() {
        let seq = parse_sequence_json(json!({
            "type": "sequence",
            "data": {
                "actors": [
                    { "id": "user", "label": "User", "kind": "user" },
                    { "id": "download", "label": "Download Epic", "kind": "service" },
                    { "id": "bucket", "label": "IndexedDBBucket", "kind": "service" },
                    { "id": "tracker", "label": "trackBucketUsage", "kind": "service" },
                    { "id": "meta", "label": "Bucket Metadata", "kind": "database" },
                    { "id": "storage", "label": "Storage Stats", "kind": "service" }
                ],
                "messages": [
                    { "type": "call", "data": { "from": "user", "to": "download", "label": "Start download" } },
                    { "type": "call", "data": { "from": "download", "to": "bucket", "label": "write(chunk)" } },
                    { "type": "call", "data": { "from": "bucket", "to": "tracker", "label": "trackBucketUsage(id, bytes)" } },
                    { "type": "call", "data": { "from": "tracker", "to": "meta", "label": "update metadata" } },
                    { "type": "note", "data": { "target": ["bucket", "meta"], "text": "Atomic update required" } },
                    { "type": "call", "data": { "from": "storage", "to": "meta", "label": "query usage stats" } },
                    { "type": "call", "data": { "from": "meta", "to": "storage", "label": "return stats" } }
                ]
            }
        }));
        assert_eq!(seq.actors.len(), 6);
        assert_eq!(seq.messages.len(), 7);
    }
}
