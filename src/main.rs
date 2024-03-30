use anyhow::{Context, Result};
use clap::Parser;
use reqwest::blocking::Client;
use serde_json;
use tracing::{info, warn, Level};
use tracing_subscriber::FmtSubscriber;

mod modules;
use modules::paper::{
    download_handler, get_build, get_latest_version, get_local_version_information,
};

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
            info!("Checking for the latest version...");
            get_latest_version(&client).context("Failed to get the latest version")?
        }
    };

    info!("Checking local version information...");
    let local_information = match get_local_version_information() {
        Ok(info) => info,
        Err(e) => {
            warn!("Failed to get local version information: {}", e);
            serde_json::Value::default()
        }
    };

    if let Some(current_version) = local_information.get("currentVersion") {
        if let Some(current_version_str) = current_version.as_str() {
            if let Some((local_mc_version, local_build)) = parse_version(current_version_str) {
                if local_mc_version == version {
                    let remote_build = get_build(&client, &version)?;
                    if local_build >= remote_build {
                        info!(
                            "Server is up to date (version {}, build {}).",
                            version, local_build
                        );
                        return Ok(());
                    } else {
                        info!(
                            "Update available for version {}. Local: {}, Remote: {}",
                            version, local_build, remote_build
                        );
                    }
                } else {
                    info!(
                        "New Minecraft version. Local: {}, Remote: {}",
                        local_mc_version, version
                    );
                }
            }
        }
    } else {
        info!("No existing version found.");
    }

    info!("Downloading version: {}", version);
    download_handler(&client, &version)
        .context(format!("Failed to download version {}", version))?;

    info!("Server updated to version: {}", version);
    Ok(())
}

fn setup_logging() {
    let subscriber = FmtSubscriber::builder()
        .with_max_level(Level::INFO)
        .finish();
    tracing::subscriber::set_global_default(subscriber).expect("Setting default subscriber failed");
}

fn parse_version(version: &str) -> Option<(&str, u16)> {
    let parts: Vec<&str> = version.split(" ").collect();
    if parts.len() == 3 {
        let build = parts[0].split("-").last()?.parse().ok()?;
        let mc_version = parts[2].trim_end_matches(')');
        Some((mc_version, build))
    } else {
        None
    }
}
