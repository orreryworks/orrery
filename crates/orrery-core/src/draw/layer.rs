//! Layer-based rendering system for SVG output.
//!
//! This module provides a type-safe layer system that allows drawable components
//! to specify which z-order layer their SVG elements should be rendered to.
//!
//! # Overview
//!
//! The layer system consists of:
//! - [`RenderLayer`]: An enum defining available rendering layers in order
//! - [`LayeredOutput`]: A structure for collecting SVG nodes by layer
//!
//! # Example
//!
//! ```
//! # use orrery_core::draw::{RenderLayer, LayeredOutput};
//! # use svg::node::element::Rectangle;
//!
//! let mut output = LayeredOutput::new();
//!
//! // Add background element
//! let bg = Rectangle::new().set("fill", "white");
//! output.add_to_layer(RenderLayer::Background, Box::new(bg));
//!
//! // Add text element
//! let text = svg::node::element::Text::new("Hello");
//! output.add_to_layer(RenderLayer::Text, Box::new(text));
//!
//! // Render all layers in order
//! let svg_nodes = output.render();
//! ```

use svg::node::element as svg_element;

/// Type alias for boxed SVG nodes.
pub type SvgNode = Box<dyn svg::Node>;

/// Defines the rendering layers for SVG output.
///
/// Layers are rendered from bottom to top in the order defined by variant declaration.
/// The `Ord` derive uses declaration order, so the first variant renders first (bottom),
/// and the last variant renders last (top).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum RenderLayer {
    /// Background elements (fills, background shapes) - renders first
    Background,
    /// Vertical lifelines in sequence diagrams
    Lifeline,
    /// Main content shapes and participants - default layer
    Content,
    /// Activation boxes (sorted by nesting within this layer)
    Activation,
    /// Fragment blocks (backgrounds, borders, separators, pentagon tabs)
    Fragment,
    /// Notes and annotations (backgrounds, fills, lines, strokes, corners)
    Note,
    /// Arrows, relations, and messages between elements
    Arrow,
    /// Text labels and annotations
    Text,
}

impl RenderLayer {
    /// Returns a human-readable name for this layer.
    pub fn name(&self) -> &'static str {
        match self {
            Self::Background => "background",
            Self::Lifeline => "lifeline",
            Self::Content => "content",
            Self::Activation => "activation",
            Self::Fragment => "fragment",
            Self::Note => "note",
            Self::Arrow => "arrow",
            Self::Text => "text",
        }
    }
}

/// Represents SVG nodes grouped by rendering layer.
///
/// This struct collects SVG nodes and organizes them by layer. When rendered,
/// nodes are emitted in layer order (bottom to top), ensuring correct z-ordering.
///
/// # Example
///
/// ```
/// # use orrery_core::draw::{RenderLayer, LayeredOutput};
/// # use svg::node::element::{Rectangle, Text as SvgText};
///
/// let mut output = LayeredOutput::new();
///
/// // Add nodes to different layers
/// let bg = Rectangle::new().set("fill", "white");
/// output.add_to_layer(RenderLayer::Background, Box::new(bg));
///
/// let text = SvgText::new("Label");
/// output.add_to_layer(RenderLayer::Text, Box::new(text));
///
/// let border = Rectangle::new().set("stroke", "black");
/// output.add_to_layer(RenderLayer::Content, Box::new(border));
///
/// // Render all layers in order
/// let svg_nodes = output.render();
/// assert_eq!(svg_nodes.len(), 3); // Three layer groups
/// ```
#[derive(Debug, Default)]
pub struct LayeredOutput {
    items: Vec<(RenderLayer, SvgNode)>,
}

impl LayeredOutput {
    /// Creates a new empty `LayeredOutput`.
    pub fn new() -> Self {
        Self::default()
    }

    /// Adds a single node to the specified layer.
    ///
    /// If the layer doesn't exist yet, it will be created. Nodes are appended
    /// to the layer in the order they are added.
    ///
    /// # Example
    ///
    /// ```
    /// # use orrery_core::draw::{RenderLayer, LayeredOutput};
    /// # use svg::node::element::Rectangle;
    ///
    /// let mut output = LayeredOutput::new();
    /// let rect = Rectangle::new();
    /// output.add_to_layer(RenderLayer::Content, Box::new(rect));
    /// ```
    pub fn add_to_layer(&mut self, layer: RenderLayer, node: SvgNode) {
        self.items.push((layer, node));
    }

    /// Merges all layers from another `LayeredOutput` into this one.
    ///
    /// Nodes from the other output are appended to existing layers in this output.
    /// This is useful for combining outputs from multiple drawables.
    ///
    /// # Example
    ///
    /// ```
    /// # use orrery_core::draw::{RenderLayer, LayeredOutput};
    /// # use svg::node::element::Rectangle;
    ///
    /// let mut output1 = LayeredOutput::new();
    /// output1.add_to_layer(RenderLayer::Content, Box::new(Rectangle::new()));
    ///
    /// let mut output2 = LayeredOutput::new();
    /// output2.add_to_layer(RenderLayer::Text, Box::new(Rectangle::new()));
    ///
    /// output1.merge(output2);
    /// // output1 now contains both Content and Text layers
    /// ```
    pub fn merge(&mut self, other: LayeredOutput) {
        self.items.extend(other.items);
    }

