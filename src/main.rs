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
use tracing::{info, subscriber, warn, Level};
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
    let mut version_history: VersionHistory;
    let mut constructed_version: std::string::String = std::string::String::new();
    let mut game_version: String;
    let build_number: u16;

    // Check version_history.json (if it exists)
    if Path::new("./version_history.json").exists() {
        let version_history_contents = fs::read_to_string("./version_history.json").await.unwrap();
        version_history =
            serde_json::from_str(version_history_contents.as_str()).unwrap();
    } else {
        info!("Unable to discern current server version");
        let mut all_game_versions: Vec<std::string::String> = get_all_game_versions(&client)
            .await
            .expect("Failed to get versions");
        game_version = latest_valid_version(&client, &mut all_game_versions)
            .await
            .expect("Couldn't get latest version");
    }

    // Check for passed version to override automatic latest
    if args.len() > 1 {
        match args[1].as_str() {
            "-v" | "--v" | "-version" | "--version" => {
                info!("Specific version requested, skipping automatic latest");
                game_version = args[2].clone();
            }
            "-l" | "--l" | "-latest" | "--latest" => {
                info!("Absolute latest version requested, ignoring current version");
                let mut all_game_versions: Vec<std::string::String> =
                    get_all_game_versions(&client)
                        .await
                        .expect("Failed to get versions");
                game_version = latest_valid_version(&client, &mut all_game_versions)
                    .await
                    .expect("Couldn't get latest version");
            }
            _ => {
                panic!("Unknown argument: {}", args[1]);
            }
        }
    } else {
        constructed_version = version_history.current_version;
        game_version = constructed_version
            .split("(MC: ")
            .collect::<Vec<&str>>()[2]
            .strip_suffix(")\"")
            .expect("Failed to strip suffix")
            .to_string();

        // if Path::new("./version_history.json").exists() {
        //     let version_history_contents =
        //         fs::read_to_string("./version_history.json").await.unwrap();
        //     let version_history_json: VersionHistory =
        //         serde_json::from_str(version_history_contents.as_str()).unwrap();
        //     current_constructed_version_build = version_history_json.current_version;
        //     game_version = current_constructed_version_build
        //         .split("(MC: ")
        //         .collect::<Vec<&str>>()[2]
        //         .strip_suffix(")\"")
        //         .expect("Failed to strip suffix")
        //         .to_string();
        // } else {
        //     info!("Unable to discern current server version");
        //     let mut all_game_versions: Vec<std::string::String> = get_all_game_versions(&client)
        //         .await
        //         .expect("Failed to get versions");
        //     game_version = latest_valid_version(&client, &mut all_game_versions)
        //         .await
        //         .expect("Couldn't get latest version");
        // }
    }

    build_number = get_build(&client, &game_version)
        .await
        .expect("Failed to get build");

    // Get file name for version and build number
    info!("Getting latest file name and checksum");
    let file = get_file(&client, &game_version, &build_number)
        .await
        .expect("Failed to get file.");

    // Construct the URL
    let url = format!(
        "https://api.papermc.io/v2/projects/paper/versions/{}/builds/{}/downloads/{}",
        game_version, build_number, file.0
    );

    // Construct the version comparison string
    let constructed_version_build = format!("git-Paper-{} (MC: {})", build_number, game_version);

    if constructed_version != constructed_version_build {
        info!("Updating server binary");
        download_file(&client, &url, &file).await.unwrap();
    } else {
        info!("Server is already up-to-date")
    }

    info!("All tasks completed")
}

async fn latest_valid_version(
    client: &Client,
    all_game_versions: &Vec<std::string::String>,
) -> Result<std::string::String, Box<dyn std::error::Error>> {
    for version in all_game_versions.iter().rev() {
        match get_build(client, &version).await {
            Ok(_) => {
                return Ok(version.clone().into());
            }
            Err(_) => {
                warn!("No builds exist for {}, checking next latest", version);
                continue;
            }
        };
    }
    return Err("No valid versions found".into());
}

async fn get_all_game_versions(
    client: &Client,
) -> Result<Vec<std::string::String>, Box<dyn std::error::Error>> {
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
    file_information: &(String, String),
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
        .template("{msg}\n{spinner:.green} [{elapsed_precise}] [{wide_bar:.cyan/blue}] {bytes}/{total_bytes} ({bytes_per_sec}, {eta})")
        .progress_chars("#>-"));
    pb.set_message(format!(
        "Downloading {} from https://api.papermc.io/",
        file_information.0
    ));

    // download chunks
    let mut file = File::create("./paper.jar").await.or(Err(format!(
        "Failed to create file '{}'",
        file_information.0
    )))?;
    let mut downloaded: u64 = 0;
    let mut stream = res.bytes_stream();

    while let Some(item) = stream.next().await {
        let chunk = item.or(Err(format!(
            "Error while downloading {}",
            file_information.0
        )))?;
        file.write(&chunk).await.or(Err(format!(
            "Error while writing to {}",
            file_information.0
        )))?;
        let new = cmp::min(downloaded + (chunk.len() as u64), total_size);
        downloaded = new;
        pb.set_position(new);
    }

    pb.finish_with_message(format!("Downloaded {}", file_information.0));

    // Verifying binary integrity
    info!("Verifying hashes");
    let contents = fs::read("./paper.jar").await?;
    let mut hasher = Sha256::new();
    hasher.update(contents);
    let hash_result = format!("{:X}", hasher.finalize());

    if hash_result == file_information.1 {
        info!("Hashes verified successfully, binary is authentic");
    } else {
        println!(
            "The binary failed verification, assuming malicious and erasing to protect the host"
        );
        fs::remove_file("./paper.jar")
            .await
            .expect("Failed to remove file, this is very bad");
        return Err("Binary failed hash verification".into());
    }

    return Ok(());
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
    pub old_version: String,
    pub current_version: String,
}
