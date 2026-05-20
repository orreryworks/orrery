//! Positioned diagram elements and arrow placement.
//!
//! A [`Component`] wraps a semantic node with its computed position and shape.
//! An [`ArrowPlacer`] decides how relations between the same component pair
//! are turned into visually distinct arrow paths (straight overlap vs. offset
//! cubic-Bézier lanes).

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

/// Strategy for routing a bucket of relations between the same unordered
/// component pair into positioned arrows.
///
/// # Implementing
///
/// - `source` is the component whose [`node_id`](Component::node_id) is the
///   *canonical* end of the pair, and `target` is the other end. For
///   self-loop buckets the same [`Component`] is passed for both.
/// - Relations within the bucket may go either direction; inspect
///   [`Relation::source`](semantic::Relation::source) to determine orientation.
/// - Return exactly one arrow per input relation, in the same order.
pub trait ArrowPlacer {
    /// Places a bucket of relations between the same component pair.
    fn place<'a>(
        &self,
        relations: &[&'a semantic::Relation],
        source: &Component<'_>,
        target: &Component<'_>,
    ) -> Vec<draw::PositionedArrowWithText<'a>>;
}

/// [`ArrowPlacer`] that emits a straight centerline path for every relation.
///
/// Parallel/reverse relations overlap into a single line. Exists as the
/// behaviour-preserving baseline; [`CurvedArrowPlacer`] is the default.
#[derive(Debug, Default, Clone, Copy)]
pub struct StraightArrowPlacer;

impl ArrowPlacer for StraightArrowPlacer {
    fn place<'a>(
        &self,
        relations: &[&'a semantic::Relation],
        source: &Component<'_>,
        target: &Component<'_>,
    ) -> Vec<draw::PositionedArrowWithText<'a>> {
        relations
            .iter()
            .map(|relation| {
                let (rel_src, rel_tgt) = align_to_relation(relation, source, target);
                positioned_arrow_from_relation(relation, rel_src, rel_tgt)
            })
            .collect()
    }
}

/// [`ArrowPlacer`] that places parallel/reverse relations on offset lanes
/// encoded as cubic Bézier curves.
///
/// Each relation at bucket position `k` of `N` gets lane offset
/// `(k - (N-1)/2) * lane_spacing`. Reverse-direction relations have their
/// offset sign-flipped so `a -> b` and `b -> a` curve to opposite sides.
///
/// Lane endpoints are found by aiming each shape's intersection ray at the
/// offset center of the other shape; control points sit at 1/3 and 2/3 of
/// the resulting lane line. For `N == 1` the offset is zero, producing a
/// straight path identical to [`StraightArrowPlacer`]. Self-loop buckets
/// currently fall back to straight lines.
#[derive(Debug, Clone, Copy)]
pub struct CurvedArrowPlacer {
    lane_spacing: f32,
}

impl CurvedArrowPlacer {
    const DEFAULT_LANE_SPACING: f32 = 22.0;

    pub fn new() -> Self {
        Self {
            lane_spacing: Self::DEFAULT_LANE_SPACING,
        }
    }

