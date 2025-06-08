
use crate::bbin_writer::BbinWriter;
use mavlink::{read_v2_msg, write_v2_msg, ardupilotmega::MavMessage};
use std::io::{self, Read, BufReader};
use std::net::TcpStream;
use chrono::Utc;

#[derive(Debug)]
pub struct BlackBoxerConfig {
    pub armed_only: bool,
    pub addr: String,
}

pub struct BlackBoxer {
    stream: TcpStream,
    is_armed: bool,
    config: BlackBoxerConfig,
}

impl BlackBoxer {
    pub fn new(config: BlackBoxerConfig) -> io::Result<Self> {
        println!("Connecting to {}", config.addr);
        let stream = TcpStream::connect(&config.addr)?;
        stream.set_nonblocking(true)?;
        println!("TCP connection established with {}", config.addr);
        Ok(BlackBoxer {
            stream,
            is_armed: false,
            config,
        })
    }

    pub fn capture_messages(&mut self) -> io::Result<()> {
        let mut reader = BufReader::new(&self.stream);
        let mut buf = [0u8; 512];
        let mut bbin_writer = BbinWriter::new(&format!("mavlink_log_{}.bbin", Utc::now().format("%Y%m%d_%H%M%S")))?;

        println!("Monitoring for arm/disarm events...");

        loop {
            match reader.read(&mut buf) {
                Ok(amt) if amt > 0 => {
                    let mut packet = &buf[..amt];
                    while !packet.is_empty() {
                        match read_v2_msg::<MavMessage, &[u8]>(&mut packet) {
                            Ok((header, msg)) => {
                                let timestamp = Utc::now();
                                match msg {
                                    MavMessage::HEARTBEAT(heartbeat) => {
                                        let new_armed = heartbeat.system_status == mavlink::ardupilotmega::MavState::MAV_STATE_ACTIVE;
                                        if new_armed != self.is_armed {
                                            self.is_armed = new_armed;
                                            println!("Vehicle {}armed", if new_armed { "" } else { "dis" });
                                        }
                                    }
                                    _ => {
                                        if !self.config.armed_only || self.is_armed {
                                            let mut raw_msg_bytes = Vec::new();
                                            write_v2_msg(&mut raw_msg_bytes, header, &msg)
                                                .map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;
                                            bbin_writer.write_message_raw(timestamp.timestamp_millis(), header, &raw_msg_bytes)?;
                                            println!("Captured message: {:?}", msg);
                                        }
                                    }
                                }
                            }
                            Err(e) => {
                                eprintln!("Failed to parse MAVLink message: {:?}", e);
                                break;
                            }
                        }
                    }
                }
                Ok(_) => {
                    std::thread::sleep(std::time::Duration::from_millis(10));
                }
                Err(ref e) if e.kind() == io::ErrorKind::WouldBlock => {
                    std::thread::sleep(std::time::Duration::from_millis(10));
                    continue;
                }
                Err(e) => {
                    eprintln!("TCP Read error: {:?}", e);
                    break;
                }
            }
        }

        bbin_writer.finalize()?;
        Ok(())
    }
}