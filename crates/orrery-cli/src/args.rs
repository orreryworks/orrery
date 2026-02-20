//! Command-line argument definitions for the Orrery CLI.
//!
//! This module defines the [`Args`] structure parsed from the command line
//! using [`clap`]. Arguments control input/output paths, configuration file
//! selection, and logging verbosity.

use clap::Parser;

/// Command-line arguments for the Orrery diagram tool
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
pub struct Args {
    /// Path to the input Orrery file
    #[arg(help = "Path to the input file")]
    pub input: String,

    /// Path to the output SVG file
    #[arg(short, long, default_value = "out.svg")]
    pub output: String,

    /// Path to configuration file (TOML)
    #[arg(short, long)]
    pub config: Option<String>,

    /// Log level (off, error, warn, info, debug, trace)
    #[arg(long, default_value = "info")]
    pub log_level: String,
}
