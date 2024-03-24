# MC Server Updater

MC Server Updater is a command-line tool written in Rust that automates the process of updating Minecraft servers to the latest version.

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

## Building from Source

1. Ensure you have Rust installed on your system. If not, [install Rust](https://www.rust-lang.org/tools/install).
2. Clone the repository and navigate to the project directory:
```
git clone https://github.com/jaiherro/mc-server-updater.git
cd mc-server-updater
```
1. Build the project using Cargo:
```
cargo build --release
```
   The compiled binary will be located in the `target/release` directory.

## Cross Compilation Guide

MC Server Updater can be cross-compiled for different target architectures using the `cross` tool. This allows you to build the updater binary for platforms other than the one you are currently using.

### Prerequisites

- [Install Rust](https://www.rust-lang.org/tools/install) on your system.
- Install `cross` by running the following command:
```
cargo install cross --git https://github.com/cross-rs/cross
```

### Cross Compiling

To cross-compile MC Server Updater for a specific target architecture, use the `cross build` command followed by the `--target` flag and the desired target triple. For example, to build for `x86_64-unknown-linux-gnu` (64-bit x86 Linux), run:

```
cross build --release --target x86_64-unknown-linux-gnu
```

The cross-compiled binary will be located in the `target/x86_64-unknown-linux-gnu/release` directory.

You can find a list of supported targets in the [cross documentation](https://github.com/cross-rs/cross#supported-targets).

## License

This project is licensed under the [MIT License](LICENSE).