    /// Packages `lane_geometry` output into a [`PositionedArrowWithText`](draw::PositionedArrowWithText).
    fn curved_arrow<'a>(
        relation: &'a semantic::Relation,
        source: &Component<'_>,
        target: &Component<'_>,
        lane_offset: f32,
    ) -> draw::PositionedArrowWithText<'a> {
        let Some((path, label_position)) = Self::lane_geometry(source, target, lane_offset) else {
            // Degenerate (zero-length centerline): fall back to straight.
            return positioned_arrow_from_relation(relation, source, target);
        };

        let arrow_def = Rc::clone(relation.arrow_definition());
        let arrow = draw::Arrow::new(arrow_def, relation.arrow_direction());
        let arrow_with_text = draw::ArrowWithText::new(arrow, relation.text());

        draw::PositionedArrowWithText::new(arrow_with_text, path)
            .with_text_position(Some(label_position))
    }

    /// Computes the cubic-Bézier path and label position for a lane offset.
    ///
    /// Returns `None` when component centers coincide (zero-length centerline).
    fn lane_geometry(
        source: &Component<'_>,
        target: &Component<'_>,
        lane_offset: f32,
    ) -> Option<(draw::ArrowPath, Point)> {
        let src_center = source.position();
        let tgt_center = target.position();

        let delta = tgt_center.sub_point(src_center);
        let len = delta.hypot();
        if len == 0.0 {
            return None;
        }
        let perp = Point::new(
            -delta.y() / len * lane_offset,
            delta.x() / len * lane_offset,
        );

        let midpoint = src_center.midpoint(tgt_center).add_point(perp);
        let src_edge = source.find_intersection(midpoint);
        let tgt_edge = target.find_intersection(midpoint);

        // Control points at 1/3 and 2/3 of src_edge→tgt_edge, offset perpendicular.
        let (third, two_thirds) = line_segment_thirds(src_edge, tgt_edge);
        let cp1 = third.add_point(perp);
        let cp2 = two_thirds.add_point(perp);

        let path = draw::ArrowPath::new(src_edge, tgt_edge, vec![cp1, cp2]);
        let label_position = cubic_bezier_midpoint(src_edge, cp1, cp2, tgt_edge);
        Some((path, label_position))
    }

    /// Lane offset for position `k` of `n`, sign-flipped for reverse relations.
    fn lane_offset_at(
        &self,
        k: usize,
        n: usize,
        relation: &semantic::Relation,
        source_id: Id,
    ) -> f32 {
        let offset = ((k as f32) - ((n - 1) as f32) / 2.0) * self.lane_spacing;
        if relation.source() != source_id {
            -offset
        } else {
            offset
        }
    }
}

impl Default for CurvedArrowPlacer {
    fn default() -> Self {
        Self::new()
    }
}

impl ArrowPlacer for CurvedArrowPlacer {
    fn place<'a>(
        &self,
        relations: &[&'a semantic::Relation],
        source: &Component<'_>,
        target: &Component<'_>,
    ) -> Vec<draw::PositionedArrowWithText<'a>> {
        // Self-loop bucket: fall back to straight lines until a dedicated
        // self-loop router lands.
        if source.node_id() == target.node_id() {
            return StraightArrowPlacer.place(relations, source, target);
        }

        let n = relations.len();
        relations
            .iter()
            .enumerate()
            .map(|(k, relation)| {
                let (rel_src, rel_tgt) = align_to_relation(relation, source, target);

                let lane_offset = self.lane_offset_at(k, n, relation, source.node_id());

                if lane_offset == 0.0 {
                    // Median lane (or N == 1): straight line.
                    return positioned_arrow_from_relation(relation, rel_src, rel_tgt);
                }

                Self::curved_arrow(relation, rel_src, rel_tgt, lane_offset)
            })
            .collect()
    }
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

/// Returns `(source, target)` or `(target, source)` depending on which
/// component matches `relation.source()`.
fn align_to_relation<'a, 'b>(
    relation: &semantic::Relation,
    source: &'b Component<'a>,
    target: &'b Component<'a>,
) -> (&'b Component<'a>, &'b Component<'a>) {
    if relation.source() == source.node_id() {
        (source, target)
    } else {
        (target, source)
    }
}

fn line_segment_thirds(start: Point, end: Point) -> (Point, Point) {
    let dx = (end.x() - start.x()) / 3.0;
    let dy = (end.y() - start.y()) / 3.0;
    (
        Point::new(start.x() + dx, start.y() + dy),
        Point::new(start.x() + 2.0 * dx, start.y() + 2.0 * dy),
    )
}

fn cubic_bezier_midpoint(start: Point, cp1: Point, cp2: Point, end: Point) -> Point {
    Point::new(
        0.125 * start.x() + 0.375 * cp1.x() + 0.375 * cp2.x() + 0.125 * end.x(),
        0.125 * start.y() + 0.375 * cp1.y() + 0.375 * cp2.y() + 0.125 * end.y(),
    )
}

