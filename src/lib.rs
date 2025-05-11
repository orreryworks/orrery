mod ast;
mod color;
mod error;
mod export;
mod graph;
mod layout;
mod shape;

use clap::Parser;
pub use error::FilamentError;
use export::Exporter;
use log::{debug, info, trace};
use std::fs;

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
        input_path = cfg.file,
        output_path = cfg.output;
        "Processing diagram",
    );

    // Reading input file
    let content = fs::read_to_string(&cfg.file)?;
    trace!(content; "File content");

    // Process the diagram through parsing and elaboration
    info!("Building diagram AST");
    let elaborated_ast = ast::build_ast(&content)?;
    debug!("AST built successfully");
    trace!(elaborated_ast:?; "Elaborated AST");

    // Process diagram based on its type
    // Build the diagram graph (common for all types)
    info!(diagram_kind:? = elaborated_ast.kind; "Building diagram graph");
    let graph = graph::Graph::from_diagram(&elaborated_ast)?;
    debug!(
        // nodes_count = graph.node_count(),
        // edges_count = graph.edge_count();
        "Graph built successfully",
    );

    // Create SVG exporter that will use diagram dimensions
    let svg_exporter = export::svg::Svg::new(&cfg.output);

    // Process diagram based on its type
    match elaborated_ast.kind {
        ast::DiagramKind::Component => {
            // Calculating component layout
            info!("Calculating component layout");
            let layout_engine = layout::create_component_engine(elaborated_ast.layout_engine)?;
            let layout = layout_engine.calculate(&graph);
            debug!(
                components_len = layout.components.len(),
                relations_len = layout.relations.len();
                "Layout calculated",
            );

            // Export the component layout
            info!("Exporting component diagram to SVG");
            svg_exporter.export_component_layout(&layout)?;
        }
        ast::DiagramKind::Sequence => {
            // Calculating sequence layout
            info!("Calculating sequence layout");
            let layout_engine = layout::create_sequence_engine(elaborated_ast.layout_engine)?;
            let layout = layout_engine.calculate(&graph);
            debug!(
                participants_len = layout.participants.len(),
                messages_len = layout.messages.len();
                "Layout calculated",
            );

            // Export the sequence layout
            info!("Exporting sequence diagram to SVG");
            svg_exporter.export_sequence_layout(&layout)?;
        }
    }

    // Common post-processing
    info!(output_file = cfg.output; "SVG exported successfully to");

    Ok(())
}
