//! Positioned diagram elements and their relationships.
//!
//! This module provides the core layout types for representing diagram elements
//! with computed positions and sizes. A [`Component`] wraps a semantic node with
//! positioning information and is used across all diagram kinds.
//!
//! # Types
//!
//! - [`Component`] - A positioned diagram element with bounds
//! - [`Layout`] - A complete layout of components and their connecting relations
//!
//! # Functions
//!
//! - [`positioned_arrow_from_relation`] - Constructs a positioned arrow from a semantic relation
//! - [`adjust_positioned_contents_offset`] - Adjusts nested content offsets based on containment

use std::{collections::HashMap, rc::Rc};

use log::{debug, error};

use orrery_core::{
    draw,
    geometry::{Bounds, Point},
    identifier::Id,
    semantic,
};

use crate::{
    error::RenderError,
    layout::{layer, positioning::LayoutBounds},
    structure,
};

// TODO: Do I need Clone?!
// TODO: Find a better name and location for this struct.
/// A positioned diagram component linking a semantic node to its rendered shape and location.
#[derive(Debug, Clone)]
pub struct Component<'a> {
    node_id: Id, // TODO: Can I get rid of this?
    drawable: Rc<draw::PositionedDrawable<draw::ShapeWithText<'a>>>, // TODO: Consider removing Rc.
}

impl<'a> Component<'a> {
    /// Creates a new component with the specified properties.
    pub fn new(
        node: &semantic::Node,
        shape_with_text: draw::ShapeWithText<'a>,
        position: Point,
    ) -> Component<'a> {
        let drawable =
            Rc::new(draw::PositionedDrawable::new(shape_with_text).with_position(position));
        Component {
            node_id: node.id(),
            drawable,
        }
    }

    /// Returns a reference to the component's shape.
    pub fn drawable(&self) -> &draw::PositionedDrawable<draw::ShapeWithText<'_>> {
        &self.drawable
    }

    /// Returns the center position of the component.
    ///
    /// The position represents the center point of the component in the layout
    /// coordinate system.
    pub fn position(&self) -> Point {
        self.drawable.position()
    }

    /// Calculates the bounds of this component.
    ///
    /// The position is treated as the center of the component,
    /// and the bounds extend half the width/height in each direction.
    pub fn bounds(&self) -> Bounds {
        self.drawable.bounds()
    }

    /// Returns the unique identifier of the AST node this component represents.
    // TODO: Can I get rid of this method?
    pub fn node_id(&self) -> Id {
        self.node_id
    }

    /// Calculates the intersection point where a line from this component's center
    /// to an external point crosses this component's shape boundary.
    ///
    /// # Arguments
    ///
    /// * `external_point` - The point to draw a line toward from this component's center.
    ///
    /// # Returns
    ///
    /// The point on this component's shape boundary where the line exits.
    pub fn find_intersection(&self, external_point: Point) -> Point {
        self.drawable
            .inner()
            .find_intersection(self.position(), external_point)
    }
}

/// Creates a [`PositionedArrowWithText`](draw::PositionedArrowWithText) from a semantic relation
/// and positioned source/target components.
///
/// Computes the arrow path by finding the intersection points between the
/// line connecting the source and target centers and each component's shape boundary.
///
/// # Arguments
///
/// * `relation` - The semantic relation to extract arrow definition and text from.
/// * `source` - The source component (for boundary intersection calculation).
/// * `target` - The target component (for boundary intersection calculation).
///
/// # Returns
///
/// A fully positioned arrow ready for rendering.
pub fn positioned_arrow_from_relation<'a>(
    relation: &'a semantic::Relation,
    source: &Component,
    target: &Component,
) -> draw::PositionedArrowWithText<'a> {
    let arrow_def = Rc::clone(relation.arrow_definition());
    let arrow = draw::Arrow::new(arrow_def, relation.arrow_direction());
    let arrow_with_text = draw::ArrowWithText::new(arrow, relation.text());

    let source_edge = source.find_intersection(target.position());
    let target_edge = target.find_intersection(source.position());
    let path = draw::ArrowPath::straight(source_edge, target_edge);

    draw::PositionedArrowWithText::new(arrow_with_text, path)
}

