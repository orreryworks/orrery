//! Sequence diagram graph structures and event management.
//!
//! This module provides the graph representation for sequence diagrams, which emphasizes
//! temporal ordering and message flow between participants. The key abstractions are:
//!
//! - [`SequenceGraph`]: The main graph structure that stores participants (nodes) and ordered events
//! - [`SequenceEvent`]: Enumeration of different event types
//!
//! # Architecture
//!
//! Sequence diagrams maintain a strict temporal ordering of events. Unlike component diagrams
//! which focus on spatial relationships, sequence diagrams preserve the exact order of
//! interactions as they appear in the source.
//!
//! The graph stores participants as nodes and maintains a separate ordered list of events
//! that represents the timeline of interactions.

use indexmap::IndexMap;
use log::debug;

use crate::{FilamentError, ast, identifier::Id};

/// Represents ordered events in a sequence diagram.
///
/// Events capture the temporal flow of a sequence diagram, maintaining the exact
/// order of interactions as specified in the source. This ordering is essential
/// for correct visualization of message sequences and activation periods.
///
/// # Variants
///
/// * [`SequenceEvent::Relation`] - A message or relation between two participants
/// * [`SequenceEvent::Activate`] - Start of an activation period for a participant
/// * [`SequenceEvent::Deactivate`] - End of an activation period for a participant
/// * [`SequenceEvent::FragmentStart`] - Start of a fragment block
/// * [`SequenceEvent::FragmentSectionStart`] - Start of a section within a fragment
/// * [`SequenceEvent::FragmentSectionEnd`] - End of a section within a fragment
/// * [`SequenceEvent::FragmentEnd`] - End of a fragment block
#[derive(Debug)]
pub enum SequenceEvent<'a> {
    /// A message or relation between two participants.
    ///
    /// Contains a reference to the AST relation which includes source, target,
    /// and any label or styling information.
    Relation(&'a ast::Relation),

    /// Start of an activation period for a participant.
    ///
    /// Activation represents a period when a participant has focus of control,
    /// typically shown as a white rectangle on the participant's lifeline.
    /// Contains the [`Id`] of the participant that becomes active.
    Activate(Id),

    /// End of an activation period for a participant.
    ///
    /// Marks the end of a focus of control period for a participant.
    /// Contains the [`Id`] of the participant that becomes inactive.
    Deactivate(Id),

    /// Start of a fragment block.
    ///
    /// Fragments group related interactions with an operation type (e.g., "alt", "loop").
    /// This event marks the beginning of a fragment's scope.
    FragmentStart(&'a ast::Fragment),

    /// Start of a section within a fragment.
    ///
    /// Sections divide a fragment into parts (e.g., different cases in an "alt" fragment).
    /// Each section may have an optional title describing its condition or purpose.
    FragmentSectionStart(&'a ast::FragmentSection),

    /// End of a section within a fragment.
    ///
    /// Marks the boundary where one section ends before another begins or the fragment ends.
    FragmentSectionEnd,

    /// End of a fragment block.
    ///
    /// Marks the end of a fragment's scope, closing the grouping of interactions.
    FragmentEnd,

    /// A note annotation in the diagram.
    ///
    /// Notes provide additional context or documentation without participating
    /// in the diagram's structural relationships.
    Note(&'a ast::Note),
}

/// Main graph structure for sequence diagrams.
///
/// This structure maintains the complete representation of a sequence diagram,
/// including all participants (as nodes) and the ordered sequence of events.
/// The graph preserves the exact ordering of events as they appear in the AST,
/// which is critical for correct temporal visualization in sequence diagrams.
#[derive(Debug)]
pub struct SequenceGraph<'a> {
    nodes: IndexMap<Id, &'a ast::Node>,
    events: Vec<SequenceEvent<'a>>,
}

impl<'a> SequenceGraph<'a> {
    /// Returns an iterator over all events in temporal order.
    ///
    /// This method provides access to events in the exact order they appear in the AST,
    /// which represents the temporal flow of the sequence.
    ///
    /// # Returns
    /// An iterator yielding [`SequenceEvent`] items in temporal order.
    pub fn events(&self) -> impl Iterator<Item = &SequenceEvent<'a>> {
        self.events.iter()
    }

    /// Returns an iterator over just relation events in temporal order.
    ///
    /// This method filters the ordered events to return only the relation (message) events,
    /// maintaining their temporal ordering. This is useful for rendering just the messages
    /// without activation/deactivation events.
    ///
    /// # Returns
    /// An iterator yielding [`ast::Relation`] items for message events only, in temporal order.
    pub fn relations(&self) -> impl Iterator<Item = &ast::Relation> {
        self.events().filter_map(|event| match event {
            SequenceEvent::Relation(relation) => Some(*relation),
            _ => None,
        })
    }

    /// Returns an iterator over all participant nodes in the sequence diagram.
    pub fn nodes(&self) -> impl Iterator<Item = &ast::Node> {
        self.nodes.values().cloned()
    }

    /// Returns an iterator over all participant IDs in the sequence diagram.
    pub fn node_ids(&self) -> impl Iterator<Item = &Id> {
        self.nodes.keys()
    }

    /// Creates a sequence graph from AST elements.
    ///
    /// Processes the elements to build the graph structure, adding participants as nodes
    /// and maintaining the temporal ordering of events. Also identifies any embedded
    /// diagrams within participant nodes that need separate processing.
    ///
    /// # Returns
    /// A tuple containing the constructed sequence graph and a vector of any embedded
    /// diagrams found during processing.
    pub(super) fn new_from_elements<'idx>(
        elements: &'a [ast::Element],
    ) -> Result<(Self, Vec<super::HierarchyNode<'a, 'idx>>), FilamentError> {
        let mut graph = Self::new();

        let child_diagrams = Self::process_elements(elements, &mut graph)?;

        Ok((graph, child_diagrams))
    }

    /// Creates a new empty sequence graph.
    fn new() -> Self {
        Self {
            nodes: IndexMap::new(),
            events: Vec::new(),
        }
    }

    /// Adds a participant node to the sequence graph.
    ///
    /// Registers the participant in the graph's node map, making it available
    /// for use in relations and activation events.
    fn add_node(&mut self, node: &'a ast::Node) {
        self.nodes.insert(node.id(), node);
    }

    /// Adds an event to the sequence graph's timeline.
    ///
    /// Events are added in the order they appear in the AST, preserving the
    /// temporal sequence of interactions in the diagram.
    fn add_event(&mut self, event: SequenceEvent<'a>) {
        self.events.push(event);
    }

    /// Process elements and add them to the graph.
    ///
    /// This helper method processes elements, adding events to the graph and returning
    /// any child diagrams found.
    fn process_elements<'idx>(
        elements: &'a [ast::Element],
        graph: &mut SequenceGraph<'a>,
    ) -> Result<Vec<super::HierarchyNode<'a, 'idx>>, FilamentError> {
        let mut child_diagrams = Vec::new();
        for element in elements {
            match element {
                ast::Element::Node(node) => {
                    graph.add_node(node);

                    // Process the node's inner block recursively
                    match node.block() {
                        ast::Block::Diagram(inner_diagram) => {
                            debug!(
                                "Processing nested diagram of kind {:?}",
                                inner_diagram.kind()
                            );
                            let inner_hierarchy_child =
                                super::HierarchyNode::build_from_ast_diagram(
                                    inner_diagram,
                                    Some(node.id()),
                                )?;
                            child_diagrams.push(inner_hierarchy_child);
                        }
                        ast::Block::None => {}
                        ast::Block::Scope(..) => {
                            unreachable!("Unexpected scope block in sequence diagram")
                        }
                    }
                }
                ast::Element::Relation(relation) => {
                    graph.add_event(SequenceEvent::Relation(relation));
                }
                ast::Element::Activate(id) => {
                    graph.add_event(SequenceEvent::Activate(*id));
                }
                ast::Element::Deactivate(id) => {
                    graph.add_event(SequenceEvent::Deactivate(*id));
                }
                ast::Element::Fragment(fragment) => {
                    // Emit FragmentStart event
                    graph.add_event(SequenceEvent::FragmentStart(fragment));

                    // Process each section
                    for section in fragment.sections() {
                        // Emit SectionStart event
                        graph.add_event(SequenceEvent::FragmentSectionStart(section));

                        // Recursively process elements within the section
                        let mut section_child_diagrams =
                            Self::process_elements(section.elements(), graph)?;
                        child_diagrams.append(&mut section_child_diagrams);

                        // Emit SectionEnd event
                        graph.add_event(SequenceEvent::FragmentSectionEnd);
                    }

                    // Emit FragmentEnd event
                    graph.add_event(SequenceEvent::FragmentEnd);
                }
                ast::Element::Note(note) => {
                    graph.add_event(SequenceEvent::Note(note));
                }
            }
        }

        Ok(child_diagrams)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        ast::{Block, Node, TypeDefinition},
        draw,
        identifier::Id,
    };
    use std::rc::Rc;

    /// Helper function to create a minimal mock node for testing
    fn create_test_node(id_str: &str) -> Node {
        let id = Id::new(id_str);
        let shape_def = Box::new(draw::RectangleDefinition::new());
        let type_def = TypeDefinition::new_shape(id, shape_def);

        Node::new(id, id_str.to_string(), None, Block::None, Rc::new(type_def))
    }

    #[test]
    fn test_participant_ordering_preserved() {
        // Create nodes in a specific NON-ALPHABETICAL order to catch sorting bugs
        // Expected order: Zara, Alice, Mike, Bob, Eve
        let zara = create_test_node("Zara");
        let alice = create_test_node("Alice");
        let mike = create_test_node("Mike");
        let bob = create_test_node("Bob");
        let eve = create_test_node("Eve");

        // Create a sequence graph and add nodes in order
        let mut graph = SequenceGraph::new();
        graph.add_node(&zara);
        graph.add_node(&alice);
        graph.add_node(&mike);
        graph.add_node(&bob);
        graph.add_node(&eve);

        // Collect node IDs from the graph
        let node_ids: Vec<Id> = graph.node_ids().copied().collect();

        // Verify that the order is preserved
        assert_eq!(node_ids.len(), 5, "Should have 5 participants");
        assert_eq!(
            node_ids[0],
            Id::new("Zara"),
            "First participant should be Zara"
        );
        assert_eq!(
            node_ids[1],
            Id::new("Alice"),
            "Second participant should be Alice"
        );
        assert_eq!(
            node_ids[2],
            Id::new("Mike"),
            "Third participant should be Mike"
        );
        assert_eq!(
            node_ids[3],
            Id::new("Bob"),
            "Fourth participant should be Bob"
        );
        assert_eq!(
            node_ids[4],
            Id::new("Eve"),
            "Fifth participant should be Eve"
        );

        // Also verify nodes() iterator preserves order
        let node_labels: Vec<String> = graph.nodes().map(|n| n.id().to_string()).collect();

        assert_eq!(
            node_labels[0], "Zara",
            "nodes() iterator: first should be Zara"
        );
        assert_eq!(
            node_labels[1], "Alice",
            "nodes() iterator: second should be Alice"
        );
        assert_eq!(
            node_labels[2], "Mike",
            "nodes() iterator: third should be Mike"
        );
        assert_eq!(
            node_labels[3], "Bob",
            "nodes() iterator: fourth should be Bob"
        );
        assert_eq!(
            node_labels[4], "Eve",
            "nodes() iterator: fifth should be Eve"
        );
    }

    #[test]
    fn test_participant_ordering_with_duplicates() {
        // Test that re-adding a node doesn't change its position
        // Use non-alphabetical order: Zara, Bob, Alice
        let zara = create_test_node("Zara");
        let bob = create_test_node("Bob");
        let alice = create_test_node("Alice");

        let mut graph = SequenceGraph::new();
        graph.add_node(&zara);
        graph.add_node(&bob);
        graph.add_node(&alice);

        // Re-add Bob (should update, not move)
        graph.add_node(&bob);

        let node_ids: Vec<Id> = graph.node_ids().copied().collect();

        assert_eq!(node_ids.len(), 3, "Should still have 3 participants");
        assert_eq!(node_ids[0], "Zara", "Zara should remain first");
        assert_eq!(node_ids[1], "Bob", "Bob should remain second");
        assert_eq!(node_ids[2], "Alice", "Alice should remain third");
    }
}
