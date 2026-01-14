use log::debug;

use filament_core::{
    draw,
    geometry::{Bounds, Point, Size},
};

use crate::{
    error::FilamentError,
    layout::{component, positioning::LayoutBounds, sequence},
};

/// Content types that can be laid out in a layer
#[derive(Debug)]
pub enum LayoutContent<'a> {
    Component(ContentStack<component::Layout<'a>>),
    Sequence(ContentStack<sequence::Layout<'a>>),
}

/// A rendering layer containing either component or sequence diagram content
#[derive(Debug)]
pub struct Layer<'a> {
    z_index: usize,
    /// Global coordinate offset for this layer
    offset: Point,
    /// Optional clipping boundary to keep content within parent
    clip_bounds: Option<Bounds>,
    /// The content of this layer
    content: LayoutContent<'a>, // TODO: Remove this one.
}

impl<'a> Layer<'a> {
    /// Create a new layer with the given z-index and content.
    fn new(z_index: usize, content: LayoutContent<'a>) -> Self {
        Self {
            z_index,
            offset: Point::default(),
            clip_bounds: None,
            content,
        }
    }

    /// Get this layer's z-index (render order).
    pub fn z_index(&self) -> usize {
        self.z_index
    }

    /// Get the global offset applied to this layer.
    pub fn offset(&self) -> Point {
        self.offset
    }

    /// Get the clipping bounds if present.
    pub fn clip_bounds(&self) -> Option<Bounds> {
        self.clip_bounds
    }

    /// Access the content for this layer.
    pub fn content(&self) -> &LayoutContent<'_> {
        &self.content
    }

    /// Set the global offset applied to this layer.
    fn set_offset(&mut self, offset: Point) {
        self.offset = offset;
    }

    /// Set or clear the clipping bounds.
    fn set_clip_bounds(&mut self, clip_bounds: Option<Bounds>) {
        self.clip_bounds = clip_bounds;
    }
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
    pub fn add_layer(&mut self, content: LayoutContent<'a>) -> usize {
        let z_index = self.layers.len();

        self.layers.push(Layer::new(z_index, content));
        z_index
    }

    /// Adjusts the position of an embedded diagram within its container and sets up clipping
    ///
    /// - `container_idx`: Index of the container layer
    /// - `positioned_shape`: The positioned drawable representing the container
    /// - `embedded_idx`: Index of the embedded diagram layer
    ///
    /// # Errors
    /// Returns an error if either index is invalid, if they refer to the same layer,
    /// or if the container shape doesn't have content bounds set.
    pub fn adjust_relative_position(
        &mut self,
        container_idx: usize,
        positioned_shape: &draw::PositionedDrawable<draw::ShapeWithText>,
        embedded_idx: usize,
    ) -> Result<(), FilamentError> {
        let [container_layer, embedded_layer] =
            self.layers
                .get_disjoint_mut([container_idx, embedded_idx])
                .map_err(|err| {
                    FilamentError::Layout(format!(
                        "Invalid layer indices (container_idx={container_idx}, embedded_idx={embedded_idx}): {err}"
                    ))
                })?;

        let content_bounds = positioned_shape.content_bounds().ok_or_else(|| {
            FilamentError::Layout(
                "Container shape must have inner content size set for embedded diagram positioning"
                    .to_string(),
            )
        })?;

        // Get the actual bounds of the embedded layout's content
        let embedded_layout_bounds = match embedded_layer.content() {
            LayoutContent::Component(layout) => layout
                .iter()
                .last()
                .map(|content| content.content().layout_bounds())
                .unwrap_or_default(),
            LayoutContent::Sequence(layout) => layout
                .iter()
                .last()
                .map(|content| content.content().layout_bounds())
                .unwrap_or_default(),
        };

        debug!(
            positioned_shape:?, content_bounds:?,
            container_idx, container_offset:?=container_layer.offset(), container_clip_bounds:?=container_layer.clip_bounds(),
            embedded_idx, embedded_offset:?=embedded_layer.offset(), embedded_clip_bounds:?=embedded_layer.clip_bounds();
            "Embedded layer before adjustment",
        );

        // Apply transformations to position the embedded diagram:
        // 1. Add the container layer's offset (for nested containers)
        // 2. Add the content area's position (where we want content to start)
        // 3. Subtract the embedded layout's min_point (where content actually starts in its local coords)
        //    This aligns the embedded content's origin with the container's content area
        embedded_layer.set_offset(
            embedded_layer
                .offset()
                .add_point(container_layer.offset())
                .add_point(content_bounds.min_point())
                .add_point(Point::new(
                    -embedded_layout_bounds.min_x(),
                    -embedded_layout_bounds.min_y(),
                )),
        );

        embedded_layer.set_clip_bounds(Some(embedded_layout_bounds));

        debug!(
            offset:?=embedded_layer.offset(), clip_bounds:?=embedded_layer.clip_bounds();
            "Adjusted embedded layer",
        );

        Ok(())
    }

    /// Returns the number of layers
    pub fn len(&self) -> usize {
        self.layers.len()
    }

    /// Returns an iterator over the layers, starting from the bottom (background) layer
    /// This ordering is appropriate for rendering, where bottom layers should be drawn first
    pub fn iter_from_bottom(&'a self) -> impl Iterator<Item = &'a Layer<'a>> {
        self.layers.iter().rev()
    }
}

/// A stack of positioned content items for layout management.
///
/// ContentStack manages a collection of positioned content items, where each item
/// has both content and an offset position.
#[derive(Debug, Clone)]
pub struct ContentStack<T: LayoutBounds>(Vec<PositionedContent<T>>);

impl<T> ContentStack<T>
where
    T: LayoutBounds,
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
    T: LayoutBounds,
{
    offset: Point,
    content: T,
}

impl<T> PositionedContent<T>
where
    T: LayoutBounds,
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
