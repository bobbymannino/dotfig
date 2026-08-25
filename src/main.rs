mod config;
mod paths;
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

    let registry = match paths::Registry::load() {
        Ok(registry) => registry,
        Err(err) => {
            error!("{err:#}");
            return ExitCode::FAILURE;
        }
    };

    let mut resolved = Vec::with_capacity(config.paths.len());
    let mut invalid = 0_usize;

    for key in &config.paths {
        if let Some(known) = registry.get(key) {
            resolved.push(known);
        } else {
            warn!("Unknown path `{key}`, skipping");
            invalid += 1;
        }
    }

    if invalid > 0 {
        warn!("Skipped {invalid} unknown path(s), check them against paths.json");
    }

    info!("Resolved {} path(s) from {}", resolved.len(), config_path.display());

    for known in &resolved {
        info!("  {}:{} -> {}", known.group, known.title, known.path);
    }

    ExitCode::SUCCESS
}
