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
use tracing::{info, subscriber, Level};
use tracing_subscriber::FmtSubscriber;

#[tokio::main]
async fn main() {
    // Setup debugger
    let subscriber = FmtSubscriber::builder()
        .with_max_level(Level::INFO)
        .finish();
    subscriber::set_global_default(subscriber).expect("setting default subscriber failed");

    // Get preferred version from arguments if it exists.
    let args: Vec<String> = env::args().collect();

    // Variables
    let client = Client::new();
    let version: String;

    // Check for passed version to override automatic latest
    if args.len() > 1 {
        info!("Argument passed, assuming version manually specified");
        version = env::args().nth(1).unwrap();
    } else {
        info!("Getting latest version");
        version = get_version(&client).await.expect("Failed to get version");
    }

    // Get latest build number for version
    info!("Getting latest build number");
    let build = get_build(&client, &version)
        .await
        .expect("Failed to get build!");

    // Get file name for version and build number
    info!("Getting latest file name and checksum");
    let file = get_file(&client, &version, &build)
        .await
        .expect("Failed to get file!");

    // Construct the URL
    let url = format!(
        "https://papermc.io/api/v2/projects/paper/versions/{}/builds/{}/downloads/{}",
        version, build, file.0
    );

    // Construct the version comparison
    let latest_version = format!("git-Paper-{} (MC: {})", build, version);
    let version_history: VersionHistory;

    // Check if a version_history.json exists, if so, read it and compare with latest version information
    if Path::new("./version_history.json").exists() {
        let version_history_contents = fs::read_to_string("./version_history.json").await.unwrap();
        version_history = serde_json::from_str(version_history_contents.as_str()).unwrap();

        if latest_version != version_history.currentVersion {
            info!("Updating server binary");
            download_file(&client, &url, &file).await.unwrap();
        } else {
            info!("Server is already up-to-date")
        }
    } else {
        info!("Unable to discern server version, getting fresh binary");
        download_file(&client, &url, &file).await.unwrap();
    }

    info!("All tasks completed")
}

async fn get_version(client: &Client) -> Result<String, Box<dyn std::error::Error>> {
    client
        .get("https://papermc.io/api/v2/projects/paper")
        .send()
        .await?
        .json::<Version>()
        .await?
        .versions
        .pop()
        .ok_or_else(|| "needed at least one version".into())
}

async fn get_build(client: &Client, version: &String) -> Result<i64, Box<dyn std::error::Error>> {
    client
        .get(format!(
            "https://papermc.io/api/v2/projects/paper/versions/{}",
            version
        ))
        .send()
        .await?
        .json::<Build>()
        .await?
        .builds
        .pop()
        .ok_or_else(|| "needed at least one build".into())
}

async fn get_file(
    client: &Client,
    version: &String,
    build: &i64,
) -> Result<(String, String), Box<dyn Error>> {
    let result = client
        .get(format!(
            "https://papermc.io/api/v2/projects/paper/versions/{}/builds/{}",
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
    pb.set_message(&format!(
        "Downloading {} from https://papermc.io/",
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

    pb.finish_with_message(&format!("Downloaded {}", file_information.0));

    // Verifying binary integrity
    info!("Verifying checksums");
    let contents = fs::read("./paper.jar").await?;
    let mut hasher = Sha256::new();
    hasher.update(contents);
    let hash_result = format!("{:X}", hasher.finalize());

    if hash_result == file_information.1 {
        info!("Checksums verified successfully")
    } else {
        fs::remove_file("./paper.jar").await?;
        return Err(
            "Checksums do not match, deleting downloaded binary and invoking a panic".into(),
        );
    }

    return Ok(());
}

#[derive(Deserialize)]
struct Version {
    versions: Vec<String>,
}

#[derive(Deserialize)]
struct Build {
    builds: Vec<i64>,
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
struct VersionHistory {
    currentVersion: String,
}
