use serde::Deserialize;
use std::{
    env,
    error::Error,
    fs::{self, File},
    io::Write,
    path::Path,
    time::Duration,
};
use tracing::{error, info, subscriber, warn, Level};
use tracing_subscriber::FmtSubscriber;
use ureq::{Agent, AgentBuilder};

fn main() {
    // Setup debugger
    let subscriber = FmtSubscriber::builder()
        .with_max_level(Level::INFO)
        .finish();
    subscriber::set_global_default(subscriber).expect("setting default subscriber failed");

    // Get arguments
    let args: Vec<String> = env::args().collect();

    // Variables
    let file_name = "purpur.jar";
    let agent = AgentBuilder::new()
        .timeout_read(Duration::from_secs(5))
        .timeout_write(Duration::from_secs(5))
        .build();
    let version_history_current: String;
    let wanted_version: String;
    let wanted_build: u16;

    // Check for existing version
    info!("Checking for current version information");
    if Path::new("./version_history.json").exists() {
        let version_history_contents = fs::read_to_string("./version_history.json").unwrap();
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
                let mut all_game_versions: Vec<String> =
                    get_all_game_versions(&agent).expect("Failed to get versions");
                wanted_version = get_latest_valid_version(&agent, &mut all_game_versions)
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
            let mut all_game_versions: Vec<String> =
                get_all_game_versions(&agent).expect("Failed to get versions");
            wanted_version = get_latest_valid_version(&agent, &mut all_game_versions)
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
    wanted_build = get_latest_build(&agent, &wanted_version).expect("Failed to get build");

    // Construct the version comparison string
    let requested_constructed_version =
        format!("git-Purpur-{} (MC: {})", wanted_build, wanted_version);

    // Get hash for version
    info!("Desired version is: {}", requested_constructed_version);
    let hash = get_hash(&agent, &wanted_version, &wanted_build).expect("Failed to get file.");

    // Construct the URL
    let url = format!(
        "https://api.purpurmc.org/v2/purpur/{}/{}/download",
        wanted_version, wanted_build
    );

    // Check if the version is already downloaded
    if version_history_current != requested_constructed_version {
        info!(
            "Now downloading version {} build {}",
            wanted_version, wanted_build
        );
        download_file(&agent, &url, file_name).unwrap();
        verify_binary(&file_name, &hash).expect("Failed to verify binary");
    } else {
        info!("Server is already up-to-date")
    }

    info!("All tasks completed")
}

fn get_latest_valid_version(
    agent: &Agent,
    all_game_versions: &Vec<String>,
) -> Result<String, Box<dyn std::error::Error>> {
    for version in all_game_versions.iter().rev() {
        match get_latest_build(&agent, &version) {
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

fn get_all_game_versions(agent: &Agent) -> Result<Vec<String>, Box<dyn std::error::Error>> {
    let version = agent
        .get("https://api.purpurmc.org/v2/purpur")
        .call()?
        .into_json::<Purpur>()?
        .versions;

    Ok(version)
}

fn get_latest_build(agent: &Agent, version: &String) -> Result<u16, Box<dyn std::error::Error>> {
    let build = agent
        .get(format!("https://api.purpurmc.org/v2/purpur/{}", version).as_str())
        .call()?
        .into_json::<Version>()?
        .builds
        .all
        .pop()
        .unwrap();

    Ok(build.parse::<u16>().unwrap())
}

fn get_hash(agent: &Agent, version: &String, build: &u16) -> Result<String, Box<dyn Error>> {
    let result = agent
        .get(format!("https://api.purpurmc.org/v2/purpur/{}/{}", version, build).as_str())
        .call()?
        .into_json::<Build>()?;

    Ok(result.md5.to_uppercase())
}

fn download_file(agent: &Agent, url: &String, file_name: &str) -> Result<(), Box<dyn Error>> {
    // Reqwest setup
    let resp = agent
        .get(url)
        .call()
        .or(Err(format!("Failed to GET from '{}'", &url)))?;

    // create file
    let mut file = File::create(format!("./{}", file_name))
        .or(Err(format!("Failed to create file '{}'", file_name)))?;

    // write bytes to file
    let mut reader = resp.into_reader();
    let mut bytes = vec![];
    reader.read_to_end(&mut bytes)?;

    file.write_all(&bytes)
        .or(Err(format!("Error while writing to {}", file_name)))?;

    return Ok(());
}

fn verify_binary(file_name: &str, hash: &String) -> Result<(), Box<dyn Error>> {
    // Verifying binary integrity
    info!("Verifying file integrity");
    let contents = fs::read(format!("./{}", file_name)).expect("Failed to read downloaded file");
    let digest = md5::compute(contents);
    let downloaded_file_hash = format!("{:X}", digest);
    if &downloaded_file_hash == hash {
        info!("Hashes match, file verified");
        return Ok(());
    } else {
        error!("Hashes do not match, downloaded binary may be corrupted, erasing file.");
        fs::remove_file(format!("./{}", file_name)).expect("Failed to remove file");
        return Err("Binary failed hash verification".into());
    }
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
    project: String,
    version: String,
}
#[derive(Deserialize)]
struct Builds {
    all: Vec<String>,
    latest: String,
}

// https://api.purpurmc.org/v2/purpur/{version}/{build} or https://api.purpurmc.org/v2/purpur/{version}/latest
#[derive(Deserialize)]
struct Build {
    build: String,
    md5: String,
    project: String,
    result: String,
    timestamp: u64,
    version: String,
}

// version_history.json
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct VersionHistory {
    current_version: String,
}
