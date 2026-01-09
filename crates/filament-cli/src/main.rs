use std::{process, str::FromStr};

use clap::Parser;
use log::{LevelFilter, debug, error, info};

use filament_cli::{Args, ErrorAdapter};

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
        // Wrap error in ErrorAdapter for rich miette formatting
        let adapted_error = ErrorAdapter(err);

        // Use miette to display the diagnostic error
        let reporter = miette::GraphicalReportHandler::new();
        let mut writer = String::new();
        reporter
            .render_report(&mut writer, &adapted_error)
            .expect("Writing to String buffer is infallible");

        error!("Failed\n{writer}");
        process::exit(1);
    }

    info!("Completed successfully");
}
