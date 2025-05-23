mod ast;
mod color;
mod config;
mod error;
mod export;
mod graph;
mod layout;
mod shape;

use clap::Parser;
use config::AppConfig;
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

    /// Path to configuration file (TOML)
    #[arg(short, long)]
    pub config: Option<String>,
}

pub fn run(cfg: &Config) -> Result<(), FilamentError> {
    info!(
        input_path = cfg.file,
        output_path = cfg.output;
        "Processing diagram",
    );

    let app_config = AppConfig::find_and_load(cfg.config.as_ref())?;

    // Reading input file
    let content = fs::read_to_string(&cfg.file)?;
    trace!(content; "File content");

    // Process the diagram through parsing and elaboration
    info!("Building diagram AST");
    let elaborated_ast = ast::build_ast(&app_config, &content)?;
    debug!("AST built successfully");
    trace!(elaborated_ast:?; "Elaborated AST");

    // Process diagram based on its type
    // Build the diagram graph (common for all types)
    info!(diagram_kind:? = elaborated_ast.kind; "Building diagram graph");
    let graphs = graph::Collection::from_diagram(&elaborated_ast)?;
    debug!("Graph built successfully");

    // Create SVG exporter builder with diagram properties
    let svg_exporter = export::svg::SvgBuilder::new(&cfg.output)
        .with_style(&app_config.style)
        .with_diagram(&elaborated_ast)
        .build()?;

    // Create a configured engine builder for processing diagrams
    let engine_builder = layout::EngineBuilder::new()
        .with_component_padding(40.0)
        .with_component_spacing(50.0)
        .with_message_spacing(60.0)
        .with_force_iterations(500);

    // Process all diagrams in the hierarchy, from innermost to outermost
    // Each embedded diagram uses its own layout engine as specified in its attributes
    info!("Processing diagrams in hierarchy");
    let (main_layout, _embedded_layouts) = engine_builder.build(&graphs); // FIXME: This is a temp and should use _embedded_loyouts.

    // Process diagram based on its type
    match main_layout {
        layout::engines::LayoutResult::Component(layout) => {
            debug!(
                components_len = layout.components.len(),
                relations_len = layout.relations.len();
                "Layout calculated",
            );

            // Export the component layout
            info!("Exporting component diagram to SVG");
            svg_exporter.export_component_layout(&layout)?;
        }
        layout::engines::LayoutResult::Sequence(layout) => {
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
