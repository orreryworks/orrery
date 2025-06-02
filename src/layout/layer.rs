use crate::layout::{
    component,
    geometry::{Bounds, Point, Size},
    sequence,
};
use log::debug;

/// Content types that can be rendered in a layer
#[derive(Debug)]
pub enum LayerContent<'a> {
    Component(component::Layout<'a>),
    Sequence(sequence::Layout<'a>),
}

/// A rendering layer containing either component or sequence diagram content
#[derive(Debug)]
pub struct Layer<'a> {
    pub z_index: usize,
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
    /// Layers are rendered from bottom to top, with higher indices appearing on top
    layers: Vec<Layer<'a>>,
}

// LayerContent implementation was simplified by removing unused conversion methods
// If conversion methods are needed in the future, they can be re-added here

impl<'a> LayeredLayout<'a> {
    /// Creates a new empty layered layout
    pub fn new() -> Self {
        Self { layers: Vec::new() }
    }

    /// Adds a layer to the layout and returns its index.
    ///
    /// The z_index is assigned based on the layer's position in the stack,
    /// with higher indices (newer layers) appearing on top.
    pub fn add_layer(&mut self, content: LayerContent<'a>) -> usize {
        let z_index = self.layers.len();

        self.layers.push(Layer {
            z_index,
            offset: Point::default(),
            clip_bounds: None,
            content,
        });
        z_index
    }

    /// Adjusts the position of an embedded diagram within its container and sets up clipping
    ///
    /// - `container_idx`: Index of the container layer
    /// - `container_position`: Position of the container component
    /// - `container_size`: Size of the container component
    /// - `embedded_idx`: Index of the embedded diagram layer
    /// - `padding`: Padding to apply between container edges and embedded content
    ///
    /// # Panics
    /// Panics if either index is invalid or if they refer to the same layer
    // TODO: We can return error instead of panicking if indices are invalid
    pub fn adjust_relative_position(
        &mut self,
        container_idx: usize,
        container_position: Point,
        container_size: Size,
        embedded_idx: usize,
        padding: f32,
    ) {
        let [container_layer, embedded_layer] = &mut self
            .layers
            .get_disjoint_mut([container_idx, embedded_idx])
            .expect("container_idx and embedded_idx must be valid, distinct indices");

        // Calculate the bounds of the container
        let container_bounds = container_position.to_bounds(container_size);

        debug!(
            container_position:?, container_size:?, container_bounds:?, padding,
            container_idx, container_offset:?=container_layer.offset, container_clip_bounds:?=container_layer.clip_bounds,
            embedded_idx, embedded_offset:?=embedded_layer.offset, embedded_clip_bounds:?=embedded_layer.clip_bounds;
            "Embedded layer before adjustment",
        );

        // Apply three transformations to position the embedded diagram:
        // 1. Add the container layer's offset (for nested containers)
        // 2. Add the container's top-left position
        // 3. Add padding to inset from the edges
        embedded_layer.offset = embedded_layer
            .offset
            .add(container_layer.offset)
            .add(container_bounds.min_point())
            .add(Point::new(padding, padding));

        // Set clip bounds with padding
        let padded_clip_bounds = container_bounds
            .inverse_translate(container_bounds.min_point())
            .translate(Point::new(-padding, -padding));

        embedded_layer.clip_bounds = Some(padded_clip_bounds);

        debug!(
            offset:?=embedded_layer.offset, clip_bounds:?=embedded_layer.clip_bounds;
            "Adjusted embedded layer",
        );
    }

    /// Returns the number of layers
    pub fn len(&self) -> usize {
        self.layers.len()
    }

    /// Returns true if there are no layers
    pub fn is_empty(&self) -> bool {
        self.layers.is_empty()
    }

    /// Returns an iterator over the layers, starting from the bottom (background) layer
    /// This ordering is appropriate for rendering, where bottom layers should be drawn first
    pub fn iter_from_bottom(&'a self) -> impl Iterator<Item = &'a Layer<'a>> {
        self.layers.iter().rev()
    }
}
