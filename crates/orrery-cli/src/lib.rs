//! CLI logic for the Orrery diagram tool.
//!
//! This module contains the core CLI logic for the Orrery diagram tool.

pub mod error_adapter;

mod args;
mod config;

pub use args::Args;

use std::fs;

use log::info;

use orrery::{DiagramBuilder, OrreryError};

/// Run the Orrery CLI application
///
/// This function processes the input file through the Orrery pipeline
/// and writes the resulting SVG to the output file.
///
/// # Arguments
///
/// * `args` - Command-line arguments
///
/// # Errors
///
/// Returns `OrreryError` for:
/// - File I/O errors
/// - Configuration loading errors
/// - Parsing errors
/// - Layout errors
/// - Rendering errors
pub fn run(args: &Args) -> Result<(), OrreryError> {
    info!(
        input_path = args.input,
        output_path = args.output;
        "Processing diagram"
    );

    // Load configuration
    let app_config = config::load_config(args.config.as_ref())?;

    // Read input file
    let source = fs::read_to_string(&args.input)?;

    // Process diagram using DiagramBuilder API
    let builder = DiagramBuilder::new(app_config);
    let diagram = builder.parse(&source)?;
    let svg = builder.render_svg(&diagram)?;

    // Write output file
    fs::write(&args.output, svg)?;

    info!(output_file = args.output; "SVG exported successfully");

    Ok(())
}
