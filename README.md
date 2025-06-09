# MAVLink Blackboxer

A Rust library and set of tools for capturing, logging, and replaying MAVLink messages, designed for drone applications. This project allows you to log MAVLink messages to a custom `.bbin` file format and replay them over TCP, with options for filtering and real-time playback.

## Features

- **Capture**: Connects to a MAVLink endpoint and logs messages to a `.bbin` file.

- **Read**: Parses and displays logged messages from `.bbin` files.

- **Replay**: Replays logged messages to a TCP target, with optional filtering and speed control.

- **Configurable**: Supports armed-only logging and custom connection addresses.

- **Efficient**: Uses bincode for serialization and a custom binary format for logs.


## Installation

1. Ensure you have [Rust](https://www.rust-lang.org/tools/install) installed.

2. Add the library to your project:

 ```
cargo add blackboxer
```
 Or, include in your Cargo.toml:

[dependencies]

```
blackboxer = "0.1.3"
```
For binaries, clone the repository and build:git clone https://github.com/Vivek2518/Blackbox-rs.git

```
cd Blackbox-rs
cargo build --release
```


## Dependencies

```
Rust (edition 2021)
mavlink crate (v0.11)
chrono (v0.4)
serde (v1.0, with derive feature)
bincode (v1.3)
```

## Usage
1. **Capture MAVLink Messages**

Captures MAVLink messages from a TCP endpoint and logs them to a .bbin file.

```
cargo run --bin mavlink-capture -- [ADDRESS] [--armed-only]
```

2. **Read BBIN Files**

Reads and displays messages from a .bbin file.

```
cargo run --bin read-bbin -- <FILE> [--show] [--filter=MSG_TYPE]
```

3. **Replay BBIN Files**

Replays messages from a .bbin file to a TCP target.

```
cargo run --bin replay-bbin -- <FILE> <TCP_TARGET> [--filter=MSG_TYPE] [--realtime] [--speed=VALUE]
```

4. **Read and Collect BBIN Data**

Reads and collect data from a .bbin file for displaying data to User Interface.

```
cargo run --bin read-collect -- <FILE> [-- filter=MSG_TYPE]
```


## Library Usage

Use the library in your Rust code to integrate MAVLink logging:

```
use blackboxer::{BlackBoxer, BlackBoxerConfig};
use std::io;

fn main() -> io::Result<()> {
    let config = BlackBoxerConfig {
        armed_only: true,
        addr: "127.0.0.1:14552".to_string(),
    };
    let mut blackboxer = BlackBoxer::new(config)?;
    blackboxer.capture_messages()?;
    Ok(())
}
```

## Project Structure

**src/lib.rs**: Core library with BlackBoxer and BbinWriter for logging.

**src/bin/main.rs**: Binary for capturing messages (Through CLI).

**src/bin/read_bbin.rs**: Binary for reading logs (Through CLI).

**src/bin/replay_bbin.rs**: Binary for replaying logs (Through CLI).

**src/bin/read_collect.rs**:Binary for display data for UI.

## License

This project is licensed under the GNU General Public License v3 (GPL-3.0). Derivative works must be open-sourced under GPL-3.0. See the LICENSE file for details.
Contributing

Fork the repo at https://github.com/Vivek2518/Blackbox-rs.
Submit issues or pull requests for bugs, features, or improvements via GitHub.

## Support

If you find this project useful, consider supporting development:

Feel free to reach out

## Contact

Author: Vivek Patwari
Email: vivekpatwari38@gmail.com
Repository: https://github.com/Vivek2518/Blackbox-rs



