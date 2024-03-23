use anyhow::{Context, Result};
use clap::Parser;
use regex::Regex;
use reqwest::blocking::Client;
use serde_json::Value;
use tracing::{info, warn, Level};
use tracing_subscriber::FmtSubscriber;

mod modules;
use modules::paper::{download_handler, get_latest_version, get_local_version_information};

use crate::modules::paper::get_build;

#[derive(Parser)]
#[command(author, about)]
struct Args {
    /// The game version to download (e.g. 1.20)
    #[arg(short, long)]
    version: Option<String>,
}

fn main() -> Result<()> {
    setup_logging();

    let args: Args = Args::parse();
    let client = Client::new();

    let version = match args.version {
        Some(v) => v,
        None => {
            info!("No version specified, checking for the latest version...");
            get_latest_version(&client).context("Failed to get the latest version")?
        }
    };

    info!("Checking for existing local version information...");
    let local_information = get_local_version_information().unwrap_or_else(|e| {
        warn!("Failed to get local version information: {}", e);
        Value::default()
    });

    if let Some(current_version) = local_information.get("currentVersion") {
        if let Some(current_version_str) = current_version.as_str() {
            let re = Regex::new(r"git-(\w+)-(\d+) \(MC: ([\d.]+)\)")?;
            if let Some(caps) = re.captures(current_version_str) {
                let local_version = caps.get(3).map_or("", |m| m.as_str());
                let local_build = caps.get(2).map_or(0, |m| m.as_str().parse().unwrap_or(0));

                if local_version == version {
                    let remote_build = get_build(&client, &version)?;
                    if local_build >= remote_build {
                        info!("Latest version and build are already downloaded.");
                        return Ok(());
                    }
                }
            }
        }
    }

    info!("Downloading version: {}", version);
    download_handler(&client, &version).context(format!("Failed to download version {}", version))?;

    info!("Server is now up to date with version: {}", version);
    Ok(())
}

fn setup_logging() {
    let subscriber = FmtSubscriber::builder()
        .with_max_level(Level::INFO)
        .finish();
    tracing::subscriber::set_global_default(subscriber).expect("Setting default subscriber failed");
}