use crate::{
    ast, draw,
    geometry::{self, Size},
    graph,
    layout::{layer, positioning::LayoutSizing},
};
use log::{debug, error};
use std::{collections::HashMap, rc::Rc};

/// Represents a diagram component with a reference to its AST node and positioning information
/// TODO: Do I need Clone?!
/// Find a better name and location for this struct.
#[derive(Debug, Clone)]
pub struct Component<'a> {
    node_id: &'a ast::TypeId, // TODO: Can I get rid of this?
    drawable: Rc<draw::PositionedDrawable<draw::ShapeWithText>>, // TODO: Consider removing Rc.
}

impl Component<'_> {
    /// Creates a new component with the specified properties.
    pub fn new<'a>(
        node: &'a ast::Node,
        shape_with_text: draw::ShapeWithText,
        position: geometry::Point,
    ) -> Component<'a> {
        let drawable =
            Rc::new(draw::PositionedDrawable::new(shape_with_text).with_position(position));
        Component {
            node_id: &node.id,
            drawable,
        }
    }

    /// Returns a reference to the component's shape.
    pub fn drawable(&self) -> &draw::PositionedDrawable<draw::ShapeWithText> {
        &self.drawable
    }

    /// Returns the center position of the component.
    ///
    /// The position represents the center point of the component in the layout
    /// coordinate system.
    pub fn position(&self) -> geometry::Point {
        self.drawable.position()
    }

    /// Calculates the bounds of this component
    ///
    /// The position is treated as the center of the component,
    /// and the bounds extend half the width/height in each direction.
    pub fn bounds(&self) -> geometry::Bounds {
        self.drawable.bounds()
    }

    /// Returns the unique identifier of the AST node this component represents.
    // TODO: Can I get rid of this method?
    pub fn node_id(&self) -> &ast::TypeId {
        self.node_id
    }
}

/// Represents a relation (connection) in a component layout with positional information.
///
/// LayoutRelation wraps an AST relation with additional layout-specific data,
/// including the indices of the source and target components within the layout.
/// This allows the layout system to efficiently reference components when
/// positioning and rendering relations.
#[derive(Debug, Clone)]
pub struct LayoutRelation<'a> {
    relation: &'a ast::Relation,
    source_index: usize,
    target_index: usize,
    text: Option<draw::Text>, // Optional text label for the relation
}

impl<'a> LayoutRelation<'a> {
    /// Creates a new LayoutRelation with the given relation and component indices.
    ///
    /// # Arguments
    /// * `relation` - Reference to the AST relation being laid out
    /// * `source_index` - Index of the source component in the layout
    /// * `target_index` - Index of the target component in the layout
    pub fn new(relation: &'a ast::Relation, source_index: usize, target_index: usize) -> Self {
        let text = relation.text();
        Self {
            relation,
            source_index,
            target_index,
            text,
        }
    }

    /// Returns a reference to the underlying AST relation.
    ///
    /// This provides access to the relation's properties such as type,
    /// attributes, and labels for rendering purposes.
    pub fn relation(&self) -> &ast::Relation {
        self.relation
    }

    pub fn text(&self) -> Option<&draw::Text> {
        self.text.as_ref()
    }
}

/// Represents a complete layout of components and their relationships.
///
/// A `Layout` contains all the positioned components and their connecting relations
/// for a diagram. It provides methods to access related components and calculate
/// overall layout dimensions.
#[derive(Debug, Clone)]
pub struct Layout<'a> {
    pub components: Vec<Component<'a>>,
    pub relations: Vec<LayoutRelation<'a>>,
}

impl<'a> Layout<'a> {
    /// Returns a reference to the source component of the given relation.
    pub fn source(&self, lr: &LayoutRelation<'a>) -> &Component<'a> {
        &self.components[lr.source_index]
    }

    /// Returns a reference to the target component of the given relation.
    pub fn target(&self, lr: &LayoutRelation<'a>) -> &Component<'a> {
        &self.components[lr.target_index]
    }
}

impl<'a> LayoutSizing for Layout<'a> {
    fn layout_size(&self) -> Size {
        // For component layouts, get the bounding box of all components
        if self.components.is_empty() {
            return Size::default();
        }

        // Calculate bounds from all components
        let bounds = self
            .components
            .iter()
            .skip(1)
            .fold(self.components[0].bounds(), |acc, comp| {
                acc.merge(&comp.bounds())
            });

        bounds.to_size()
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
/// # Panics
/// Panics if a component referenced in the containment graph is not found in its
/// corresponding layout layer.
// TODO: Once added enough abstractions, make this a method on ContentStack.
pub fn adjust_positioned_contents_offset<'a>(
    content_stack: &mut layer::ContentStack<Layout<'a>>,
    graph: &'a graph::Graph<'a>,
) {
    let container_indices: HashMap<_, _> = graph
        .containment_scopes()
        .iter()
        .enumerate()
        .filter_map(|(idx, scope)| scope.container().map(|container| (container, idx)))
        .collect();

    for (source_idx, source_scope) in graph.containment_scopes().iter().enumerate().rev() {
        for (node_idx, destination_idx) in source_scope.node_indices().filter_map(|node_idx| {
            container_indices
                .get(&node_idx)
                .map(|&destination_idx| (node_idx, destination_idx))
        }) {
            if source_idx == destination_idx {
                // If the source and destination are the same, skip
                error!(index = source_idx; "Source and destination indices are the same");
                continue;
            }
            let source = content_stack.get_unchecked(source_idx);
            let node = graph.node_from_idx(node_idx);

            // Find the component in the source layer that matches the node
            let source_component = source
                .content()
                .components
                .iter()
                .find(|component| component.node_id == &node.id)
                .expect("Component must exist in source layer");
            let target_offset = source
                .offset()
                .add(source_component.bounds().min_point())
                .add(
                    source_component
                        .drawable
                        .inner()
                        .shape_to_inner_content_min_point(),
                ); // TODO: This does not account for text.
            debug!(
                node_id:? = node.id,
                source_offset:? = source.offset();
                "Adjusting positioned content offset [source]",
            );
            let target = content_stack.get_mut_unchecked(destination_idx);
            debug!(
                node_id:? = node.id,
                original_offset:? = target.offset(),
                new_offset:? = target_offset;
                "Adjusting positioned content offset [target]",
            );
            target.set_offset(target_offset);
        }
    }
}
