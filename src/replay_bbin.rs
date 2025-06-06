use std::env;
use std::fs::File;
use std::io::{BufReader, Read, Write};
use std::net::TcpStream;
use std::thread::sleep;
use std::time::{Duration, Instant};

use bincode::deserialize_from;
use mavlink::{read_v2_msg, write_v2_msg};
use mavlink::ardupilotmega::MavMessage;
use serde::Deserialize;

#[derive(Debug, Deserialize)]
struct LoggedMessageHeader {
    timestamp: i64,
    sequence: u8,
    system_id: u8,
    component_id: u8,
    msg_len: u16,
}

fn main() -> std::io::Result<()> {
    let args: Vec<String> = env::args().collect();

    if args.len() < 3 {
        eprintln!("Usage: {} <file.bbin> <tcp_target> [--filter=MSG_TYPE] [--realtime] [--speed=VALUE]", args[0]);
        return Ok(());
    }

    let file_path = &args[1];
    let target = &args[2];
    let filter_arg = args.iter().find(|a| a.starts_with("--filter="));
    let filter = filter_arg.map(|s| s.trim_start_matches("--filter=").to_string());
    let realtime = args.contains(&"--realtime".to_string());
    let speed_arg = args.iter().find(|a| a.starts_with("--speed="));
    let speed: f32 = speed_arg
        .map(|s| s.trim_start_matches("--speed="))
        .and_then(|s| s.parse().ok())
        .unwrap_or(1.0); // Default to 1.0 (normal speed) if not specified or invalid

    if speed <= 0.0 {
        eprintln!("Error: --speed must be a positive value");
        return Ok(());
    }

    let mut stream = TcpStream::connect(target)?;
    println!("Connected to {}", target);

    let file = File::open(file_path)?;
    let mut reader = BufReader::new(file);

    let mut prev_time: Option<i64> = None;
    let _start = Instant::now();

    loop {
        let header: LoggedMessageHeader = match deserialize_from(&mut reader) {
            Ok(h) => h,
            Err(_) => {
                println!("Reached end of file or encountered error.");
                break;
            }
        };

        let mut msg_buf = vec![0u8; header.msg_len as usize];
        if let Err(e) = reader.read_exact(&mut msg_buf) {
            eprintln!("Failed to read message bytes: {}", e);
            break;
        }

        let mut packet = &msg_buf[..];
        if let Ok((_, msg)) = read_v2_msg::<MavMessage, _>(&mut packet) {
            let msg_type = format!("{:?}", msg);
            if let Some(ref filter_type) = filter {
                if !msg_type.contains(filter_type) {
                    continue;
                }
            }

            // Optional delay simulation with speed adjustment
            if realtime {
                if let Some(prev) = prev_time {
                    let delta = header.timestamp - prev;
                    if delta > 0 {
                        let adjusted_delta = (delta as f64 / speed as f64) as u64;
                        sleep(Duration::from_millis(adjusted_delta));
                    }
                }
                prev_time = Some(header.timestamp);
            }

            let mut out_buf = Vec::new();
            let fake_header = mavlink::MavHeader {
                sequence: header.sequence,
                system_id: header.system_id,
                component_id: header.component_id,
            };

            if let Err(e) = write_v2_msg(&mut out_buf, fake_header, &msg) {
                eprintln!("Failed to write message to buffer: {}", e);
                continue;
            }

            stream.write_all(&out_buf)?;
            println!("Replayed message: {:?}", msg);
        } else {
            eprintln!("Failed to decode message");
        }
    }

    println!("Replay complete.");
    Ok(())
}