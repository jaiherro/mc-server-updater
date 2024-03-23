# MC Server Updater

MC Server Updater is a command-line tool written in Rust that automates the process of updating Minecraft servers to the latest version of the Paper server software.

## Features

- Automatically checks for the latest version of Paper and downloads the server JAR file
- Verifies the integrity of the downloaded file using SHA256 hash comparison
- Supports specifying a specific Minecraft version to download
- Stores version history locally for easy tracking of updates
- Fast, efficient, and reliable updating process

## Installation

1. Go to the [GitHub releases page](https://github.com/jaiherro/mc-server-updater/releases) for the MC Server Updater repository.
2. Download the latest release binary for your operating system.
3. Place the downloaded binary in the root directory of your Minecraft server.
4. Ensure that the binary has executable permissions. On Unix-based systems, you can use the following command:
   ```
   chmod +x updater
   ```

## Usage

To update your Minecraft server to the latest version of Paper, navigate to your server's root directory and run the `updater` binary:

```
./updater
```

By default, the tool will check for the latest version and download the corresponding server JAR file.

If you want to update to a specific Minecraft version, you can use the `--version` or `-v` flag followed by the desired version number:

```
./updater --version 1.20
```

The updated `server.jar` file will be downloaded to the current directory, overwriting any existing file with the same name.

### Automatic Updating on Server Startup

To automatically update your Minecraft server to the latest version every time you start it, you can include the MC Server Updater in your server's startup script. Here's an example of how you can modify your startup script:

```bash
#!/bin/bash

# Run MC Server Updater
./updater

# Start the Minecraft server
java -Xmx2G -jar server.jar nogui
```

In this example, the `updater` binary is executed before starting the Minecraft server. This ensures that the server is always updated to the latest version before it is launched.

Make sure to adjust the startup script according to your server's specific requirements, such as the amount of memory allocated (`-Xmx` flag) and any additional Java options.

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