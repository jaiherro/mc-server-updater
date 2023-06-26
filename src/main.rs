use clap::Parser;
use reqwest::{Client, Response};
use serde::Deserialize;
use std::{
    error::Error,
    fs::{self, File},
    io::Write,
    path::Path,
};
use tracing::{error, info, subscriber, warn, Level};
use tracing_subscriber::FmtSubscriber;

mod variants;

#[derive(Parser)]
#[command(author, version, about)]
struct Args {
    /// The server variant to download (e.g. paper, purpur, etc.)
    #[arg(short, long, default_value = "paper")]
    variant: Option<String>,

    /// The game release to download (e.g. 1.20)
    #[arg(short, long, conflicts_with = "latest")]
    release: Option<String>,

    /// Download the absolute latest release
    #[arg(short, long, conflicts_with = "release")]
    latest: bool,
}

#[tokio::main]
async fn main() {
    // Get arguments
    let args: Args = Args::parse();

    // Create a client
    let client: Client = Client::new();

    // Create debugger subscriber
    let subscriber = FmtSubscriber::builder()
        .with_max_level(Level::INFO)
        .finish();
    subscriber::set_global_default(subscriber).expect("setting default subscriber failed");

    // Variables
    let file_name: &str = "server.jar";

    // Extract local version and build information from version_history.json
    let local_information: VersionBuild = match extract_local_information() {
        Ok(local_version_build) => local_version_build,
        Err(_) => {
            warn!("Failed to extract local version and build information");
            VersionBuild {
                variant: args.variant.as_deref().unwrap().to_string(),
                release: "0.0".to_string(),
                build: 0,
            }
        }
    };

    // If local information exists but is different to the requested variant, warn the user
    if local_information.variant != args.variant.as_deref().unwrap() {
        warn!("Local variant is different to requested variant");
    }

    // Match the arguments to determine the scripts logic
    match (args.variant, args.release, args.latest) {
        // If variant and release are provided, download the requested release
        (Some(variant), Some(release), _) => {
            info!("Downloading requested release");
            match download_release(&client, &variant, &release, file_name).await {
                Ok(_) => info!("Downloaded requested release"),
                Err(error) => error!("Failed to download requested release: {}", error),
            }
        }

        // If variant and latest are provided, download the latest release
        (Some(variant), _, true) => {
            info!("Downloading latest release");
            match download_latest_release(&client, &variant, file_name).await {
                Ok(_) => info!("Downloaded latest release"),
                Err(error) => error!("Failed to download latest release: {}", error),
            }
        }

        // If variant is provided but no release or latest, download the latest release
        (Some(variant), _, _) => {
            info!("Downloading latest release");
            match download_latest_release(&client, &variant, file_name).await {
                Ok(_) => info!("Downloaded latest release"),
                Err(error) => error!("Failed to download latest release: {}", error),
            }
        }

        // If no variant is provided, warn the user
        (None, _, _) => warn!("No variant provided"),
    }

    // Verify the integrity of the downloaded file
    info!("Verifying file integrity");
    match verify_binary(file_name, &local_information.md5) {
        Ok(_) => info!("File integrity verified"),
        Err(error) => error!("Failed to verify file integrity: {}", error),
    }
}

fn extract_local_information() -> Result<VersionBuild, String> {
    // Check for existing version
    info!("Checking for existing local version information");

    // If version_history.json exists, read it
    if !Path::new("version_history.json").exists() {
        return Err("Failed to find version history".to_string());
    }

    // Read the file
    let version_history_contents: String = match fs::read_to_string("version_history.json") {
        Ok(version_history_contents) => version_history_contents,
        Err(_) => return Err("Failed to read version history".to_string()),
    };
    let version_history_json: VersionHistory =
        match serde_json::from_str(version_history_contents.as_str()) {
            Ok(version_history_json) => version_history_json,
            Err(_) => return Err("Failed to parse version history".to_string()),
        };

    // Extract the variant, release, and build
    let release_split: Vec<&str> = version_history_json
        .current_version
        .split(" (MC: ")
        .collect();
    let release: String = release_split[1].replace(")", "");

    let variant_build_split: Vec<&str> = release_split[0].split('-').collect();
    let variant: String = variant_build_split[1].to_string().to_lowercase();
    let build: String = variant_build_split[2].to_string();

    // Parse build to u16 and match result
    let build: u16 = match build.parse::<u16>() {
        Ok(build) => build,
        Err(_) => return Err(format!("Failed to parse build number: {}", build)),
    };

    info!("Information found");

    // Return the version and build as struct
    Ok(VersionBuild {
        variant,
        release,
        build,
    })
}

async fn download_file(
    client: &Client,
    url: &String,
    file_name: &str,
) -> Result<(), Box<dyn Error>> {
    // Reqwest setup
    let resp: Response = client
        .get(url)
        .send()
        .await?
        .error_for_status()
        .or(Err(format!("Failed to GET from '{}'", &url)))?;

    // create file
    let mut file = File::create(format!("{}", file_name))
        .or(Err(format!("Failed to create file '{}'", file_name)))?;

    // write bytes to file
    let bytes = resp.bytes().await?;

    file.write_all(&bytes)
        .or(Err(format!("Error while writing to {}", file_name)))?;

    return Ok(());
}

// Standard version+build struct
#[derive(Deserialize)]
struct VersionBuild {
    variant: String,
    release: String,
    build: u16,
}

// version_history.json
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct VersionHistory {
    current_version: String,
}
