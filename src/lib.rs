use mavlink::ardupilotmega::MavMessage;
use mavlink::MavHeader;
use std::io::{self, Read, BufReader, Write};
use std::net::TcpStream;
use std::time::Duration;
use chrono::{Utc};
use std::fs::File;
use serde::{Serialize, Deserialize};
use bincode::{serialize};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct LoggedMessageHeader {
    pub timestamp: i64, // milliseconds since UNIX epoch
    pub sequence: u8,
    pub system_id: u8,
    pub component_id: u8,
    pub msg_len: u16,
}

impl LoggedMessageHeader {
    pub fn from_mav_header(timestamp: i64, header: MavHeader, msg_len: usize) -> Self {
        Self {
            timestamp,
            sequence: header.sequence,
            system_id: header.system_id,
            component_id: header.component_id,
            msg_len: msg_len as u16,
        }
    }
}

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

#[derive(Serialize, Deserialize)]
struct BbinHeader {
    magic: [u8; 4], // "BBIN"
    version: u16,   // e.g., 1.0 as 10
    start_timestamp: i64, // Unix timestamp in milliseconds
}

#[derive(Serialize, Deserialize)]
struct BbinIndexEntry {
    message_type: String, // e.g., "GPS_RAW_INT"
    offset: u64,         // Byte offset in file
    timestamp: i64,      // Unix timestamp in milliseconds
}

struct BbinWriter {
    file: File,
    index: Vec<BbinIndexEntry>,
    current_offset: u64,
}

impl BbinWriter {
    fn new(filename: &str) -> io::Result<Self> {
        let mut file = File::create(filename)?;
        // Write header
        let header = BbinHeader {
            magic: *b"BBIN",
            version: 10, // 1.0
            start_timestamp: Utc::now().timestamp_millis(),
        };
        let header_bytes = serialize(&header).map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;
        file.write_all(&header_bytes)?;
        Ok(BbinWriter {
            file,
            index: Vec::new(),
            current_offset: header_bytes.len() as u64,
        })
    }

    fn write_message_raw(&mut self, timestamp: i64, header: MavHeader, raw_msg_bytes: &[u8]) -> io::Result<()> {
        let logged_header = LoggedMessageHeader::from_mav_header(timestamp, header, raw_msg_bytes.len());
        // Serialize header + timestamp with bincode
        let header_bytes = serialize(&logged_header).map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;
        self.file.write_all(&header_bytes)?;
        // Write raw MAVLink message bytes directly (not serialized)
        self.file.write_all(raw_msg_bytes)?;

        // Save index
        let msg_type = "MavMessage"; // Improve by extracting exact message type string if needed
        self.index.push(BbinIndexEntry {
            message_type: msg_type.to_string(),
            offset: self.current_offset,
            timestamp,
        });
        self.current_offset += (header_bytes.len() + raw_msg_bytes.len()) as u64;
        Ok(())
    }

    fn save_index(&mut self) -> io::Result<()> {
        let index_bytes = serialize(&self.index).map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;
        self.file.write_all(&index_bytes)?;
        // Write footer (simple length for now)
        let footer = self.current_offset;
        let footer_bytes = serialize(&footer).map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;
        self.file.write_all(&footer_bytes)?;
        Ok(())
    }

    /// Call this to finalize the file before closing
    pub fn finalize(&mut self) -> io::Result<()> {
        self.save_index()?;
        self.file.flush()?;
        Ok(())
    }
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
                        match mavlink::read_v2_msg::<MavMessage, &[u8]>(&mut packet) {
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
                                            mavlink::write_v2_msg(&mut raw_msg_bytes, header, &msg)
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
                    std::thread::sleep(Duration::from_millis(10));
                }
                Err(ref e) if e.kind() == io::ErrorKind::WouldBlock => {
                    std::thread::sleep(Duration::from_millis(10));
                    continue;
                }
                Err(e) => {
                    eprintln!("TCP Read error: {:?}", e);
                    break; // Exit loop on error to finalize
                }
            }
        }

        // Finalize the BBIN file on exiting the loop
        bbin_writer.finalize()?;
        Ok(())
    }
}
