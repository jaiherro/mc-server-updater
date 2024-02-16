use anyhow::Result;
use clap::Parser;
use reqwest::blocking::{Client, Response};
use serde::Deserialize;
use serde_json::Value;
use std::{
    error::Error,
    fs::{self, File},
    io::Write,
    path::Path,
};
use tracing::{error, info, warn, Level};
use tracing_subscriber::FmtSubscriber;
use regex::Regex;

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
        download_specific_version(&client, &version, "server.jar");
    } else if args.latest {
        info!("Downloading the latest version.");
        download_latest_version(&client, &local_information, "server.jar");
    } else {
        info!("Determining appropriate action...");
        download_based_on_local_information(&client, &local_information, "server.jar");
    }
}

fn setup_logging() {
    let subscriber = FmtSubscriber::builder()
        .with_max_level(Level::INFO)
        .finish();
    tracing::subscriber::set_global_default(subscriber).expect("Setting default subscriber failed");
}

fn get_local_version_information_or_default() -> VersionInformation {
    match get_local_version_information() {
        Ok(info) => info,
        Err(_) => {
            warn!("Failed to extract local version and build information, using defaults.");
            VersionInformation {
                server_type: String::from("paper"),
                version: String::from("0.0"),
                build: 0,
            }
        }
    }
}

fn download_specific_version(client: &Client, version: &str, binary_name: &str) {
    match paper_logic(client, version, binary_name) {
        Ok(_) => info!("Successfully downloaded version: {}", version),
        Err(e) => error!("Failed to download requested version: {}", e),
    }
}

fn download_latest_version(client: &Client, local_information: &VersionInformation, binary_name: &str) {
    match download_latest_version_wrapper(client, local_information, binary_name) {
        Ok(_) => info!("Successfully downloaded the latest version."),
        Err(e) => error!("Failed to download the latest version: {}", e),
    }
}

fn download_based_on_local_information(client: &Client, local_information: &VersionInformation, binary_name: &str) {
    // Example placeholder for logic to intelligently download based on local information
    // This is where you'd implement the commented-out logic, adjusted as necessary.
    info!("Placeholder for intelligent download logic based on local information.");
}

// TODO: need to add a check to see if the file already exists

fn paper_logic(client: &Client, version: &str, binary_name: &str) -> Result<(), Box<dyn Error>> {
    let build = &modules::paper::get_build(client, version)?;

    let build_filename = modules::paper::get_build_filename(client, version, build)?;

    let build_hash = modules::paper::get_build_hash(client, version, build)?;

    let download_url = modules::paper::url(version, build, &build_filename);

    download_file(client, &download_url, binary_name)?;

    // Verify the file and directly return the result
    modules::paper::verify_binary(binary_name, &build_hash)
}

fn download_latest_version_wrapper(
    client: &Client,
    local_information: &VersionInformation,
    binary_name: &str,
) -> Result<(), Box<dyn Error>> {
    // Get the latest version
    let version: String = modules::paper::get_latest_version(client)?;

    // Get build number
    let build: u16 = modules::paper::get_build(client, &version)?;

    if compare_versions(local_information, &VersionInformation {
        server_type: String::from("paper"),
        version: version.clone(),
        build,
    }) {
        info!("Latest version already downloaded");
        return Ok(());
    }

    // Get filename
    let build_filename: String = modules::paper::get_build_filename(client, &version, &build)?;

    // Get hash
    let build_hash: String = modules::paper::get_build_hash(client, &version, &build)?;

    // Get the download url
    let download_url: String = modules::paper::url(&version, &build, &build_filename);

    // Download the file
    download_file(client, &download_url, binary_name)?;

    // Verify the file
    modules::paper::verify_binary(binary_name, &build_hash)?;

    Ok(())
}

fn compare_versions(local_version: &VersionInformation, remote_version: &VersionInformation) -> bool {
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
    let caps = re.captures(current_version)
        .ok_or_else(|| "Failed to match version pattern")?;

    let server_type = caps.get(1).ok_or_else(|| "Failed to extract server type")?.as_str().to_string();
    let build = caps.get(2).ok_or_else(|| "Failed to extract build")?.as_str().parse::<u16>()?;
    let version = caps.get(3).ok_or_else(|| "Failed to extract version")?.as_str().to_string();

    info!("Information found: [{}, {}, {}]", server_type, version, build);

    Ok(VersionInformation {
        server_type,
        version,
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

    Ok(())
}

// Standard struct
#[derive(Deserialize)]
struct VersionInformation {
    server_type: String,
    version: String,
    build: u16,
}
