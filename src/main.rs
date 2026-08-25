mod config;
mod utils;

use std::{path::PathBuf, process::ExitCode};

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

    let config_path = if let Ok(path) = utils::does_file_exist(&args.config) {
        path
    } else {
        warn!("Config file {} does not exist, creating now...", &args.config);
        if let Err(err) = std::fs::write(&args.config, "{}") {
            error!("Failed to create config file: {}", err);
            return ExitCode::FAILURE;
        }
        info!("Config file created successfully");
        PathBuf::from(&args.config)
    };

    let config = match config::Config::from_file(&config_path) {
        Ok(config) => config,
        Err(err) => {
            error!("{err:#}");
            return ExitCode::FAILURE;
        }
    };

    if config.paths.is_empty() {
        info!("There are no paths loaded so nothing will be restored or backed up");
        return ExitCode::SUCCESS;
    }

    info!("Loaded {} path(s) from {}", config.paths.len(), config_path.display());

    ExitCode::SUCCESS
}
