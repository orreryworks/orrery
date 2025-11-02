use crate::{
    ast, draw,
    geometry::{self, Size},
    identifier::Id,
    layout::{layer, positioning::LayoutSizing},
    structure,
};
use log::{debug, error};
use std::{borrow::Cow, collections::HashMap, rc::Rc};

/// Represents a diagram component with a reference to its AST node and positioning information
/// TODO: Do I need Clone?!
/// Find a better name and location for this struct.
#[derive(Debug, Clone)]
pub struct Component {
    node_id: Id, // TODO: Can I get rid of this?
    drawable: Rc<draw::PositionedDrawable<draw::ShapeWithText>>, // TODO: Consider removing Rc.
}

impl Component {
    /// Creates a new component with the specified properties.
    pub fn new(
        node: &ast::Node,
        shape_with_text: draw::ShapeWithText,
        position: geometry::Point,
    ) -> Component {
        let drawable =
            Rc::new(draw::PositionedDrawable::new(shape_with_text).with_position(position));
        Component {
            node_id: node.id(),
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
    pub fn node_id(&self) -> Id {
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
pub struct LayoutRelation {
    source_index: usize,
    target_index: usize,
    arrow_with_text: draw::ArrowWithText,
}

impl LayoutRelation {
    /// Creates a new LayoutRelation from an AST relation and component indices.
    ///
    /// This method extracts the arrow definition and text from the AST relation
    /// and creates a self-contained LayoutRelation that doesn't depend on the
    /// original AST lifetime.
    ///
    /// # Arguments
    /// * `relation` - Reference to the AST relation being laid out
    /// * `source_index` - Index of the source component in the layout
    /// * `target_index` - Index of the target component in the layout
    ///
    /// # Returns
    /// A new LayoutRelation containing all necessary rendering information
    pub fn from_ast(relation: &ast::Relation, source_index: usize, target_index: usize) -> Self {
        let arrow_def = relation.clone_arrow_definition();
        let arrow = draw::Arrow::new(Cow::Owned(arrow_def), relation.arrow_direction());
        let mut arrow_with_text = draw::ArrowWithText::new(arrow);
        if let Some(text) = relation.text() {
            arrow_with_text.set_text(text);
        }
        Self {
            source_index,
            target_index,
            arrow_with_text,
        }
    }

    /// Returns a reference to the arrow with text for this relation.
    pub fn arrow_with_text(&self) -> &draw::ArrowWithText {
        &self.arrow_with_text
    }
}

/// Represents a complete layout of components and their relationships.
///
/// A `Layout` contains all the positioned components and their connecting relations
/// for a diagram. It provides methods to access related components and calculate
/// overall layout dimensions.
#[derive(Debug, Clone)]
pub struct Layout {
    components: Vec<Component>,
    relations: Vec<LayoutRelation>,
}

impl Layout {
    /// Creates a new layout with the given components and relations.
    pub fn new(components: Vec<Component>, relations: Vec<LayoutRelation>) -> Self {
        Self {
            components,
            relations,
        }
    }

    /// Returns a reference to the components in this layout.
    pub fn components(&self) -> &[Component] {
        &self.components
    }

    /// Returns a reference to the relations in this layout.
    pub fn relations(&self) -> &[LayoutRelation] {
        &self.relations
    }

    /// Returns a reference to the source component of the given relation.
    ///
    /// # Panics
    /// Panics if the source index is out of bounds.
    pub fn source(&self, lr: &LayoutRelation) -> &Component {
        &self.components[lr.source_index]
    }

    /// Returns a reference to the target component of the given relation.
    ///
    /// # Panics
    /// Panics if the target index is out of bounds.
    pub fn target(&self, lr: &LayoutRelation) -> &Component {
        &self.components[lr.target_index]
    }
}

impl LayoutSizing for Layout {
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
    content_stack: &mut layer::ContentStack<Layout>,
    graph: &'a structure::ComponentGraph<'a, '_>,
) {
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
            let node = graph.node_by_id(node_id).expect("Node must exist");

            // Find the component in the source layer that matches the node
            let source_component = source
                .content()
                .components()
                .iter()
                .find(|component| component.node_id == node.id())
                .expect("Component must exist in source layer");
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
                node_id:? = node.id(),
                source_offset:? = source.offset();
                "Adjusting positioned content offset [source]",
            );
            let target = content_stack.get_mut_unchecked(destination_idx);
            debug!(
                node_id:? = node.id(),
                original_offset:? = target.offset(),
                new_offset:? = target_offset;
                "Adjusting positioned content offset [target]",
            );
            target.set_offset(target_offset);
        }
    }
}
