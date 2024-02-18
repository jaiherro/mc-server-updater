use anyhow::{anyhow, bail, Context, Result};
use reqwest::blocking::Client;
use serde::Deserialize;
use sha2::{Digest, Sha256};
use std::{fs::{read, remove_file, File}, io::copy};
use tracing::{error, info};

pub fn download_handler(client: &Client, version: &String, build: &u16) -> Result<()> {
    info!(
        "Starting download for version {} with build {}",
        version, build
    );

    let local_filename = "server.jar".to_string();

    let filename =
        get_build_filename(client, version, build).context("Failed to get the build filename")?;

    let remote_hash =
        get_build_hash(client, version, build).context("Failed to get the build hash")?;

    download_file(client, &url(version, build, &filename), &local_filename)
        .context("Failed to download the file")?;

    verify_binary(&local_filename, &remote_hash).context("Failed to verify the binary")?;

    Ok(())
}

pub fn download_file(client: &Client, url: &str, filename: &str) -> Result<()> {
    info!("Downloading file from {}", url);
    let mut response = client.get(url)
        .send()
        .with_context(|| format!("Failed to send GET request to {}", url))?;

    let mut file = File::create(filename)
        .with_context(|| format!("Failed to create file {}", filename))?;

    copy(&mut response, &mut file)
        .with_context(|| format!("Failed to write to file {}", filename))?;

    Ok(())
}

pub fn url(version: &str, build: &u16, filename: &str) -> String {
    format!(
        "https://api.papermc.io/v2/projects/paper/versions/{}/builds/{}/downloads/{}",
        version, build, filename
    )
}

pub fn get_latest_version(client: &Client) -> Result<String> {
    let version = client
        .get("https://api.papermc.io/v2/projects/paper/")
        .send()
        .with_context(|| "Failed to send GET request to PaperMC API")?
        .json::<Paper>()
        .with_context(|| "Failed to deserialize JSON response from PaperMC API")?
        .versions
        .pop()
        .ok_or_else(|| anyhow!("No versions found in PaperMC API response"));

    version
}

pub fn get_versions(client: &Client) -> Result<Vec<String>> {
    let versions = client
        .get("https://api.papermc.io/v2/projects/paper/")
        .send()
        .with_context(|| "Failed to send GET request to PaperMC API")?
        .json::<Paper>()
        .with_context(|| "Failed to deserialize JSON response from PaperMC API")?
        .versions;

    Ok(versions)
}

pub fn get_build(client: &Client, version: &str) -> Result<u16> {
    let build = client
        .get(format!("https://api.papermc.io/v2/projects/paper/versions/{}", version))
        .send()
        .with_context(|| format!("Failed to send GET request for version {}", version))?
        .json::<Versions>()
        .with_context(|| format!("Failed to deserialize JSON response for version {}", version))?
        .builds
        .pop()
        .ok_or_else(|| anyhow!("No build numbers found for version {}", version))?;

    Ok(build)
}

pub fn get_build_filename(client: &Client, version: &str, build: &u16) -> Result<String> {
    let result = client
        .get(format!("https://api.papermc.io/v2/projects/paper/versions/{}/builds/{}", version, build))
        .send()
        .with_context(|| format!("Failed to send GET request for version {} build {}", version, build))?
        .json::<Builds>()
        .with_context(|| format!("Failed to deserialize JSON response for version {} build {}", version, build))?;

    Ok(result.downloads.application.name)
}


pub fn get_build_hash(client: &Client, version: &str, build: &u16) -> Result<String> {
    let result = client
        .get(format!("https://api.papermc.io/v2/projects/paper/versions/{}/builds/{}", version, build))
        .send()
        .with_context(|| format!("Failed to send GET request for version {} build {}", version, build))?
        .json::<Builds>()
        .with_context(|| format!("Failed to deserialize JSON response for version {} build {}", version, build))?;

    Ok(result.downloads.application.sha256.to_uppercase())
}

pub fn verify_binary(filename: &str, remote_hash: &String) -> Result<()> {
    info!("Verifying integrity of file \"{}\"...", filename);

    // Read file
    let file = read(filename).context(format!("Failed to read file {}", filename))?;

    let mut hasher = Sha256::new();
    hasher.update(&file);
    let result = hasher.finalize();

    // Compare hashes
    let local_hash = format!("{:X}", result);
    if local_hash != *remote_hash {
        error!("Hashes do not match! Erasing downloaded file...");
        remove_file(filename).context(format!("Failed to remove file {}", filename))?;
        bail!(
            "Hash mismatch for file {}: expected {}, got {}",
            filename,
            remote_hash,
            local_hash
        );
    }

    info!("Hashes match! File is authentic.");

    Ok(())
}

// https://api.papermc.io/v2/projects/paper/
#[derive(Deserialize)]
struct Paper {
    versions: Vec<String>,
}

// https://api.papermc.io/v2/projects/paper/versions/{version}
#[derive(Deserialize)]
struct Versions {
    builds: Vec<u16>,
}

// https://api.papermc.io/v2/projects/paper/versions/{version}/builds/{build}
#[derive(Deserialize)]
struct Builds {
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
