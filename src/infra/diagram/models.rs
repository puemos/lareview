use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::fmt;

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

impl std::str::FromStr for NodeKind {
    type Err = std::convert::Infallible;

    fn from_str(raw: &str) -> std::result::Result<Self, Self::Err> {
        let normalized = raw.trim().to_ascii_lowercase().replace('-', "_");
        Ok(match normalized.as_str() {
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
        })
    }
}

impl<'de> Deserialize<'de> for NodeKind {
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let raw = String::deserialize(deserializer)?;
        Ok(raw.parse().unwrap_or(NodeKind::Generic))
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

impl std::str::FromStr for ActorKind {
    type Err = std::convert::Infallible;

    fn from_str(raw: &str) -> std::result::Result<Self, Self::Err> {
        let normalized = raw.trim().to_ascii_lowercase().replace('-', "_");
        Ok(match normalized.as_str() {
            "user" => Self::User,
            "service" => Self::Service,
            "database" | "db" => Self::Database,
            "queue" => Self::Queue,
            "external" => Self::External,
            "file" => Self::File,
            "generic" => Self::Generic,
            _ => Self::Generic,
        })
    }
}

impl<'de> Deserialize<'de> for ActorKind {
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let raw = String::deserialize(deserializer)?;
        Ok(raw.parse().unwrap_or(ActorKind::Generic))
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
