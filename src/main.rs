use reqwest::blocking::{Client, Response};
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

fn main() {
    let client = Client::new();
    let response = client
        .get("https://www.rust-lang.org")
        .send()
        .expect("working");

    let body = response.text().expect("working");
    println!("body = {:?}", body);
}

fn old_main() {
    // Get arguments
    let args: Vec<String> = env::args().collect();

    // Variables
    let file_name: &str = "server.jar";
    let agent: Agent = AgentBuilder::new()
        .timeout_read(Duration::from_secs(5))
        .timeout_write(Duration::from_secs(5))
        .build();
    let mut current_version_build: (String, u16) = ("".to_string(), 0);
    let mut wanted_version_build: (String, u16) = ("".to_string(), 0);

    let current_version_exists: bool;
    let mut passed_version: bool = false;
    let mut wants_latest: bool = false;

    // Setup debugger
    let subscriber = FmtSubscriber::builder()
        .with_max_level(Level::INFO)
        .finish();
    subscriber::set_global_default(subscriber).expect("setting default subscriber failed");

    // Flag/argument parser
    if args.len() > 1 {
        match args[1].as_str() {
            // Version
            "-v" | "--v" | "-version" | "--version" => {
                info!("Version {} requested", &args[2]);
                passed_version = true;
            }

            // Latest
            "-l" | "--l" | "-latest" | "--latest" => {
                info!("Latest version requested");
                wants_latest = true;
            }

            // Help
            "-h" | "--h" | "-help" | "--help" => {
                help();
                return;
            }

            // Unknown
            _ => {
                // List all unknown arguments
                // TODO: add .advance_by(1) to skip the first argument once the function is stabilised https://github.com/rust-lang/rust/issues/77404
                // for arg in args.iter() {
                //     error!("Unknown argument: {}", arg);
                // }

                // Announce the first unknown argument
                error!("Unknown argument: {}", args[1]);

                // Show how to call help
                info!("Use -h or --help to show help");

                // Panic
                panic!("Panicking due to unknown arguments");
            }
        }
    }

    // Check for existing version
    info!("Checking for existing version information");

    // If version_history.json exists, read it
    if Path::new("version_history.json").exists() {
        // Read the file
        let version_history_contents = fs::read_to_string("version_history.json").unwrap();
        let version_history_json: VersionHistory =
            serde_json::from_str(version_history_contents.as_str()).unwrap();

        info!("Information found");

        // Set the current version
        current_version_exists = true;
        current_version_build = extract_version_and_build(&version_history_json.current_version);
    } else {
        info!("No information found");
        current_version_exists = false;
    }

    // Determine the version to download
    if passed_version {
        wanted_version_build.0 = args[2].clone();
    } else if wants_latest | !current_version_exists {
        let mut all_game_versions: Vec<String> =
            get_all_game_versions(&agent).expect("Failed to get versions");
        wanted_version_build.0 = get_latest_valid_version(&agent, &mut all_game_versions)
            .expect("Couldn't get latest version");
    } else if current_version_exists {
        wanted_version_build.0 = current_version_build.0.clone();
    } else {
        panic!("Failed to intelligently determine appropriate version");
    }

    // Get the latest build number for a specific version
    wanted_version_build.1 =
        get_latest_build(&agent, &wanted_version_build.0).expect("Failed to get build");

    // Log the existing version if it exists
    if current_version_exists {
        info!(
            "Existing version: Purpur {} build {}",
            current_version_build.0, current_version_build.1
        );
    }

    // Log the wanted version
    info!(
        "Wanted version: Purpur {} build {}",
        wanted_version_build.0, wanted_version_build.1
    );

    // Check if the version is already downloaded
    if current_version_exists
        && (current_version_build.0 == wanted_version_build.0
            && current_version_build.1 == wanted_version_build.1)
    {
        info!("Server is already up-to-date");
        return;
    }

    // Get hash for version
    let hash = get_hash(&agent, &wanted_version_build.0, &wanted_version_build.1)
        .expect("Failed to get file.");

    // Construct the URL
    let binary_url = format!(
        "https://api.purpurmc.org/v2/purpur/{}/{}/download",
        wanted_version_build.0, wanted_version_build.1
    );

    // Download the file
    info!(
        "Downloading: Purpur {} build {}",
        wanted_version_build.0, wanted_version_build.1
    );
    download_file(&agent, &binary_url, file_name).unwrap();
    verify_binary(&file_name, &hash).expect("Failed to verify binary");

    info!("All tasks completed successfully, server is up-to-date");
}

fn extract_version_and_build(version: &str) -> (String, u16) {
    let version_split: Vec<&str> = version.split(" (MC: ").collect();
    let version = version_split[1].replace(")", "");
    let build = version_split[0].replace("git-Purpur-", "");
    return (version, build.parse().unwrap());
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
    let mut file = File::create(format!("{}", file_name))
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
    let contents = fs::read(format!("{}", file_name)).expect("Failed to read downloaded file");
    let digest = md5::compute(contents);
    let downloaded_file_hash = format!("{:X}", digest);
    if &downloaded_file_hash == hash {
        info!("Hashes match, file verified");
        return Ok(());
    } else {
        error!("Hashes do not match, downloaded binary may be corrupted, erasing file.");
        fs::remove_file(format!("{}", file_name)).expect("Failed to remove file");
        return Err("Binary failed hash verification".into());
    }
}

fn help() {
    // Get version from Cargo.toml
    const NAME: &str = env!("CARGO_PKG_NAME");
    const VERSION: &str = env!("CARGO_PKG_VERSION");

    #[cfg(any(target_os = "linux", target_os = "macos"))]
    const EXTENSION: &str = "";

    #[cfg(target_os = "windows")]
    const EXTENSION: &str = ".exe";

    // Print help
    println!("updater v{}\n", VERSION);
    println!("USAGE:");
    println!("    {}{} [FLAGS] [ARGS]\n", NAME, EXTENSION);

    // Flags
    println!("FLAGS:");
    println!("    -v, -version    Specify a version to download");
    println!("    -l, -latest     Download the latest version\n");

    // Arguments
    println!("ARGS:");
    println!("    VERSION         Specify a version to download\n");

    // Discussion
    println!("DISCUSSION:");
    println!("    If no flags or arguments are specified, and the currently run");
    println!("    version of Purpur (Minecraft) can be found, the script will");
    println!("    remain on that version and only download new builds for it.\n");

    // Examples
    println!("EXAMPLES:");
    println!("    {}{}", NAME, EXTENSION);
    println!("    {}{} -l", NAME, EXTENSION);
    println!("    {}{} -v 1.19.3", NAME, EXTENSION);
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
    all: Vec<String>,
}

// https://api.purpurmc.org/v2/purpur/{version}/{build} or https://api.purpurmc.org/v2/purpur/{version}/latest
#[derive(Deserialize)]
struct Build {
    md5: String,
}

// version_history.json
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct VersionHistory {
    current_version: String,
}
