use std::{
    error::Error,
    fs::{read, remove_file, File},
};

use anyhow::Result;
use reqwest::blocking::Client;
use serde::Deserialize;
use sha2::{Digest, Sha256};
use tracing::{error, info};

pub fn download_handler(
    client: &Client,
    version: &String,
    build: &u16,
) -> Result<(), Box<dyn Error>> {
    // Local filename
    let local_filename = "server.jar".to_string();

    // Get filename
    let filename = get_build_filename(client, version, build)?;

    // Get hash
    let hash = get_build_hash(client, version, build)?;

    // Download the file
    download_file(client, &url(version, build, &filename), &local_filename)?;

    // Verify the file
    verify_binary(&local_filename, &hash)?;

    Ok(())
}

pub fn download_file(
    client: &Client,
    url: &String,
    filename: &String,
) -> Result<(), Box<dyn Error>> {
    // Download the file
    info!("Downloading file from {}", url);
    let mut response = client.get(url).send()?;
    let mut file = File::create(filename)?;

    // Write the file
    std::io::copy(&mut response, &mut file)?;

    Ok(())
}

pub fn url(version: &str, build: &u16, filename: &String) -> String {
    // Construct the URL
    return format!(
        "https://api.papermc.io/v2/projects/paper/versions/{}/builds/{}/downloads/{}",
        version, build, filename
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

// pub fn get_versions(client: &Client) -> Result<Vec<String>> {
//     let version = client
//         .get("https://api.papermc.io/v2/projects/paper/")
//         .send()?
//         .json::<Paper>()?
//         .versions;

//     Ok(version)
// }

pub fn get_build(client: &Client, version: &str) -> Result<u16> {
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
    version: &str,
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
    version: &str,
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

pub fn verify_binary(filename: &str, hash: &String) -> Result<(), Box<dyn Error>> {
    info!("Verifying integrity of file \"{}\"...", filename);

    // Read file
    let file = read(filename)?;

    let mut hasher = Sha256::new();
    hasher.update(&file);
    let result = hasher.finalize();

    // Compare hashes
    let file_hash = format!("{:X}", result);
    if file_hash != *hash {
        error!("Hashes do not match");
        remove_file(filename)?;
        return Err("Hashes do not match".into());
    }

    info!("Hashes match! File is verified.");

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