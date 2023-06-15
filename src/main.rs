use clap::Parser;
use reqwest::{Client, Response};
use serde::Deserialize;
use std::{
    env,
    error::Error,
    fs::{self, File},
    io::Write,
    ops::ControlFlow,
    path::Path,
    time::Duration,
};
use tracing::{error, info, subscriber, warn, Level};
use tracing_subscriber::FmtSubscriber;

#[derive(Parser)]
#[command(author, version, about)]
struct Args {
    /// The game version to download (e.g. 1.20)
    #[arg(short, long, conflicts_with = "latest")]
    version: String,

    /// Download the absolute latest version
    #[arg(short, long, conflicts_with = "version")]
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
    let local_version_build: VersionBuild = match extract_local_version_and_build() {
        Ok(local_version_build) => local_version_build,
        Err(error) => {
            error!(
                "Failed to extract local version and build information: {}",
                error
            );
            panic!("Panicking due to failed extraction of local version and build information");
        }
    };

    // Determine what version and build to download
    let version_build: VersionBuild = if args.latest {
        // Get the latest version and build information from the API
        let latest_version_build: VersionBuild = match get_latest_version_and_build(&client).await {
            Ok(latest_version_build) => latest_version_build,
            Err(error) => {
                error!(
                    "Failed to get latest version and build information: {}",
                    error
                );
                panic!(
                    "Panicking due to failed extraction of latest version and build information"
                );
            }
        };
        // Check if the latest build is newer than the local build
        if latest_version_build.version != local_version_build.version
            && latest_version_build.build > local_version_build.build
        {
            info!("Latest version is newer than local version");
            latest_version_build
        } else {
            info!("Latest version is not newer than local version");
            local_version_build
        }
    } else {
        // Check if the passed version is newer than the local version
        if args.version > local_version_build.version {
            info!("Passed version is newer than local version");
            VersionBuild {
                version: args.version,
                build: 0,
            }
        } else {
            info!("Passed version is not newer than local version");
            local_version_build
        }
    };

    // Check if the version and build are the same as the local version and build
    if version_build.version == local_version_build.version
        && version_build.build == local_version_build.build
    {
        info!("Version and build are the same as the local version and build");
        info!("Skipping download");
        return;
    }

    // Download the server.jar
    info!("Downloading server.jar");
    match download_server_jar(&client, version_build).await {
        Ok(_) => info!("Downloaded server.jar"),
        Err(error) => {
            error!("Failed to download server.jar: {}", error);
            panic!("Panicking due to failed download of server.jar");
        }
    };
}

fn extract_local_version_and_build() -> Result<VersionBuild, String> {
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

    // Extract the version and build
    let version_split: Vec<&str> = version_history_json
        .current_version
        .split(" (MC: ")
        .collect();
    let version: String = version_split[1].replace(")", "");
    let build: String = version_split[0].replace("git-Purpur-", "");

    // Parse build to u16 and match result
    let build: u16 = match build.parse::<u16>() {
        Ok(build) => build,
        Err(_) => return Err(format!("Failed to parse build number: {}", build)),
    };

    info!("Information found");

    // Return the version and build as struct
    Ok(VersionBuild { version, build })
}

async fn get_latest_version_and_build(client: &Client) -> Result<VersionBuild, String> {
    // Get the latest version and build information from the API
    info!("Getting latest version and build information from API");
    let latest_version_build: VersionBuild =
        match client.get("https://api.pl3x.net/v2/purpur").send().await {
            Ok(response) => match response.json::<VersionBuild>().await {
                Ok(latest_version_build) => latest_version_build,
                Err(_) => return Err("Failed to parse response".to_string()),
            },
            Err(_) => return Err("Failed to get response".to_string()),
        };
    info!("Latest version and build information found");

    // Return the latest version and build as struct
    Ok(latest_version_build)
}

async fn get_all_game_versions(client: &Client) -> Result<Vec<String>, Box<dyn std::error::Error>> {
    let version = client
        .get("https://api.purpurmc.org/v2/purpur")
        .call()?
        .into_json::<Purpur>()?
        .versions;

    Ok(version)
}

async fn get_latest_build(client: &Client, version: &String) -> Result<u16, Box<dyn std::error::Error>> {
    let build = client
        .get(format!("https://api.purpurmc.org/v2/purpur/{}", version).as_str())
        .call()?
        .into_json::<Version>()?
        .builds
        .all
        .pop()
        .unwrap();

    Ok(build.parse::<u16>().unwrap())
}

async fn get_hash(client: &Client, version: &String, build: &u16) -> Result<String, Box<dyn Error>> {
    let result = client
        .get(format!("https://api.purpurmc.org/v2/purpur/{}/{}", version, build).as_str())
        .call()?
        .into_json::<Build>()?;

    Ok(result.md5.to_uppercase())
}

fn download_file(client: &Client, url: &String, file_name: &str) -> Result<(), Box<dyn Error>> {
    // Reqwest setup
    let resp = client
        .get(url)
        .call()
        .or(Err(format!("Failed to GET from '{}'", &url)))?;

    // create file
    let mut file = File::create(format!("{}", file_name))
        .or(Err(format!("Failed to create file '{}'", file_name)))?;

    // write bytes to file
    let mut reader = resp.into_reader();
    let mut bytes = vec![];
    reader.read_to_end(&mut bytes)?;

    file.write_all(&bytes)
        .or(Err(format!("Error while writing to {}", file_name)))?;

    return Ok(());
}

fn verify_binary(file_name: &str, hash: &String) -> Result<(), Box<dyn Error>> {
    // Verifying binary integrity
    info!("Verifying file integrity");
    let contents = fs::read(format!("{}", file_name)).expect("Failed to read downloaded file");
    let digest = md5::compute(contents);
    let downloaded_file_hash = format!("{:X}", digest);
    if &downloaded_file_hash == hash {
        info!("Hashes match, file verified");
        return Ok(());
    } else {
        error!("Hashes do not match, downloaded binary may be corrupted, erasing file.");
        fs::remove_file(format!("{}", file_name)).expect("Failed to remove file");
        return Err("Binary failed hash verification".into());
    }
}

#[derive(Deserialize)]
struct VersionBuild {
    version: String,
    build: u16,
}

// https://api.purpurmc.org/v2/purpur
#[derive(Deserialize)]
struct Purpur {
    versions: Vec<String>,
}

// https://api.purpurmc.org/v2/purpur/{version}
#[derive(Deserialize)]
struct Version {
    builds: Builds,
}
#[derive(Deserialize)]
struct Builds {
    all: Vec<String>,
}

// https://api.purpurmc.org/v2/purpur/{version}/{build} or https://api.purpurmc.org/v2/purpur/{version}/latest
#[derive(Deserialize)]
struct Build {
    md5: String,
}

// version_history.json
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct VersionHistory {
    current_version: String,
}