    /// Returns `true` if there are no nodes in any layer.
    ///
    /// # Example
    ///
    /// ```
    /// # use orrery_core::draw::{RenderLayer, LayeredOutput};
    /// # use svg::node::element::Rectangle;
    ///
    /// let mut output = LayeredOutput::new();
    /// assert!(output.is_empty());
    ///
    /// output.add_to_layer(RenderLayer::Content, Box::new(Rectangle::new()));
    /// assert!(!output.is_empty());
    /// ```
    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    /// Renders all layers to SVG groups, consuming the output.
    ///
    /// Each non-empty layer becomes an SVG `<g>` element with a `data-layer`
    /// attribute identifying the layer. Empty layers are skipped.
    ///
    /// Layers are rendered from bottom to top based on the `Ord` implementation
    /// of `RenderLayer` (declaration order in the enum).
    ///
    /// This method consumes the `LayeredOutput` to avoid cloning SVG nodes.
    ///
    /// # Returns
    ///
    /// A vector of SVG group nodes, one per non-empty layer, in rendering order.
    ///
    /// # Example
    ///
    /// ```
    /// use orrery_core::draw::{RenderLayer, LayeredOutput};
    /// # use svg::node::element::Rectangle;
    ///
    /// let mut output = LayeredOutput::new();
    /// output.add_to_layer(RenderLayer::Background, Box::new(Rectangle::new()));
    /// output.add_to_layer(RenderLayer::Content, Box::new(Rectangle::new()));
    ///
    /// let svg_nodes = output.render(); // Consumes output
    /// // Background layer renders first, then Content
    /// assert_eq!(svg_nodes.len(), 2);
    /// ```
    pub fn render(mut self) -> Vec<SvgNode> {
        if self.is_empty() {
            return Vec::new();
        }

        // Sort all items by layer - Stable sorting
        self.items.sort_by_key(|(layer, _)| *layer);

        let mut result = Vec::new();
        let mut current_layer = self.items[0].0;
        let mut current_group = svg_element::Group::new().set("data-layer", current_layer.name());

        for (layer, node) in self.items {
            if layer != current_layer {
                // Finish previous layer group
                result.push(Box::new(current_group) as SvgNode);

                // Start new layer group
                current_layer = layer;
                current_group = svg_element::Group::new().set("data-layer", layer.name());
            }

            current_group = current_group.add(node);
        }

        // Add final group
        result.push(Box::new(current_group) as SvgNode);

        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use svg::node::element::Rectangle;

    #[test]
    fn test_layered_output_new() {
        let output = LayeredOutput::new();
        assert!(output.is_empty());
    }

    #[test]
    fn test_layered_output_add_to_layer() {
        let mut output = LayeredOutput::new();
        assert!(output.is_empty());

        let rect = Rectangle::new();
        output.add_to_layer(RenderLayer::Content, Box::new(rect));
        assert!(!output.is_empty());
    }

    #[test]
    fn test_layered_output_merge() {
        let mut output1 = LayeredOutput::new();
        output1.add_to_layer(RenderLayer::Content, Box::new(Rectangle::new()));

        let mut output2 = LayeredOutput::new();
        output2.add_to_layer(RenderLayer::Note, Box::new(Rectangle::new()));

        output1.merge(output2);
        assert!(!output1.is_empty());

        let nodes = output1.render();
        assert_eq!(nodes.len(), 2);
    }

    #[test]
    fn test_layered_output_is_empty() {
        let mut output = LayeredOutput::new();
        assert!(output.is_empty());

        output.add_to_layer(RenderLayer::Content, Box::new(Rectangle::new()));
        assert!(!output.is_empty());
    }

    #[test]
    fn test_layered_output_render() {
        let mut output = LayeredOutput::new();

        output.add_to_layer(RenderLayer::Content, Box::new(Rectangle::new()));
        output.add_to_layer(RenderLayer::Note, Box::new(Rectangle::new()));
        output.add_to_layer(RenderLayer::Text, Box::new(Rectangle::new()));

        let svg_nodes = output.render();

        assert_eq!(svg_nodes.len(), 3);
    }

    #[test]
    fn test_layered_output_merge_same_layer() {
        let mut output1 = LayeredOutput::new();
        output1.add_to_layer(RenderLayer::Content, Box::new(Rectangle::new()));

        let mut output2 = LayeredOutput::new();
        output2.add_to_layer(RenderLayer::Content, Box::new(Rectangle::new()));

        output1.merge(output2);

        let nodes = output1.render();
        // Should have 1 group with both Content layer
        assert_eq!(nodes.len(), 1);
    }
}
