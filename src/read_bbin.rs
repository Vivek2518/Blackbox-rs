use std::env;
use std::fs::File;
use std::io::{BufReader, Read, ErrorKind};
use bincode::{deserialize_from, ErrorKind as BincodeErrorKind};
use serde::Deserialize;
use mavlink::{read_v2_msg};
use mavlink::ardupilotmega::MavMessage;

#[derive(Debug, Deserialize)]
struct LoggedMessageHeader {
    timestamp: i64,
    _sequence: u8,
    _system_id: u8,
    _component_id: u8,
    msg_len: u16,
}

fn main() -> std::io::Result<()> {
    let args: Vec<String> = env::args().collect();

    if args.len() < 2 {
        eprintln!("Usage: {} <file.bbin> [--show] [--filter=MSG_TYPE]", args[0]);
        return Ok(());
    }

    let file_path = args[1].clone();
    let show = args.contains(&"--show".to_string());
    let filter_arg = args.iter().find(|s| s.starts_with("--filter="));
    let filter_msg_type = filter_arg
        .map(|s| s.trim_start_matches("--filter=").to_string());

    let file = File::open(file_path)?;
    let mut reader = BufReader::new(file);

    loop {
        // Try to read header first
        let header: LoggedMessageHeader = match deserialize_from(&mut reader) {
            Ok(h) => h,
            Err(e) => {
                match *e {
                    BincodeErrorKind::Io(ref io_err) if io_err.kind() == ErrorKind::UnexpectedEof => {
                        println!("Reached EOF or incomplete message, stopping.");
                        break;
                    }
                    _ => {
                        eprintln!("Header read error: {}", e);
                        break;
                    }
                }
            }
        };

        // Then read the raw message bytes
        let mut msg_buf = vec![0u8; header.msg_len as usize];
        if let Err(e) = reader.read_exact(&mut msg_buf) {
            eprintln!("Failed to read message bytes: {}", e);
            break;
        }

        // Now decode it with mavlink
        let mut packet: &[u8] = &msg_buf;
        match read_v2_msg::<MavMessage, _>(&mut packet) {
            Ok((_hdr, msg)) => {
                let msg_type_str = format!("{:?}", msg);

                if show {
                    if let Some(filter) = &filter_msg_type {
                        if msg_type_str.contains(filter) {
                            println!("{} message: {:?}\nTimestamp: {}", filter, msg, header.timestamp);
                        }
                    } else {
                        println!("Message: {:?}\nTimestamp: {}", msg, header.timestamp);
                    }
                }
            }
            Err(e) => {
                eprintln!("Failed to decode MAVLink message: {:?}", e);
            }
        }
    }

    Ok(())
}
