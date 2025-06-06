# MAVLink Blackboxer

A Rust library and set of tools for capturing, logging, and replaying MAVLink messages, designed for drone applications. This project allows you to log MAVLink messages to a custom `.bbin` file format and replay them over TCP, with options for filtering and real-time playback.

## Features
- **Capture**: Connects to a MAVLink endpoint (e.g., `127.0.0.1:14552`) and logs messages to a `.bbin` file.
- **Read**: Parses and displays logged messages from `.bbin` files.
- **Replay**: Replays logged messages to a TCP target, with optional filtering and speed control.
- **Configurable**: Supports armed-only logging and custom connection addresses.
- **Efficient**: Uses bincode for serialization and a custom binary format for logs.

## Installation

1. Ensure you have [Rust](https://www.rust-lang.org/tools/install) installed.
2. Add the library to your project:
 ```
   cargo add blackboxer

   Or, include in your Cargo.toml:

   [dependencies]

   blackboxer = "0.1.0"

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


ADDRESS: TCP address to connect to (default: 127.0.0.1:14552).
```
--armed-only: Only log messages when the vehicle is armed.

```

Example:cargo run --bin mavlink-capture -- 127.0.0.1:14550 --armed-only
Outputs a file like mavlink_log_20250606_191100.bbin.



2. **Read BBIN Files**
Reads and displays messages from a .bbin file.
```
cargo run --bin read-bbin -- <FILE> [--show] [--filter=MSG_TYPE]

```

<FILE>: Path to the .bbin file.
--show: Display message details.
--filter=MSG_TYPE: Filter by message type (e.g., HEARTBEAT).
Example:cargo run --bin read-bbin -- mavlink_log_20250606_191100.bbin --show --filter=HEARTBEAT



3. **Replay BBIN Files**
Replays messages from a .bbin file to a TCP target.
```
cargo run --bin replay-bbin -- <FILE> <TCP_TARGET> [--filter=MSG_TYPE] [--realtime] [--speed=VALUE]

```

<FILE>: Path to the .bbin file.
<TCP_TARGET>: Target TCP address (e.g., 127.0.0.1:14550).
--filter=MSG_TYPE: Replay only specific message types.
--realtime: Replay with original timing.
--speed=VALUE: Adjust replay speed (e.g., 2.0 for 2x faster, 0.5 for half speed).
Example:cargo run --bin replay-bbin -- mavlink_log_20250606_191100.bbin 127.0.0.1:14550 --realtime --speed=1.5



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

src/lib.rs: Core library with BlackBoxer and BbinWriter for logging.
src/bin/main.rs: Binary for capturing messages.
src/bin/read_bbin.rs: Binary for reading logs.
src/bin/replay_bbin.rs: Binary for replaying logs.

## License
This project is licensed under the GNU General Public License v3 (GPL-3.0). Derivative works must be open-sourced under GPL-3.0. See the LICENSE file for details.
Contributing

Fork the repo at https://github.com/Vivek2518/Blackbox-rs.
Submit issues or pull requests for bugs, features, or improvements via GitHub.

## Support
If you find this project useful, consider supporting development:

GitHub Sponsors (Update with your link)
PayPal (Update with your link)

## Contact

Author: Vivek Patwari
Email: vivekpatwari38@gmail.com
Repository: https://github.com/Vivek2518/Blackbox-rs



