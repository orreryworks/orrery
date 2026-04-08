//! CLI logic for the Orrery diagram tool.
//!
//! Wires together configuration loading, the [`DiagramBuilder`] pipeline,
//! and file I/O to turn a `.orr` source file into an SVG on disk.

mod args;
mod config;
mod error;
mod source_provider;

pub use args::Args;
pub use error::Error;

use std::{fs, path::Path};

use bumpalo::Bump;
use log::info;

use orrery::DiagramBuilder;

use source_provider::FsSourceProvider;

/// Runs the Orrery CLI application.
///
/// Loads configuration, parses the input `.orr` file, renders the
/// resulting diagram to SVG, and writes it to the output path.
///
/// # Arguments
///
/// * `args` - Command-line arguments.
/// * `arena` - Bump arena for source text allocation. Must outlive the
///   returned error, since [`Error::Parse`] borrows from it.
///
/// # Errors
///
/// Returns [`Error::Parse`] for syntax/validation errors with rich
/// diagnostics, or [`Error::Render`] for I/O, layout, or export errors.
pub fn run<'a>(args: &Args, arena: &'a Bump) -> Result<(), Error<'a>> {
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
    let diagram = builder.parse(arena, root_path)?;
    let svg = builder.render_svg(&diagram)?;

    // Write output file
    fs::write(&args.output, svg)?;

    info!(output_file = args.output; "SVG exported successfully");

    Ok(())
}
