use futures_util::StreamExt;
use indicatif::{ProgressBar, ProgressStyle};
use reqwest::Client;
use serde::Deserialize;
use sha2::{Digest, Sha256};
use std::{
    cmp::{self},
    env,
    error::Error,
    path::Path,
};
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
                wanted_version = latest_valid_version(&client, &mut all_game_versions)
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
            wanted_version = latest_valid_version(&client, &mut all_game_versions)
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
    wanted_build = get_build(&client, &wanted_version)
        .await
        .expect("Failed to get build");

    // Construct the version comparison string
    let requested_constructed_version =
        format!("git-Paper-{} (MC: {})", wanted_build, wanted_version);

    // Get file name for version and build number
    info!("Desired version is: {}", requested_constructed_version);
    let file = get_file(&client, &wanted_version, &wanted_build)
        .await
        .expect("Failed to get file.");

    // Construct the URL
    let url = format!(
        "https://api.papermc.io/v2/projects/paper/versions/{}/builds/{}/downloads/{}",
        wanted_version, wanted_build, file.0
    );

    // Check if the version is already downloaded
    if version_history_current != requested_constructed_version {
        info!(
            "Now downloading version {} build {}",
            wanted_version, wanted_build
        );
        download_file(&client, &url, &file.0).await.unwrap();
        verify_binary(&file.1)
            .await
            .expect("Failed to verify binary");
    } else {
        info!("Server is already up-to-date")
    }

    info!("All tasks completed")
}

async fn latest_valid_version(
    client: &Client,
    all_game_versions: &Vec<String>,
) -> Result<String, Box<dyn std::error::Error>> {
    for version in all_game_versions.iter().rev() {
        match get_build(client, &version).await {
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
        .get("https://api.papermc.io/v2/projects/paper")
        .send()
        .await?
        .json::<Version>()
        .await?
        .versions;

    Ok(version)
}

async fn get_build(client: &Client, version: &String) -> Result<u16, Box<dyn std::error::Error>> {
    client
        .get(format!(
            "https://api.papermc.io/v2/projects/paper/versions/{}",
            version
        ))
        .send()
        .await?
        .json::<Build>()
        .await?
        .builds
        .pop()
        .ok_or_else(|| "Needed at least one build but found none".into())
}

async fn get_file(
    client: &Client,
    version: &String,
    build: &u16,
) -> Result<(String, String), Box<dyn Error>> {
    let result = client
        .get(format!(
            "https://api.papermc.io/v2/projects/paper/versions/{}/builds/{}",
            version, build
        ))
        .send()
        .await?
        .json::<Project>()
        .await?;

    Ok((
        result.downloads.application.name,
        result.downloads.application.sha256.to_uppercase(),
    ))
}

async fn download_file(
    client: &Client,
    url: &String,
    file_name: &String,
) -> Result<(), Box<dyn Error>> {
    // Reqwest setup
    let res = client
        .get(url)
        .send()
        .await
        .or(Err(format!("Failed to GET from '{}'", &url)))?;
    let total_size = res
        .content_length()
        .ok_or(format!("Failed to get content length from '{}'", &url))?;

    // Indicatif setup
    let pb = ProgressBar::new(total_size);
    pb.set_style(ProgressStyle::default_bar()
        .template("{spinner:.white} [{wide_bar:.white}] {bytes}/{total_bytes} ({bytes_per_sec}, {eta})")
        .progress_chars("#>-"));

    // download chunks
    let mut file = File::create("./paper.jar")
        .await
        .or(Err(format!("Failed to create file '{}'", file_name)))?;
    let mut downloaded: u64 = 0;
    let mut stream = res.bytes_stream();

    while let Some(item) = stream.next().await {
        let chunk = item.or(Err(format!("Error while downloading {}", file_name)))?;
        file.write(&chunk)
            .await
            .or(Err(format!("Error while writing to {}", file_name)))?;
        let new = cmp::min(downloaded + (chunk.len() as u64), total_size);
        downloaded = new;
        pb.set_position(new);
    }

    return Ok(());
}

async fn verify_binary(hash: &String) -> Result<(), Box<dyn Error>> {
    // Verifying binary integrity
    info!("Verifying file integrity");
    let contents = fs::read("./paper.jar")
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
        fs::remove_file("./paper.jar")
            .await
            .expect("Failed to remove file");
        return Err("Binary failed hash verification".into());
    }
}

#[derive(Deserialize)]
struct Version {
    versions: Vec<String>,
}

#[derive(Deserialize)]
struct Build {
    builds: Vec<u16>,
}

#[derive(Deserialize)]
struct Project {
    downloads: Downloads,
}

#[derive(Deserialize)]
struct Downloads {
    application: Application,
}

#[derive(Deserialize)]
struct Application {
    name: String,
    sha256: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VersionHistory {
    pub current_version: String,
}
