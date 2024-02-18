use anyhow::Result;
use clap::Parser;
use modules::paper::download_handler;
use regex::Regex;
use reqwest::blocking::Client;
use serde::Deserialize;
use serde_json::Value;
use std::{
    error::Error,
    fs::{self},
    path::Path,
};
use tracing::{error, info, warn, Level};
use tracing_subscriber::FmtSubscriber;

mod modules;

#[derive(Parser)]
#[command(author, about)]
struct Args {
    /// The game version to download (e.g. 1.20)
    #[arg(short, long, conflicts_with = "latest")]
    version: Option<String>,

    /// Download the absolute latest version
    #[arg(short, long, conflicts_with = "version")]
    latest: bool,
}

fn main() {
    // Set up logging
    setup_logging();

    // Parse command-line arguments
    let args: Args = Args::parse();

    // Create an HTTP client instance
    let client = Client::new();

    // Extract local version information or use default
    let local_information = get_local_version_information_or_default();

    // Determine action based on arguments
    if let Some(version) = args.version {
        info!("Downloading requested version: {}", version);
        if let Err(e) = download_specific_version(&client, &local_information, &version) {
            error!("Failed to download specific version: {}", e);
        }
    } else if args.latest || local_information.version.is_empty() {
        info!("Downloading the latest version.");
        if let Err(e) = download_latest_version(&client, &local_information) {
            error!("Failed to download latest version: {}", e);
        }
    } else {
        info!("Downloading the latest build of the current version.");
        if let Err(e) = download_specific_version(&client, &local_information, local_information.version.as_str()) {
            error!("Failed to download latest build: {}", e);
        }
    }
}

fn setup_logging() {
    let subscriber = FmtSubscriber::builder()
        .with_max_level(Level::INFO)
        .finish();
    tracing::subscriber::set_global_default(subscriber).expect("Setting default subscriber failed");
}

fn get_local_version_information_or_default() -> VersionInformation {
    get_local_version_information().unwrap_or_else(|e| {
        warn!(
            "Failed to extract local version and build information, using defaults: {}",
            e
        );
        VersionInformation {
            server_type: "Paper".to_owned(),
            ..VersionInformation::default()
        }
    })
}

fn download_specific_version(
    client: &Client,
    local_information: &VersionInformation,
    version: &str,
) -> Result<(), Box<dyn Error>> {
    let build: u16 = modules::paper::get_build(client, version).unwrap();

    let remote_information = VersionInformation {
        server_type: "Paper".to_string(),
        version: version.to_string(),
        build,
    };

    if compare_versions(local_information, &remote_information) {
        info!("Latest version already downloaded");
        return Ok(());
    }

    download_handler(client, &version.to_string(), &build)?;

    Ok(())
}

fn download_latest_version(
    client: &Client,
    local_information: &VersionInformation,
) -> Result<(), Box<dyn Error>> {
    let version: String = modules::paper::get_latest_version(client)?;

    let build: u16 = modules::paper::get_build(client, &version)?;

    let remote_information = VersionInformation {
        server_type: "Paper".to_string(),
        version: version.clone(),
        build,
    };

    if compare_versions(local_information, &remote_information) {
        info!("Latest version already downloaded");
        return Ok(());
    }

    download_handler(client, &version, &build)?;

    Ok(())
}

fn compare_versions(
    local_version: &VersionInformation,
    remote_version: &VersionInformation,
) -> bool {
    if local_version.server_type != remote_version.server_type {
        return false;
    }

    if local_version.version != remote_version.version {
        return false;
    }

    if local_version.build != remote_version.build {
        return false;
    }

    true
}

fn get_local_version_information() -> Result<VersionInformation, Box<dyn Error>> {
    info!("Checking for existing local version information");

    if !Path::new("version_history.json").exists() {
        return Err("Failed to find version history".into());
    }

    info!("Reading version_history.json");
    let version_history_contents = fs::read_to_string("version_history.json")?;

    info!("Parsing version_history.json as JSON");
    let version_history_json: Value = serde_json::from_str(&version_history_contents)?;

    let current_version = version_history_json["currentVersion"]
        .as_str()
        .ok_or_else(|| "Failed to parse current version")?;

    let re = Regex::new(r"git-(\w+)-(\d+) \(MC: ([\d.]+)\)")?;
    let caps = re
        .captures(current_version)
        .ok_or_else(|| "Failed to match version pattern")?;

    let server_type = caps
        .get(1)
        .ok_or_else(|| "Failed to extract server type")?
        .as_str()
        .to_string();
    let build = caps
        .get(2)
        .ok_or_else(|| "Failed to extract build")?
        .as_str()
        .parse::<u16>()?;
    let version = caps
        .get(3)
        .ok_or_else(|| "Failed to extract version")?
        .as_str()
        .to_string();

    info!(
        "Information found: [{}, {}, {}]",
        server_type, version, build
    );

    Ok(VersionInformation {
        server_type,
        version,
        build,
    })
}

// Standard struct
#[derive(Deserialize, Debug, Default)]
pub struct VersionInformation {
    server_type: String,
    version: String,
    build: u16,
}
