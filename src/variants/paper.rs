use std::{error::Error, fs};

use reqwest::Client;
use serde::Deserialize;
use tracing::{info, error};

use crate::VersionBuild;

pub async fn get_versions(client: &Client) -> Result<Vec<String>, Box<dyn std::error::Error>> {
    let version = client
        .get("https://api.papermc.io/v2/projects/paper/")
        .send()
        .await?
        .json::<Paper>()
        .await?
        .versions;

    Ok(version)
}

pub async fn get_build(
    client: &Client,
    version: &String,
) -> Result<u16, Box<dyn std::error::Error>> {
    let build = client
        .get(
            format!(
                "https://api.papermc.io/v2/projects/paper/versions/{}",
                version
            )
            .as_str(),
        )
        .send()
        .await?
        .json::<Versions>()
        .await?
        .builds
        .pop()
        .unwrap();

    // Parse the build number to u16
    let build: u16 = build.parse::<u16>().unwrap();
    Ok(build)
}

pub async fn get_hash(
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
        .send()
        .await?
        .json::<Builds>()
        .await?;

    Ok(result.downloads.application.sha256.to_uppercase())
}

pub fn verify_binary(file_name: &str, hash: &String) -> Result<(), Box<dyn Error>> {
    // Verifying binary integrity
    info!("Verifying file integrity");
    let contents = fs::read(file_name).expect("Failed to read downloaded file");
    let computed_hash = sha256::digest(contents.try_into().unwrap());
    if &computed_hash == hash {
        info!("Hashes match, file verified");
        return Ok(());
    } else {
        error!("Hashes do not match, downloaded binary may be corrupted, erasing file.");
        fs::remove_file(format!("{}", file_name)).expect("Failed to remove file");
        return Err("Binary failed hash verification".into());
    }
}

// https://api.papermc.io/v2/projects/paper/
#[derive(Deserialize)]
struct Paper {
    versions: Vec<String>,
}

// https://api.papermc.io/v2/projects/paper/versions/{version}
#[derive(Deserialize)]
struct Versions {
    builds: Vec<String>,
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
