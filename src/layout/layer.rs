use crate::{
    draw,
    geometry::{Bounds, Insets, Point, Size},
    layout::{component, positioning::LayoutSizing, sequence},
};
use log::debug;

/// Content types that can be laid out in a layer
#[derive(Debug)]
pub enum LayoutContent {
    Component(ContentStack<component::Layout>),
    Sequence(ContentStack<sequence::Layout>),
}

/// A rendering layer containing either component or sequence diagram content
#[derive(Debug)]
pub struct Layer {
    pub z_index: usize,
    /// Global coordinate offset for this layer
    pub offset: Point,
    /// Optional clipping boundary to keep content within parent
    pub clip_bounds: Option<Bounds>,
    /// The content of this layer
    pub content: LayoutContent, // TODO: Remove this one.
}

/// Collection of all diagram layers for rendering
#[derive(Debug)]
pub struct LayeredLayout {
    /// Ordered layers from bottom (0) to top
    /// Layers are rendered from bottom to top, with higher indices appearing on top
    layers: Vec<Layer>,
}

// LayerContent implementation was simplified by removing unused conversion methods
// If conversion methods are needed in the future, they can be re-added here

impl<'a> LayeredLayout {
    /// Creates a new empty layered layout
    pub fn new() -> Self {
        Self { layers: Vec::new() }
    }

    /// Adds a layer to the layout and returns its index.
    ///
    /// The z_index is assigned based on the layer's position in the stack,
    /// with higher indices (newer layers) appearing on top.
    pub fn add_layer(&mut self, content: LayoutContent) -> usize {
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
        positioned_shape: &draw::PositionedDrawable<draw::ShapeWithText>,
        embedded_idx: usize,
        padding: Insets,
    ) {
        let [container_layer, embedded_layer] = &mut self
            .layers
            .get_disjoint_mut([container_idx, embedded_idx])
            .expect("container_idx and embedded_idx must be valid, distinct indices");

        // Calculate the bounds of the container
        let container_bounds = positioned_shape.bounds();

        debug!(
            positioned_shape:?, container_bounds:?,
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
            .add_point(container_layer.offset)
            .add_point(container_bounds.min_point())
            .add_point(positioned_shape.inner().shape_to_inner_content_min_point());

        // Set clip bounds with padding
        let padded_clip_bounds = container_bounds
            .inverse_translate(container_bounds.min_point())
            .translate(Point::new(-padding.left(), -padding.top()));

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
    pub fn iter_from_bottom(&'a self) -> impl Iterator<Item = &'a Layer> {
        self.layers.iter().rev()
    }
}

/// A stack of positioned content items for layout management.
///
/// ContentStack manages a collection of positioned content items, where each item
/// has both content and an offset position.
#[derive(Debug, Clone)]
pub struct ContentStack<T: LayoutSizing>(Vec<PositionedContent<T>>);

impl<T> ContentStack<T>
where
    T: LayoutSizing,
{
    /// Creates a new empty content stack.
    pub fn new() -> Self {
        Self(Vec::new())
    }

    /// Returns a reference to the positioned content at the given index without bounds checking.
    ///
    /// # Safety
    /// The caller must ensure that the index is within bounds.
    pub fn get_unchecked(&self, index: usize) -> &PositionedContent<T> {
        &self.0[index]
    }

    /// Returns a mutable reference to the positioned content at the given index without bounds checking.
    ///
    /// # Safety
    /// The caller must ensure that the index is within bounds.
    pub fn get_mut_unchecked(&mut self, index: usize) -> &mut PositionedContent<T> {
        &mut self.0[index]
    }

    /// Adds a positioned content item to the stack.
    pub fn push(&mut self, positioned_content: PositionedContent<T>) {
        self.0.push(positioned_content);
    }

    /// Returns the layout size of this content stack.
    ///
    /// For a content stack, this returns the size of the last positioned content item,
    /// as it represents the final computed layout. If the stack is empty, returns
    /// a default (zero) size.
    pub fn layout_size(&self) -> Size {
        // For content stack, return the size of the last positioned content's content
        self.0
            .last()
            .map(|content| content.layout_size())
            .unwrap_or_default()
    }

    /// Returns an iterator over the positioned content items.
    pub fn iter(&self) -> impl DoubleEndedIterator<Item = &PositionedContent<T>> {
        self.0.iter()
    }

    /// Returns the number of positioned content items in the stack.
    pub fn len(&self) -> usize {
        self.0.len()
    }
}

/// A content item with an associated position offset.
///
/// PositionedContent wraps layout content with positional information,
/// allowing content to be placed at specific coordinates within a larger layout.
#[derive(Debug, Clone)]
pub struct PositionedContent<T>
where
    T: LayoutSizing,
{
    offset: Point,
    content: T,
}

impl<T> PositionedContent<T>
where
    T: LayoutSizing,
{
    /// Creates new positioned content with the given content and default (zero) offset.
    pub fn new(content: T) -> Self {
        Self {
            content,
            offset: Point::default(),
        }
    }

    /// Returns the position offset for this content.
    pub fn offset(&self) -> Point {
        self.offset
    }

    /// Returns a reference to the content.
    pub fn content(&self) -> &T {
        &self.content
    }

    /// Sets the position offset for this content.
    pub fn set_offset(&mut self, offset: Point) {
        self.offset = offset;
    }

    /// Returns the layout size of the contained content.
    pub fn layout_size(&self) -> Size {
        self.content.layout_size()
    }
}
