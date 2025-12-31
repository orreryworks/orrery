use std::{process, str::FromStr};

use clap::Parser;
use log::{LevelFilter, debug, error, info};

use filament::FilamentError;
use filament_cli::Args;

fn main() {
    // Install miette's pretty panic hook early for better panic reports
    miette::set_panic_hook();

    // Parse configuration first
    let args = Args::parse();

    // Initialize the logger with the specified log level
    let log_level = LevelFilter::from_str(&args.log_level).unwrap_or_else(|_| {
        eprintln!(
            "Invalid log level: {}. Using 'warn' instead.",
            args.log_level
        );
        LevelFilter::Warn
    });

    env_logger::Builder::from_env(env_logger::Env::default())
        .filter_level(log_level)
        .init();

    info!(log_level:?; "Starting Filament");
    debug!(args:?; "Parsed arguments");

    // Run the application
    if let Err(err) = filament_cli::run(&args) {
        // Use miette to display a rich diagnostic error
        let reporter = miette::GraphicalReportHandler::new();
        let mut writer = String::new();
        match err {
            FilamentError::LexerDiagnostic { .. }
            | FilamentError::ParseDiagnostic { .. }
            | FilamentError::ElaborationDiagnostic { .. }
            | FilamentError::ValidationDiagnostic { .. } => {
                // Pass the FilamentError itself which implements Diagnostic and has source_code
                reporter.render_report(&mut writer, &err).unwrap();
            }
            err => {
                writer.push_str(&err.to_string());
            }
        }
        error!("Failed\n{writer}");
        process::exit(1);
    }

    info!("Completed successfully");
}
