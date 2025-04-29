use clap::Parser;
use filament::{Config, FilamentError};
use log::{error, info, LevelFilter};
use std::{process, str::FromStr};

fn main() {
    // Parse configuration first
    let cfg = Config::parse();

    // Initialize the logger with the specified log level
    let log_level = LevelFilter::from_str(&cfg.log_level).unwrap_or_else(|_| {
        eprintln!(
            "Invalid log level: {}. Using 'info' instead.",
            cfg.log_level
        );
        LevelFilter::Info
    });

    env_logger::Builder::from_env(env_logger::Env::default())
        .filter_level(log_level)
        .init();

    info!("Starting Filament with log level: {}", log_level);
    info!("Parsed configuration: {:?}", cfg);

    // Run the application
    if let Err(err) = filament::run(&cfg) {
        // Use miette to display a rich diagnostic error
        let reporter = miette::GraphicalReportHandler::new();
        let mut writer = String::new();
        match err {
            FilamentError::ParseDiagnostic(ref err) => {
                reporter.render_report(&mut writer, err).unwrap();
            }
            FilamentError::ElaborationDiagnostic(ref err) => {
                reporter.render_report(&mut writer, err).unwrap();
            }
            err => {
                writer.push_str(&err.to_string());
            }
        }
        error!("Failed\n{}", writer);
        process::exit(1);
    }

    info!("Completed successfully");
}
