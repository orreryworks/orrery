mod ast;
mod color;
mod error;
mod export;
mod graph;
mod layout;
mod shape;

use ast::{elaborate, parser};
use error::FilamentError;
use std::fs;

use clap::Parser;
use log::{debug, info, trace};

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
pub struct Config {
    /// Log level (off, error, warn, info, debug, trace)
    #[arg(long, default_value = "info")]
    pub log_level: String,

    /// Path to the input file
    #[arg(help = "Path to the input file")]
    pub file: String,

    /// Path to the output SVG file
    #[arg(short, long, default_value = "out.svg")]
    pub output: String,
}

pub fn run(cfg: &Config) -> Result<(), FilamentError> {
    info!(
        "Processing diagram with input: {}, output: {}",
        cfg.file, cfg.output
    );

    // Reading input file
    let content = fs::read_to_string(&cfg.file)?;
    trace!("File content: {}", content);

    // Parsing the diagram
    info!("Parsing diagram");
    let ast = parser::build_diagram(&content)?;
    debug!("Parsed AST successfully");

    // Elaborating the AST
    info!("Elaborating AST");
    let elaborate_builder = elaborate::Builder::new();
    let elaborated_ast = elaborate_builder.build(&ast)?;
    debug!("Elaborated AST successfully");

    // Process diagram based on its type
    // Build the diagram graph (common for all types)
    info!("Building {:?} diagram graph", elaborated_ast.kind);
    let graph = graph::diagram_to_graph(&elaborated_ast)?;
    debug!(
        "Graph built successfully with {} nodes and {} edges",
        graph.node_count(),
        graph.edge_count()
    );

    // Create SVG exporter that will use diagram dimensions
    let svg_exporter = export::svg::Svg::new(&cfg.output);

    // Process diagram based on its type
    match elaborated_ast.kind {
        elaborate::DiagramKind::Component => {
            // Calculating component layout
            info!("Calculating component layout");
            let layout_engine = layout::component::Engine::new();
            let layout = layout_engine.calculate(&graph);
            debug!(
                "Layout calculated with {} components and {} relations",
                layout.components.len(),
                layout.relations.len()
            );

            // Export the component layout
            info!("Exporting component diagram to SVG");
            svg_exporter.export_component_layout(&layout)?;
        }
        elaborate::DiagramKind::Sequence => {
            // Calculating sequence layout
            info!("Calculating sequence layout");
            let layout_engine = layout::sequence::Engine::new();
            let layout = layout_engine.calculate(&graph);
            debug!(
                "Layout calculated with {} participants and {} messages",
                layout.participants.len(),
                layout.messages.len()
            );

            // Export the sequence layout
            info!("Exporting sequence diagram to SVG");
            svg_exporter.export_sequence_layout(&layout)?;
        }
    }

    // Common post-processing
    info!("SVG exported successfully to: {}", cfg.output);

    // Debug output for development purposes
    trace!(target: "ast", elaborated_ast:?; "Elaborated AST");

    Ok(())
}