#[cfg(test)]
mod tests {
    use float_cmp::{approx_eq, assert_approx_eq};

    use super::*;

    fn assert_point_approx_eq(actual: Point, expected: Point) {
        assert!(
            approx_eq!(f32, actual.x(), expected.x(), epsilon = 0.01)
                && approx_eq!(f32, actual.y(), expected.y(), epsilon = 0.01),
            "expected ({}, {}), got ({}, {})",
            expected.x(),
            expected.y(),
            actual.x(),
            actual.y(),
        );
    }

    fn make_relation(source: Id, target: Id) -> semantic::Relation {
        semantic::Relation::new(
            source,
            target,
            draw::ArrowDirection::Forward,
            None,
            Rc::new(draw::ArrowDefinition::default()),
        )
    }

    fn make_node(name: &str) -> semantic::Node {
        let id = Id::new(name);
        let shape_def =
            Rc::new(Box::new(draw::RectangleDefinition::new()) as Box<dyn draw::ShapeDefinition>);
        semantic::Node::new(id, None, semantic::Block::None, shape_def)
    }

    fn make_component<'a>(node: &'a semantic::Node, position: Point) -> Component<'a> {
        let shape = draw::Shape::new(Rc::clone(node.shape_definition()));
        let shape_with_text = draw::ShapeWithText::new(shape, None);
        Component::new(node, shape_with_text, position)
    }

    #[test]
    fn line_segment_thirds_returns_third_points() {
        let (cp1, cp2) = line_segment_thirds(Point::new(0.0, 0.0), Point::new(30.0, 60.0));
        assert_eq!(cp1, Point::new(10.0, 20.0));
        assert_eq!(cp2, Point::new(20.0, 40.0));
    }

    #[test]
    fn cubic_bezier_midpoint_at_half() {
        // For a degenerate cubic (control points colinear with endpoints), the
        // parametric midpoint coincides with the geometric midpoint.
        let s = Point::new(0.0, 0.0);
        let cp1 = Point::new(30.0, 0.0);
        let cp2 = Point::new(60.0, 0.0);
        let d = Point::new(90.0, 0.0);
        let mid = cubic_bezier_midpoint(s, cp1, cp2, d);
        assert_eq!(mid, Point::new(45.0, 0.0));
    }

    #[test]
    fn lane_offset_at_assigns_symmetric_lanes() {
        let a_id = Id::new("a");
        let b_id = Id::new("b");
        let r0 = make_relation(a_id, b_id);
        let r1 = make_relation(a_id, b_id);
        let router = CurvedArrowPlacer { lane_spacing: 10.0 };

        let off0 = router.lane_offset_at(0, 2, &r0, a_id);
        let off1 = router.lane_offset_at(1, 2, &r1, a_id);
        assert_eq!(off0, -5.0);
        assert_eq!(off1, 5.0);
    }

    #[test]
    fn lane_offset_at_three_relations_has_zero_median() {
        let a_id = Id::new("a");
        let b_id = Id::new("b");
        let r = make_relation(a_id, b_id);
        let router = CurvedArrowPlacer { lane_spacing: 10.0 };
        assert_eq!(router.lane_offset_at(1, 3, &r, a_id), 0.0);
        assert_eq!(router.lane_offset_at(2, 3, &r, a_id), 10.0);
    }

    #[test]
    fn lane_offset_at_single_relation_is_zero() {
        let a_id = Id::new("a");
        let b_id = Id::new("b");
        let r = make_relation(a_id, b_id);
        let router = CurvedArrowPlacer::new();
        assert_eq!(router.lane_offset_at(0, 1, &r, a_id), 0.0);
    }

    #[test]
    fn lane_geometry_emits_cubic_bezier_with_two_control_points() {
        // Shape size is 12x12 (half = 6). Components at (0,0) and (1000,0).
        // lane_offset = 18, perp = (0, 18), midpoint = (500, 18).
        // src_edge: ray from (0,0) toward (500,18) exits at x=6 → (6, 0.216).
        // tgt_edge: ray from (1000,0) toward (500,18) exits at x=994 → (994, 0.216).
        // cp1 = 1/3 of edge segment + perp = (335.33, 18.216).
        // cp2 = 2/3 of edge segment + perp = (664.67, 18.216).
        // label = cubic midpoint ≈ (500, 13.716).
        let a_node = make_node("a");
        let b_node = make_node("b");
        let a = make_component(&a_node, Point::new(0.0, 0.0));
        let b = make_component(&b_node, Point::new(1000.0, 0.0));

        let (path, label) =
            CurvedArrowPlacer::lane_geometry(&a, &b, 18.0).expect("non-degenerate centerline");

        assert_eq!(path.control_points().len(), 2);

        assert_point_approx_eq(path.source(), Point::new(6.0, 0.216));
        assert_point_approx_eq(path.destination(), Point::new(994.0, 0.216));
        assert_point_approx_eq(path.control_points()[0], Point::new(335.33, 18.216));
        assert_point_approx_eq(path.control_points()[1], Point::new(664.67, 18.216));
        assert_point_approx_eq(label, Point::new(500.0, 13.716));
    }

    #[test]
    fn lane_geometry_opposite_offsets_produce_mirrored_control_points() {
        let a_node = make_node("a");
        let b_node = make_node("b");
        let a = make_component(&a_node, Point::new(0.0, 0.0));
        let b = make_component(&b_node, Point::new(1000.0, 0.0));

        let (path_pos, _) = CurvedArrowPlacer::lane_geometry(&a, &b, 18.0).unwrap();
        let (path_neg, _) = CurvedArrowPlacer::lane_geometry(&a, &b, -18.0).unwrap();

        let cp1_pos = path_pos.control_points()[0];
        let cp1_neg = path_neg.control_points()[0];
        assert_point_approx_eq(cp1_neg, Point::new(cp1_pos.x(), -cp1_pos.y()));
    }

    #[test]
    fn lane_geometry_offset_scales_with_magnitude() {
        let a_node = make_node("a");
        let b_node = make_node("b");
        let a = make_component(&a_node, Point::new(0.0, 0.0));
        let b = make_component(&b_node, Point::new(1000.0, 0.0));

        let (path_small, _) = CurvedArrowPlacer::lane_geometry(&a, &b, 10.0).unwrap();
        let (path_large, _) = CurvedArrowPlacer::lane_geometry(&a, &b, 40.0).unwrap();

        let cp1_small = path_small.control_points()[0];
        let cp1_large = path_large.control_points()[0];
        // 4x lane offset → 4x perpendicular offset on control points.
        assert_approx_eq!(f32, cp1_large.y() / cp1_small.y(), 4.0);
    }

    #[test]
    fn lane_geometry_returns_none_for_zero_length_centerline() {
        let a_node = make_node("a");
        let a = make_component(&a_node, Point::new(0.0, 0.0));
        assert!(CurvedArrowPlacer::lane_geometry(&a, &a, 18.0).is_none());
    }

    #[test]
    fn place_produces_one_arrow_per_relation() {
        let a_node = make_node("a");
        let b_node = make_node("b");
        let a = make_component(&a_node, Point::new(0.0, 0.0));
        let b = make_component(&b_node, Point::new(100.0, 0.0));
        let r1 = make_relation(a.node_id(), b.node_id());
        let r2 = make_relation(a.node_id(), b.node_id());
        let r3 = make_relation(b.node_id(), a.node_id());

        let router = CurvedArrowPlacer::new();
        let out = router.place(&[&r1, &r2, &r3], &a, &b);
        assert_eq!(out.len(), 3);

        // Self-loop bucket also returns one arrow per relation.
        let r4 = make_relation(a.node_id(), a.node_id());
        let r5 = make_relation(a.node_id(), a.node_id());
        let out_self = router.place(&[&r4, &r5], &a, &a);
        assert_eq!(out_self.len(), 2);
    }
}
