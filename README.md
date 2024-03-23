# MC Server Updater

MC Server Updater is a command-line tool written in Rust that automates the process of updating Minecraft servers to the latest version of the Paper server software.

## Features

- Automatically checks for the latest version of Paper and downloads the server JAR file
- Verifies the integrity of the downloaded file using SHA256 hash comparison
- Supports specifying a specific Minecraft version to download
- Fast, efficient, and reliable updating process

## Installation

1. Go to the [GitHub releases page](https://github.com/jaiherro/mc-server-updater/releases) for the MC Server Updater repository.
2. Download the latest release binary for your operating system.
3. Place the downloaded binary in the root directory of your Minecraft server.
4. Ensure that the binary has executable permissions. On Unix-based systems, you can use the following command:
   ```
   chmod +x mc-server-updater
   ```

## Usage

To update your Minecraft server to the latest version of Paper, navigate to your server's root directory and run the `mc-server-updater` binary:

```
./mc-server-updater
```

By default, the tool will check for the latest version and download the corresponding server JAR file.

If you want to update to a specific Minecraft version, you can use the `--version` or `-v` flag followed by the desired version number:

```
./mc-server-updater --version 1.20
```

The updated `server.jar` file will be downloaded to the current directory, overwriting any existing file with the same name.

## Configuration

The MC Server Updater does not require any additional configuration files. However, it does store version history information in a `version_history.json` file located in the same directory as the binary. This file is automatically created and updated by the tool.

## Building from Source

If you prefer to build the MC Server Updater from source, follow these steps:

1. Ensure you have Rust installed on your system. If not, follow the official installation guide at https://www.rust-lang.org/tools/install.
2. Clone the repository:
   ```
   git clone https://github.com/jaiherro/mc-server-updater.git
   ```
3. Navigate to the project directory:
   ```
   cd mc-server-updater
   ```
4. Build the project:
   ```
   cargo build --release
   ```
   The compiled binary will be located in the `target/release` directory.

## Contribution Guidelines

Contributions to the MC Server Updater project are welcome! If you would like to contribute, please follow these steps:

1. Fork the repository on GitHub.
2. Create a new branch with a descriptive name for your feature or bug fix.
3. Make your changes, following the code style and conventions used in the project.
4. Write tests for your changes and ensure all existing tests pass.
5. Commit your changes and push them to your forked repository.
6. Submit a pull request to the main repository, describing your changes in detail.

Please ensure that your contributions align with the project's scope and objectives.

## Testing

The project includes unit tests to ensure the correctness and reliability of the code. To run the tests, use the following command:

```
cargo test
```

Make sure all tests pass before submitting a pull request.

## License

This project is licensed under the [MIT License](LICENSE).

## Acknowledgements

The MC Server Updater project relies on the following dependencies:

- [reqwest](https://crates.io/crates/reqwest) - A simple and powerful Rust HTTP client
- [serde](https://crates.io/crates/serde) - A generic serialization/deserialization framework
- [serde_json](https://crates.io/crates/serde_json) - A JSON serialization file format
- [clap](https://crates.io/crates/clap) - A simple to use, efficient, and full-featured command line argument parser
- [sha2](https://crates.io/crates/sha2) - SHA-2 hash functions
- [tracing](https://crates.io/crates/tracing) - Application-level tracing for Rust
- [anyhow](https://crates.io/crates/anyhow) - Flexible concrete Error type built on std::error::Error

Special thanks to the developers and maintainers of these libraries for their valuable contributions to the Rust ecosystem.