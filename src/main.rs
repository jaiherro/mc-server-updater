use clap::Parser;
use reqwest::blocking::{Client, Response};
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
    /// The server variant to download (e.g. paper, purpur)
    #[arg(short, long, default_value = "paper")]
    variant: Option<String>,

    /// The game release to download (e.g. 1.20)
    #[arg(short, long, conflicts_with = "latest")]
    release: Option<String>,

    /// Download the absolute latest release
    #[arg(short, long, conflicts_with = "release")]
    latest: bool,
}

fn main() {
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
    let binary_name: &str = "server.jar";

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

    // Match the arguments to determine the scripts logic
    match (args.variant, args.release, args.latest) {
        // If release is provided, download the requested release
        (Some(variant), Some(release), _) => {
            info!("Downloading requested release");
            match download_release_wrapper(&client, &variant, &release, binary_name) {
                Ok(_) => info!("Downloaded requested release"),
                Err(error) => error!("Failed to download requested release: {}", error),
            }
        }

        // If latest is flagged, download the latest release
        (Some(variant), _, true) => {
            info!("Downloading latest release");
            match download_latest_release_wrapper(&client, &variant, &local_information, binary_name)
            {
                Ok(_) => info!("Downloaded latest release"),
                Err(error) => error!("Failed to download latest release: {}", error),
            }
        }

        // If nothing is provided, try to download the latest build of the local release (if it exists) or the latest release (if it doesn't)
        (Some(variant), _, _) => {
            // info!("Downloading latest release");
            // match download_intelligently_wrapper(&client, &variant, &local_information, file_name) {
            //     Ok(_) => info!("Downloaded latest release"),
            //     Err(error) => error!("Failed to download: {}", error),
            // }
        }

        // If no variant is provided, warn the user
        (None, _, _) => warn!("No variant provided"),
    }
}

// TODO: need to add a check to see if the file already exists

fn download_release_wrapper(
    client: &Client,
    variant: &String,
    release: &String,
    binary_name: &str,
) -> Result<(), Box<dyn Error>> {
    // Match the variant
    match variant.as_str() {
        "paper" => {
            // Get build number
            let build: u16 = match variants::paper::get_build(client, release) {
                Ok(build) => build,
                Err(error) => return Err(error),
            };

            // Get filename
            let build_filename: String =
                match variants::paper::get_build_filename(client, release, &build) {
                    Ok(file_name) => file_name,
                    Err(error) => return Err(error),
                };

            // Get hash
            let build_hash: String = match variants::paper::get_build_hash(client, release, &build)
            {
                Ok(build_hash) => build_hash,
                Err(error) => return Err(error),
            };

            // Get the download url
            let download_url: String = variants::paper::url(release, &build, &build_filename);

            // Download the file
            match download_file(client, &download_url, binary_name) {
                Ok(_) => {}
                Err(error) => return Err(error),
            }

            // Verify the file
            match variants::paper::verify_binary(binary_name, &build_hash) {
                Ok(_) => Ok(()),
                Err(error) => return Err(error),
            }
        }
        "purpur" => {
            // Get build number
            let build: u16 = match variants::purpur::get_build(client, release) {
                Ok(build) => build,
                Err(error) => return Err(error),
            };

            // Get hash
            let build_hash: String = match variants::purpur::get_build_hash(client, release, &build)
            {
                Ok(build_hash) => build_hash,
                Err(error) => return Err(error),
            };

            // Get the download url
            let download_url: String = variants::purpur::url(release, &build);

            // Download the file
            match download_file(client, &download_url, binary_name) {
                Ok(_) => {}
                Err(error) => return Err(error),
            }

            // Verify the file
            match variants::purpur::verify_binary(binary_name, &build_hash) {
                Ok(_) => Ok(()),
                Err(error) => return Err(error),
            }
        }
        _ => Err("Unknown variant".into()),
    }
}

fn download_latest_release_wrapper(
    client: &Client,
    variant: &String,
    local_information: &VersionBuild,
    binary_name: &str,
) -> Result<(), Box<dyn Error>> {
    // Match the variant
    match variant.as_str() {
        "paper" => {
            // Get the latest release
            let release: String = match variants::paper::get_latest_version(client) {
                Ok(release) => release,
                Err(error) => return Err(error),
            };

            // Get build number
            let build: u16 = match variants::paper::get_build(client, &release) {
                Ok(build) => build,
                Err(error) => return Err(error),
            };

            // Get filename
            let build_filename: String =
                match variants::paper::get_build_filename(client, &release, &build) {
                    Ok(file_name) => file_name,
                    Err(error) => return Err(error),
                };

            // Get hash
            let build_hash: String = match variants::paper::get_build_hash(client, &release, &build)
            {
                Ok(build_hash) => build_hash,
                Err(error) => return Err(error),
            };

            // Get the download url
            let download_url: String = variants::paper::url(&release, &build, &build_filename);

            // Download the file
            match download_file(client, &download_url, binary_name) {
                Ok(_) => {}
                Err(error) => return Err(error),
            }

            // Verify the file
            match variants::paper::verify_binary(binary_name, &build_hash) {
                Ok(_) => Ok(()),
                Err(error) => return Err(error),
            }
        }
        "purpur" => {
            // Get the latest release
            let release: String = match variants::purpur::get_latest_version(client) {
                Ok(release) => release,
                Err(error) => return Err(error),
            };

            // Get build number
            let build: u16 = match variants::purpur::get_build(client, &release) {
                Ok(build) => build,
                Err(error) => return Err(error),
            };

            // Get hash
            let build_hash: String =
                match variants::purpur::get_build_hash(client, &release, &build) {
                    Ok(build_hash) => build_hash,
                    Err(error) => return Err(error),
                };

            // Get the download url
            let download_url: String = variants::purpur::url(&release, &build);

            // Download the file
            match download_file(client, &download_url, binary_name) {
                Ok(_) => {}
                Err(error) => return Err(error),
            }

            // Verify the file
            match variants::purpur::verify_binary(binary_name, &build_hash) {
                Ok(_) => Ok(()),
                Err(error) => return Err(error),
            }
        }
        _ => Err("Unknown variant".into()),
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

fn download_file(client: &Client, url: &String, file_name: &str) -> Result<(), Box<dyn Error>> {
    // Reqwest setup
    let resp: Response = client
        .get(url)
        .send()?
        .error_for_status()
        .or(Err(format!("Failed to GET from '{}'", &url)))?;

    // create file
    let mut file = File::create(format!("{}", file_name))
        .or(Err(format!("Failed to create file '{}'", file_name)))?;

    // write bytes to file
    let bytes = resp.bytes()?;

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
