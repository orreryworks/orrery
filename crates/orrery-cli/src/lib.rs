//! CLI logic for the Orrery diagram tool.
//!
//! Wires together configuration loading, the [`DiagramBuilder`] pipeline,
//! and file I/O to turn a `.orr` source file into an SVG on disk.

pub mod error_adapter;

mod args;
mod config;
mod source_provider;

pub use args::Args;

use std::{fs, path::Path};

use log::info;

use orrery::{DiagramBuilder, OrreryError};

use source_provider::FsSourceProvider;

/// Runs the Orrery CLI application.
///
/// Loads configuration, parses the input `.orr` file, renders the
/// resulting diagram to SVG, and writes it to the output path.
///
/// # Arguments
///
/// * `args` - Command-line arguments.
///
/// # Errors
///
/// Returns [`OrreryError`] if:
/// - Configuration cannot be loaded
/// - The input file cannot be found or parsed
/// - Layout or rendering fails
/// - The output file cannot be written
pub fn run(args: &Args) -> Result<(), OrreryError> {
    info!(
        input_path = args.input,
        output_path = args.output;
        "Processing diagram"
    );

    // Load configuration
    let app_config = config::load_config(args.config.as_ref())?;

    // Process diagram using DiagramBuilder API
    let root_path = Path::new(&args.input);
    let provider = FsSourceProvider::new();
    let builder = DiagramBuilder::new(app_config, &provider);
    let diagram = builder.parse(root_path)?;
    let svg = builder.render_svg(&diagram)?;

    // Write output file
    fs::write(&args.output, svg)?;

    info!(output_file = args.output; "SVG exported successfully");

    Ok(())
}
