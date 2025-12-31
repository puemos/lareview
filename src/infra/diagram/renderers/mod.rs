use crate::infra::diagram::models::*;

pub mod d2;
pub mod mermaid;

/// Renderer trait.
pub trait DiagramRenderer {
    fn render(&self, diagram: &Diagram) -> Result<String>;
}
