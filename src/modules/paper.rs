use anyhow::{Context, Result};
use reqwest::blocking::Client;
use serde::Deserialize;
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::fs::{self, read, remove_file, File};
use std::io::copy;
use std::path::Path;
use tracing::{error, info};

pub fn download_handler(client: &Client, version: &str) -> Result<()> {
    info!("Getting build information for version: {}", version);
    let build = get_build(client, version)
        .context(format!("Failed to get build for version {}", version))?;

    info!("Getting download URL...");
    let filename =
        get_build_filename(client, version, &build).context("Failed to get build filename")?;
    let url = url(version, &build, &filename);

    info!("Getting remote hash...");
    let remote_hash =
        get_build_hash(client, version, &build).context("Failed to get remote hash")?;

    info!("Downloading server jar...");
    let local_filename = "server.jar";
    download_file(client, &url, local_filename).context("Failed to download server jar")?;

    info!("Verifying downloaded file...");
    verify_binary(local_filename, &remote_hash).context("Failed to verify downloaded file")?;

    Ok(())
}

pub fn get_latest_version(client: &Client) -> Result<String> {
    let project: Project = client
        .get("https://api.papermc.io/v2/projects/paper")
        .send()
        .context("Failed to get latest version")?
        .json()
        .context("Failed to parse latest version response")?;

    let latest_version = project.versions.last().context("No versions found")?;

    Ok(latest_version.to_string())
}

pub fn get_local_version_information() -> Result<Value> {
    let path = Path::new("version_history.json");
    if !path.exists() {
        return Ok(Value::default());
    }

    let contents = fs::read_to_string(path).context("Failed to read version history")?;
    let version_history: Value =
        serde_json::from_str(&contents).context("Failed to parse version history")?;

    Ok(version_history)
}

pub fn url(version: &str, build: &u16, filename: &str) -> String {
    format!(
        "https://api.papermc.io/v2/projects/paper/versions/{}/builds/{}/downloads/{}",
        version, build, filename
    )
}

pub fn get_build(client: &Client, version: &str) -> Result<u16> {
    let version_info: Version = client
        .get(format!(
            "https://api.papermc.io/v2/projects/paper/versions/{}",
            version
        ))
        .send()
        .with_context(|| format!("Failed to get build for version {}", version))?
        .json()
        .with_context(|| format!("Failed to parse build for version {}", version))?;

    Ok(version_info.builds.into_iter().max().unwrap_or(0))
}

fn get_build_filename(client: &Client, version: &str, build: &u16) -> Result<String> {
    let build_info: Build = client
        .get(format!(
            "https://api.papermc.io/v2/projects/paper/versions/{}/builds/{}",
            version, build
        ))
        .send()
        .with_context(|| {
            format!(
                "Failed to get download URL for version {} build {}",
                version, build
            )
        })?
        .json()
        .with_context(|| {
            format!(
                "Failed to parse download URL for version {} build {}",
                version, build
            )
        })?;

    Ok(build_info.downloads.application.name)
}

fn get_build_hash(client: &Client, version: &str, build: &u16) -> Result<String> {
    let build_info: Build = client
        .get(format!(
            "https://api.papermc.io/v2/projects/paper/versions/{}/builds/{}",
            version, build
        ))
        .send()
        .with_context(|| format!("Failed to get hash for version {} build {}", version, build))?
        .json()
        .with_context(|| {
            format!(
                "Failed to parse hash for version {} build {}",
                version, build
            )
        })?;

    Ok(build_info.downloads.application.sha256.to_uppercase())
}

fn download_file(client: &Client, url: &str, filename: &str) -> Result<()> {
    let mut response = client
        .get(url)
        .send()
        .context(format!("Failed to download from {}", url))?;
    let mut file = File::create(filename).context(format!("Failed to create file {}", filename))?;
    copy(&mut response, &mut file).context(format!("Failed to write to file {}", filename))?;
    Ok(())
}

fn verify_binary(filename: &str, remote_hash: &str) -> Result<()> {
    let contents = read(filename).context(format!("Failed to read file {}", filename))?;
    let mut hasher = Sha256::new();
    hasher.update(&contents);
    let local_hash = format!("{:X}", hasher.finalize());

    if local_hash != remote_hash {
        error!(
            "Hash mismatch for {}: expected {}, got {}",
            filename, remote_hash, local_hash
        );
        remove_file(filename).context(format!("Failed to remove file {}", filename))?;
        return Err(anyhow::anyhow!("Hash verification failed for {}", filename));
    }

    info!("Hash verified for {}", filename);
    Ok(())
}

#[derive(Deserialize)]
struct Project {
    versions: Vec<String>,
}

#[derive(Deserialize)]
struct Version {
    builds: Vec<u16>,
}

#[derive(Deserialize)]
struct Build {
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
