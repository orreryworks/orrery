use crate::layout::{
    common::{Bounds, Point},
    component, sequence,
};

/// Content types that can be rendered in a layer
#[derive(Debug)]
pub enum LayerContent<'a> {
    Component(component::Layout<'a>),
    Sequence(sequence::Layout<'a>),
}

/// A rendering layer containing either component or sequence diagram content
#[derive(Debug)]
pub struct Layer<'a> {
    /// Z-index determines rendering order (0 = top layer, higher = base layers)
    pub z_index: u32,
    /// Global coordinate offset for this layer
    pub offset: Point,
    /// Optional clipping boundary to keep content within parent
    pub clip_bounds: Option<Bounds>,
    /// The content of this layer
    pub content: LayerContent<'a>,
}

/// Collection of all diagram layers for rendering
#[derive(Debug)]
pub struct LayeredLayout<'a> {
    /// Ordered layers from bottom (0) to top
    pub layers: Vec<Layer<'a>>,
}

// LayerContent implementation was simplified by removing unused conversion methods
// If conversion methods are needed in the future, they can be re-added here

impl<'a> LayeredLayout<'a> {
    /// Creates a new empty layered layout
    pub fn new() -> Self {
        Self { layers: Vec::new() }
    }

    /// Adds a layer to the layout
    pub fn add_layer(&mut self, layer: Layer<'a>) {
        self.layers.push(layer);
        // Sort layers by z-index to ensure correct rendering order
        self.layers.sort_by_key(|layer| u32::MAX - layer.z_index); // OPTIMIZE: No need to sort every time.
    }

    /// Returns the number of layers
    pub fn len(&self) -> usize {
        self.layers.len()
    }

    /// Returns true if there are no layers
    pub fn is_empty(&self) -> bool {
        self.layers.is_empty()
    }
}