/// A complete layout of components and their relationships.
///
/// A `Layout` contains all the positioned components and their connecting relations
/// for a diagram. It provides methods to access related components and calculate
/// overall layout dimensions.
#[derive(Debug, Clone)]
pub struct Layout<'a> {
    components: Vec<Component<'a>>,
    relations: Vec<draw::PositionedArrowWithText<'a>>,
    bounds: Bounds,
}

impl<'a> Layout<'a> {
    /// Creates a new layout with the given components and relations.
    pub fn new(
        components: Vec<Component<'a>>,
        relations: Vec<draw::PositionedArrowWithText<'a>>,
    ) -> Self {
        let bounds = if components.is_empty() {
            Bounds::default()
        } else {
            components
                .iter()
                .skip(1)
                .fold(components[0].bounds(), |acc, comp| {
                    acc.merge(&comp.bounds())
                })
        };

        Self {
            components,
            relations,
            bounds,
        }
    }

    /// Returns a reference to the components in this layout.
    pub fn components(&self) -> &[Component<'a>] {
        &self.components
    }

    /// Returns a reference to the relations in this layout.
    pub fn relations(&self) -> &[draw::PositionedArrowWithText<'a>] {
        &self.relations
    }
}

impl<'a> LayoutBounds for Layout<'a> {
    fn layout_bounds(&self) -> Bounds {
        self.bounds
    }
}

/// Adjusts the offset of positioned contents in a content stack based on containment relationships.
///
/// This function handles the proper positioning of nested elements within their containers.
///
/// # Arguments
/// * `content_stack` - Mutable reference to the content stack containing all layout layers
/// * `graph` - Reference to the containment graph that defines parent-child relationships
///
/// # Behavior
/// The function processes containment scopes in reverse order to ensure proper nesting.
/// For each nested element, it:
/// 1. Finds the container component in the source layer
/// 2. Calculates the target offset based on the container's bounds and shape properties
/// 3. Updates the destination layer's offset to position the nested content correctly
///
/// # Errors
/// Returns `RenderError::Layout` if a component referenced in the containment graph
/// is not found in its corresponding layout layer.
// TODO: Once added enough abstractions, make this a method on ContentStack.
pub fn adjust_positioned_contents_offset<'a>(
    content_stack: &mut layer::ContentStack<Layout>,
    graph: &'a structure::ComponentGraph<'a, '_>,
) -> Result<(), RenderError> {
    let container_indices: HashMap<_, _> = graph
        .containment_scopes()
        .enumerate()
        .filter_map(|(idx, scope)| scope.container().map(|container| (container, idx)))
        .collect();

    for (source_idx, source_scope) in graph.containment_scopes().enumerate().rev() {
        for (node_id, destination_idx) in source_scope.node_ids().filter_map(|node_id| {
            container_indices
                .get(&node_id)
                .map(|&destination_idx| (node_id, destination_idx))
        }) {
            if source_idx == destination_idx {
                // If the source and destination are the same, skip
                error!(index = source_idx; "Source and destination indices are the same");
                continue;
            }
            let source = content_stack.get_unchecked(source_idx);
            let node = graph.node_by_id(node_id).ok_or_else(|| {
                RenderError::Layout(format!(
                    "Node with id {node_id} not found in graph during layout adjustment"
                ))
            })?;

            // Find the component in the source layer that matches the node
            let source_component = source
                .content()
                .components()
                .iter()
                .find(|component| component.node_id == node.id())
                .ok_or_else(|| {
                    RenderError::Layout(format!(
                        "Component with id {node} not found in source layer {source_idx}"
                    ))
                })?;
            let target_offset = source
                .offset()
                .add_point(source_component.bounds().min_point())
                .add_point(
                    source_component
                        .drawable
                        .inner()
                        .shape_to_inner_content_min_point(),
                ); // TODO: This does not account for text.
            debug!(
                node_id:% = node,
                source_offset:? = source.offset();
                "Adjusting positioned content offset [source]",
            );
            let target = content_stack.get_mut_unchecked(destination_idx);
            debug!(
                node_id:% = node,
                original_offset:? = target.offset(),
                new_offset:? = target_offset;
                "Adjusting positioned content offset [target]",
            );
            target.set_offset(target_offset);
        }
    }
    Ok(())
}
