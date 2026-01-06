//! Example: Creating a diagram from Filament source code
//!
//! This example demonstrates the basic workflow of:
//! 1. Creating a DiagramBuilder with default configuration
//! 2. Parsing Filament source code into a semantic diagram
//! 3. Rendering the semantic diagram to SVG

use filament::DiagramBuilder;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Define Filament source code for a component diagram
    let source = r#"
        diagram component;

        // Define custom types
        type Database = Rectangle [fill_color="lightblue"];
        type Service = Rectangle [fill_color="lightyellow"];

        // Create components
        frontend as "Frontend App": Service;
        backend as "Backend API": Service;
        database as "PostgreSQL": Database;

        // Define relationships
        frontend -> backend: "REST API";
        backend -> database: "SQL queries";
    "#;

    // Create a builder with default configuration
    let builder = DiagramBuilder::default();

    // Parse the source code into a semantic diagram
    println!("Parsing diagram from source...");
    let diagram = builder.parse(source)?;

    // Inspect the parsed diagram
    println!("Diagram kind: {:?}", diagram.kind());
    println!("Layout engine: {:?}", diagram.layout_engine());
    println!("Number of elements: {}", diagram.scope().elements().len());

    // Render the semantic diagram to SVG
    println!("\nRendering to SVG...");
    let svg = builder.render_svg(&diagram)?;

    // Output SVG statistics
    println!("SVG generated successfully!");
    println!("SVG length: {} bytes", svg.len());

    // Optionally write to file
    let output_path = "from_source_output.svg";
    std::fs::write(output_path, &svg)?;
    println!("SVG written to: {}", output_path);

    Ok(())
}
