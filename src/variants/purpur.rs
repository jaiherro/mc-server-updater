use std::error::Error;

use reqwest::Client;
use serde::Deserialize;
use tracing::info;

use crate::VersionBuild;

pub async fn get_latest_version_and_build(client: &Client) -> Result<VersionBuild, String> {
    // Get the latest version and build information from the API
    info!("Getting latest version and build information from API");
    let latest_version_build: VersionBuild =
        match client.get("https://api.pl3x.net/v2/purpur").send().await {
            Ok(response) => match response.json::<VersionBuild>().await {
                Ok(latest_version_build) => latest_version_build,
                Err(_) => return Err("Failed to parse response".to_string()),
            },
            Err(_) => return Err("Failed to get response".to_string()),
        };
    info!("Latest version and build information found");

    // Return the latest version and build as struct
    Ok(latest_version_build)
}

pub async fn get_all_game_versions(
    client: &Client,
) -> Result<Vec<String>, Box<dyn std::error::Error>> {
    let version = client
        .get("https://api.purpurmc.org/v2/purpur")
        .send()
        .await?
        .json::<Purpur>()
        .await?
        .versions;

    Ok(version)
}

pub async fn get_latest_build(
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
