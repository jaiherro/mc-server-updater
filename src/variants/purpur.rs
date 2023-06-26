use std::{
    error::Error,
    fs::{read, remove_file},
};

use md5::{Digest, Md5};
use reqwest::Client;
use serde::Deserialize;
use tracing::{error, info};

use crate::VersionBuild;

pub async fn get_versions(client: &Client) -> Result<Vec<String>, Box<dyn std::error::Error>> {
    let version = client
        .get("https://api.purpurmc.org/v2/purpur")
        .send()
        .await?
        .json::<Purpur>()
        .await?
        .versions;

    Ok(version)
}

pub async fn get_build(
    client: &Client,
    version: &String,
) -> Result<u16, Box<dyn std::error::Error>> {
    let build: String = client
        .get(format!("https://api.purpurmc.org/v2/purpur/{}", version).as_str())
        .send()
        .await?
        .json::<Version>()
        .await?
        .builds
        .latest;

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
        .get(format!("https://api.purpurmc.org/v2/purpur/{}/{}", version, build).as_str())
        .send()
        .await?
        .json::<Build>()
        .await?;

    Ok(result.md5.to_uppercase())
}

pub fn verify_binary(file_name: &str, hash: &String) -> Result<(), Box<dyn Error>> {
    // Verifying binary integrity
    info!("Verifying file integrity");

    // Read file
    let file = read(file_name)?;

    let mut hasher = Md5::new();
    hasher.update(&file);
    let result = hasher.finalize();

    // Compare hashes
    let file_hash = format!("{:X}", result);
    if file_hash != *hash {
        error!("Hashes do not match");
        remove_file(&file_name).expect("Failed to remove file");
        return Err("Hashes do not match".into());
    }

    info!("Hashes match");

    Ok(())
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
    latest: String,
    all: Vec<String>,
}

// https://api.purpurmc.org/v2/purpur/{version}/{build} or https://api.purpurmc.org/v2/purpur/{version}/latest
#[derive(Deserialize)]
struct Build {
    md5: String,
}
