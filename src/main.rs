use reqwest::Client;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{env, error::Error, path::Path};
use tokio::{
    fs::{self, File},
    io::AsyncWriteExt,
};
use tracing::{error, info, subscriber, warn, Level};
use tracing_subscriber::FmtSubscriber;

#[tokio::main]
async fn main() {
    // Setup debugger
    let subscriber = FmtSubscriber::builder()
        .with_max_level(Level::INFO)
        .finish();
    subscriber::set_global_default(subscriber).expect("setting default subscriber failed");

    // Get arguments
    let args: Vec<String> = env::args().collect();

    // Variables
    let file_name = "purpur.jar";
    let client = Client::new();
    let version_history_current: String;
    let wanted_version: String;
    let wanted_build: u16;

    // Check for existing version
    info!("Checking for current version information");
    if Path::new("./version_history.json").exists() {
        let version_history_contents = fs::read_to_string("./version_history.json").await.unwrap();
        let version_history_json: VersionHistory =
            serde_json::from_str(version_history_contents.as_str()).unwrap();
        version_history_current = version_history_json.current_version;
        info!("Current version found: {}", version_history_current);
    } else {
        version_history_current = "0.0.0".to_string();
        info!("Current version not found, getting latest");
    }

    // Check for passed version to override automatic latest, latest flag, or none of them
    if args.len() > 1 {
        match args[1].as_str() {
            // Check for passed version
            "-v" | "--v" | "-version" | "--version" => {
                info!("Version {} requested", &args[2]);
                wanted_version = args[2].clone();
            }
            // Check for latest flag
            "-l" | "--l" | "-latest" | "--latest" => {
                info!("Absolute latest version requested");
                let mut all_game_versions: Vec<String> = get_all_game_versions(&client)
                    .await
                    .expect("Failed to get versions");
                wanted_version = get_latest_valid_version(&client, &mut all_game_versions)
                    .await
                    .expect("Couldn't get latest version");
            }
            // If none of the above, panic
            _ => {
                panic!("Unknown argument: {}", args[1]);
            }
        }
    } else {
        // If version_history.json couldn't be found or read, use the latest version
        if version_history_current == "0.0.0" {
            let mut all_game_versions: Vec<String> = get_all_game_versions(&client)
                .await
                .expect("Failed to get versions");
            wanted_version = get_latest_valid_version(&client, &mut all_game_versions)
                .await
                .expect("Couldn't get latest version");
        // If version_history.json was found, extract the current version
        } else {
            wanted_version = version_history_current
                .split("(MC: ")
                .collect::<Vec<&str>>()[1]
                .strip_suffix(")")
                .expect("Failed to strip suffix")
                .to_string();
        }
    }

    // Get the latest build number for a specific version
    info!("Getting latest build for version: {}", wanted_version);
    wanted_build = get_latest_build(&client, &wanted_version)
        .await
        .expect("Failed to get build");

    // Construct the version comparison string
    let requested_constructed_version =
        format!("git-Purpur-{} (MC: {})", wanted_build, wanted_version);

    // Get hash for version
    info!("Desired version is: {}", requested_constructed_version);
    let hash = get_hash(&client, &wanted_version, &wanted_build)
        .await
        .expect("Failed to get file.");

    // Construct the URL
    let url = format!(
        "https://api.purpurmc.org/v2/purpur/{}/{}/download",
        wanted_version, wanted_build
    );

    // Check if the version is already downloaded
    if version_history_current != requested_constructed_version {
        info!(
            "Now downloading version {} build {}",
            wanted_version, wanted_build
        );
        download_file(&client, &url, file_name).await.unwrap();
        verify_binary(file_name, &hash)
            .await
            .expect("Failed to verify binary");
    } else {
        info!("Server is already up-to-date")
    }

    info!("All tasks completed")
}

async fn get_latest_valid_version(
    client: &Client,
    all_game_versions: &Vec<String>,
) -> Result<String, Box<dyn std::error::Error>> {
    for version in all_game_versions.iter().rev() {
        match get_latest_build(client, &version).await {
            Ok(_) => {
                return Ok(version.clone().into());
            }
            Err(_) => {
                warn!(
                    "No builds exist for {}, checking next latest version",
                    version
                );
                continue;
            }
        };
    }
    return Err("No valid versions found".into());
}

async fn get_all_game_versions(client: &Client) -> Result<Vec<String>, Box<dyn std::error::Error>> {
    let version = client
        .get("https://api.purpurmc.org/v2/purpur")
        .send()
        .await?
        .json::<Purpur>()
        .await?
        .versions;

    Ok(version)
}

async fn get_latest_build(client: &Client, version: &String) -> Result<u16, Box<dyn std::error::Error>> {
    let debug = client
        .get(format!("https://api.purpurmc.org/v2/purpur/{}", version))
        .send()
        .await?
        .json::<Version>()
        .await?;

    println!("{}", serde_json::to_string_pretty(&debug).unwrap());

    let build = client
        .get(format!("https://api.purpurmc.org/v2/purpur/{}", version))
        .send()
        .await?
        .json::<Version>()
        .await?
        .builds
        .all
        .pop()
        .ok_or_else(|| "Needed at least one build but found none")?;

    Ok(build)
}

async fn get_hash(
    client: &Client,
    version: &String,
    build: &u16,
) -> Result<String, Box<dyn Error>> {
    let result = client
        .get(format!(
            "https://api.purpurmc.org/v2/purpur/{}/{}",
            version, build
        ))
        .send()
        .await?
        .json::<Build>()
        .await?;

    Ok(result.md5.to_uppercase())
}

async fn download_file(
    client: &Client,
    url: &String,
    file_name: &str,
) -> Result<(), Box<dyn Error>> {
    // Reqwest setup
    let res = client
        .get(url)
        .send()
        .await
        .or(Err(format!("Failed to GET from '{}'", &url)))?;

    // create file
    let mut file = File::create("./purpur.jar")
        .await
        .or(Err(format!("Failed to create file '{}'", file_name)))?;

    // write bytes to file
    file.write(&res.bytes().await?)
        .await
        .or(Err(format!("Error while writing to {}", file_name)))?;

    return Ok(());
}

async fn verify_binary(file_name: &str, hash: &String) -> Result<(), Box<dyn Error>> {
    // Verifying binary integrity
    info!("Verifying file integrity");
    let contents = fs::read(format!("./{}", file_name))
        .await
        .expect("Failed to read downloaded file");
    let mut hasher = Sha256::new();
    hasher.update(contents);
    let hash_result = format!("{:X}", hasher.finalize());
    if &hash_result == hash {
        info!("Hashes match, file verified");
        return Ok(());
    } else {
        error!("Hashes do not match, downloaded binary may be corrupted, erasing file");
        fs::remove_file("./purpur.jar")
            .await
            .expect("Failed to remove file");
        return Err("Binary failed hash verification".into());
    }
}

// https://api.purpurmc.org/v2/purpur
#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
struct Purpur {
    versions: Vec<String>,
}

// https://api.purpurmc.org/v2/purpur/{version}
#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
struct Version {
    builds: Builds,
    project: String,
    version: String,
}
#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
struct Builds {
    all: Vec<u16>,
    latest: u16,
}

// https://api.purpurmc.org/v2/purpur/{version}/{build} or https://api.purpurmc.org/v2/purpur/{version}/latest
#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
struct Build {
    build: u16,
    md5: String,
    project: String,
    result: String,
    timestamp: u64,
    version: String,
}

// version_history.json
#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
struct VersionHistory {
    current_version: String,
}
