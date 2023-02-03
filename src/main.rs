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
    // Get arguments
    let args: Vec<String> = env::args().collect();

    // Help
    if args.len() > 1
        && (args[1] == "-h" || args[1] == "--h" || args[1] == "-help" || args[1] == "--help")
    {
        help();
        return;
    }

    // Setup debugger
    let subscriber = FmtSubscriber::builder()
        .with_max_level(Level::INFO)
        .finish();
    subscriber::set_global_default(subscriber).expect("setting default subscriber failed");

    // Variables
    let file_name = "server.jar";
    let agent = AgentBuilder::new()
        .timeout_read(Duration::from_secs(5))
        .timeout_write(Duration::from_secs(5))
        .build();
    let version_history_current: String;
    let wanted_version: String;
    let wanted_build: u16;

    // Check for existing version
    info!("Checking for existing version information");
    if Path::new("./version_history.json").exists() {
        let version_history_contents = fs::read_to_string("./version_history.json").unwrap();
        let version_history_json: VersionHistory =
            serde_json::from_str(version_history_contents.as_str()).unwrap();
        version_history_current = version_history_json.current_version;
        info!("Existing version found: {}", version_history_current);
    } else {
        version_history_current = "0.0.0".to_string();
        info!("Existing version not found, getting latest");
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
                // List all unknown arguments
                for arg in args.iter() {
                    error!("Unknown argument: {}", arg);
                }

                // Show how to call help
                info!("Use -h or --help to show help");

                // Panic
                panic!("Panicking due to unknown arguments");
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
    let binary_url = format!(
        "https://api.purpurmc.org/v2/purpur/{}/{}/download",
        wanted_version, wanted_build
    );

    // Check if the version is already downloaded
    if version_history_current != requested_constructed_version {
        info!(
            "Now downloading version {} build {}",
            wanted_version, wanted_build
        );
        download_file(&agent, &binary_url, file_name).unwrap();
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
    println!("    If no flags or arguments are specified, and the currently");
    println!("    run version of Purpur can be found, the script will");
    println!("    remain on that version and only download new builds for it.\n");

    // Examples
    println!("EXAMPLES:");
    println!("    {}{} -v 1.19.3", NAME, EXTENSION);
    println!("    {}{} -l", NAME, EXTENSION);
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
