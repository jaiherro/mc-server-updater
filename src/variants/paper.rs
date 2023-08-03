use std::{
    error::Error,
    fs::{read, remove_file},
};

use anyhow::Result;
use reqwest::blocking::Client;
use serde::Deserialize;
use sha2::{Digest, Sha256};
use tracing::{error, info};

pub fn url(release: &String, build: &u16, filename: &String) -> String {
    // Construct the URL
    return format!(
        "https://api.papermc.io/v2/projects/paper/versions/{}/builds/{}/downloads/{}",
        release, build, filename
    );
}

pub fn get_latest_version(client: &Client) -> Result<String> {
    let version = client
        .get("https://api.papermc.io/v2/projects/paper/")
        .send()?
        .json::<Paper>()?
        .versions
        .pop()
        .unwrap();

    Ok(version)
}

pub fn get_versions(client: &Client) -> Result<Vec<String>> {
    let version = client
        .get("https://api.papermc.io/v2/projects/paper/")
        .send()?
        .json::<Paper>()?
        .versions;

    Ok(version)
}

pub fn get_build(client: &Client, version: &String) -> Result<u16> {
    let build = client
        .get(
            format!(
                "https://api.papermc.io/v2/projects/paper/versions/{}",
                version
            )
            .as_str(),
        )
        .send()?
        .json::<Versions>()?
        .builds
        .pop()
        .unwrap();

    Ok(build)
}

pub fn get_build_filename(
    client: &Client,
    version: &String,
    build: &u16,
) -> Result<String, Box<dyn Error>> {
    let result = client
        .get(
            format!(
                "https://api.papermc.io/v2/projects/paper/versions/{}/builds/{}",
                version, build
            )
            .as_str(),
        )
        .send()?
        .json::<Builds>()?;

    Ok(result.downloads.application.name)
}

pub fn get_build_hash(
    client: &Client,
    version: &String,
    build: &u16,
) -> Result<String, Box<dyn Error>> {
    let result = client
        .get(
            format!(
                "https://api.papermc.io/v2/projects/paper/versions/{}/builds/{}",
                version, build
            )
            .as_str(),
        )
        .send()?
        .json::<Builds>()?;

    Ok(result.downloads.application.sha256.to_uppercase())
}

pub fn verify_binary(binary_name: &str, hash: &String) -> Result<(), Box<dyn Error>> {
    // Verifying binary integrity
    info!("Verifying file integrity");

    // Read file
    let file = read(binary_name)?;

    let mut hasher = Sha256::new();
    hasher.update(&file);
    let result = hasher.finalize();

    // Compare hashes
    let file_hash = format!("{:X}", result);
    if file_hash != *hash {
        error!("Hashes do not match");
        remove_file(binary_name).expect("Failed to remove file");
        return Err("Hashes do not match".into());
    }

    info!("Hashes match");

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
