mod utils;

use std::process::ExitCode;

use clap::Parser;
use tracing::{error, info, warn};

/// A simple CLI tool for creating and restoring backups for your dot files.
#[derive(Parser, Debug)]
#[command(version, about)]
struct Args {
    #[clap(short, long, default_value = "dotfig.json")]
    config: String,
}

fn main() -> ExitCode {
    tracing_subscriber::fmt()
        .with_ansi(console::colors_enabled())
        .with_target(false)
        .init();

    let args = Args::parse();

    let config_file_exists = utils::does_file_exist(&args.config);
    if config_file_exists.is_err() {
        warn!("Config file {} does not exist, creating now...", &args.config);
        if let Err(err) = std::fs::write(&args.config, "{}") {
            error!("Failed to create config file: {}", err);
            return ExitCode::FAILURE;
        };
        info!("Config file created successfully.");
    };

    ExitCode::SUCCESS
}
