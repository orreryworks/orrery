//! Example: Creating a diagram from the semantic model
//!
//! This example demonstrates how to programmatically build a diagram
//! using the semantic model types directly, without parsing source code.

use std::rc::Rc;

use orrery::{
    DiagramBuilder,
    draw::{
        ArrowDefinition, ArrowDirection, RectangleDefinition, ShapeDefinition, StrokeDefinition,
    },
    identifier::Id,
    semantic::{Block, Diagram, DiagramKind, Element, LayoutEngine, Node, Relation, Scope},
};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("Building diagram from semantic model...\n");

    // Create shape definitions for our nodes
    let rectangle_def: Rc<Box<dyn ShapeDefinition>> = Rc::new(Box::new(RectangleDefinition::new()));

    // Create arrow definition for relations
    let stroke_def = Rc::new(StrokeDefinition::default());
    let arrow_def = Rc::new(ArrowDefinition::new(stroke_def));

    // Create node identifiers (Id is Copy, so we can reuse them)
    let client_id = Id::new("client");
    let server_id = Id::new("server");
    let database_id = Id::new("database");

    // Create nodes (components in the diagram)
    let client_node = Node::new(
        client_id,
        "client".to_string(),
        Some("Web Client".to_string()), // display name
        Block::None,
        Rc::clone(&rectangle_def),
    );

    let server_node = Node::new(
        server_id,
        "server".to_string(),
        Some("API Server".to_string()),
        Block::None,
        Rc::clone(&rectangle_def),
    );

    let database_node = Node::new(
        database_id,
        "database".to_string(),
        Some("Database".to_string()),
        Block::None,
        Rc::clone(&rectangle_def),
    );

    // Create relations (connections between nodes)
    let client_to_server = Relation::new(
        client_id,
        server_id,
        ArrowDirection::Forward,
        Some("HTTP requests".to_string()),
        Rc::clone(&arrow_def),
    );

    let server_to_database = Relation::new(
        server_id,
        database_id,
        ArrowDirection::Forward,
        Some("SQL queries".to_string()),
        Rc::clone(&arrow_def),
    );

    // Build the list of elements
    let elements = vec![
        Element::Node(client_node),
        Element::Node(server_node),
        Element::Node(database_node),
        Element::Relation(client_to_server),
        Element::Relation(server_to_database),
    ];

    // Create the scope containing all elements
    let scope = Scope::new(elements);

    // Create the diagram
    let diagram = Diagram::new(
        DiagramKind::Component,
        scope,
        LayoutEngine::Basic,
        None, // no background color
        None, // no lifeline definition (not a sequence diagram)
    );

    // Print diagram info
    println!("Created diagram:");
    println!("  Kind: {:?}", diagram.kind());
    println!("  Layout engine: {:?}", diagram.layout_engine());
    println!("  Elements: {}", diagram.scope().elements().len());
    println!();

    // Render the diagram to SVG using DiagramBuilder
    println!("Rendering to SVG...");
    let builder = DiagramBuilder::default();
    let svg = builder.render_svg(&diagram)?;

    // Output SVG statistics
    println!("SVG generated successfully!");
    println!("SVG length: {} bytes", svg.len());

    // Write to file
    let output_path = "from_semantic_output.svg";
    std::fs::write(output_path, &svg)?;
    println!("SVG written to: {}", output_path);

    Ok(())
}
