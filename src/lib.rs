use mavlink::ardupilotmega::MavMessage;
use mavlink::{MavHeader, read_v2_msg, write_v2_msg};
use std::io::{self, Read, BufReader, Write};
use std::net::TcpStream;
use std::time::{Duration, Instant};
use chrono::{Utc};
use std::fs::File;
use serde::{Serialize, Deserialize};
use bincode::{serialize, deserialize_from};

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

pub struct BbinWriter {
    file: File,
    index: Vec<BbinIndexEntry>,
    current_offset: u64,
}

impl BbinWriter {
    pub fn new(filename: &str) -> io::Result<Self> {
        let mut file = File::create(filename)?;
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

    pub fn write_message_raw(&mut self, timestamp: i64, header: MavHeader, raw_msg_bytes: &[u8]) -> io::Result<()> {
        let logged_header = LoggedMessageHeader::from_mav_header(timestamp, header, raw_msg_bytes.len());
        let header_bytes = serialize(&logged_header).map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;
        self.file.write_all(&header_bytes)?;
        self.file.write_all(raw_msg_bytes)?;
        let msg_type = "MavMessage"; // Improve by extracting exact message type string if needed
        self.index.push(BbinIndexEntry {
            message_type: msg_type.to_string(),
            offset: self.current_offset,
            timestamp,
        });
        self.current_offset += (header_bytes.len() + raw_msg_bytes.len()) as u64;
        Ok(())
    }

    pub fn save_index(&mut self) -> io::Result<()> {
        let index_bytes = serialize(&self.index).map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;
        self.file.write_all(&index_bytes)?;
        let footer = self.current_offset;
        let footer_bytes = serialize(&footer).map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;
        self.file.write_all(&footer_bytes)?;
        Ok(())
    }

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
                    break;
                }
            }
        }

        bbin_writer.finalize()?;
        Ok(())
    }
}

pub struct BbinReader {
    reader: BufReader<File>,
}

impl BbinReader {
    pub fn new(file_path: &str) -> io::Result<Self> {
        let file = File::open(file_path)?;
        let mut reader = BufReader::new(file);
        let header: BbinHeader = deserialize_from(&mut reader)
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
        if header.magic != *b"BBIN" {
            return Err(io::Error::new(io::ErrorKind::InvalidData, "Invalid BBIN file magic"));
        }
        Ok(BbinReader { reader })
    }

    pub fn read_messages(&mut self, filter_msg_type: Option<&str>, show: bool) -> io::Result<()> {
        loop {
            let header: LoggedMessageHeader = match deserialize_from(&mut self.reader) {
                Ok(h) => h,
                Err(e) => {
                    match *e {
                        bincode::ErrorKind::Io(ref io_err) if io_err.kind() == io::ErrorKind::UnexpectedEof => {
                            if show {
                                println!("Reached EOF or incomplete message, stopping.");
                            }
                            break;
                        }
                        _ => {
                            return Err(io::Error::new(io::ErrorKind::InvalidData, e));
                        }
                    }
                }
            };

            let mut msg_buf = vec![0u8; header.msg_len as usize];
            self.reader.read_exact(&mut msg_buf)?;

            let mut packet: &[u8] = &msg_buf;
            match mavlink::read_v2_msg::<MavMessage, _>(&mut packet) {
                Ok((_hdr, msg)) => {
                    let msg_type_str = format!("{:?}", msg);
                    if show {
                        if let Some(filter) = filter_msg_type {
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
}

// New BbinReplayer implementation for replaying .bbin files
pub struct BbinReplayer {
    reader: BufReader<File>,
    stream: TcpStream,
}

impl BbinReplayer {
    pub fn new(file_path: &str, target: &str) -> io::Result<Self> {
        let file = File::open(file_path)?;
        let mut reader = BufReader::new(file);
        let header: BbinHeader = deserialize_from(&mut reader)
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
        if header.magic != *b"BBIN" {
            return Err(io::Error::new(io::ErrorKind::InvalidData, "Invalid BBIN file magic"));
        }
        let stream = TcpStream::connect(target)?;
        println!("Connected to {}", target);
        Ok(BbinReplayer { reader, stream })
    }

    pub fn replay_messages(&mut self, filter_msg_type: Option<&str>, realtime: bool, speed: f32) -> io::Result<()> {
        if speed <= 0.0 {
            return Err(io::Error::new(io::ErrorKind::InvalidInput, "Speed must be a positive value"));
        }

        let mut prev_time: Option<i64> = None;
        let _start = Instant::now();

        loop {
            let header: LoggedMessageHeader = match deserialize_from(&mut self.reader) {
                Ok(h) => h,
                Err(e) => {
                    match *e {
                        bincode::ErrorKind::Io(ref io_err) if io_err.kind() == io::ErrorKind::UnexpectedEof => {
                            println!("Reached end of file or encountered error.");
                            break;
                        }
                        _ => {
                            return Err(io::Error::new(io::ErrorKind::InvalidData, e));
                        }
                    }
                }
            };

            let mut msg_buf = vec![0u8; header.msg_len as usize];
            if let Err(e) = self.reader.read_exact(&mut msg_buf) {
                eprintln!("Failed to read message bytes: {}", e);
                break;
            }

            let mut packet = &msg_buf[..];
            match read_v2_msg::<MavMessage, _>(&mut packet) {
                Ok((_, msg)) => {
                    let msg_type = format!("{:?}", msg);
                    if let Some(filter) = filter_msg_type {
                        if !msg_type.contains(filter) {
                            continue;
                        }
                    }

                    if realtime {
                        if let Some(prev) = prev_time {
                            let delta = header.timestamp - prev;
                            if delta > 0 {
                                let adjusted_delta = (delta as f64 / speed as f64) as u64;
                                std::thread::sleep(Duration::from_millis(adjusted_delta));
                            }
                        }
                        prev_time = Some(header.timestamp);
                    }

                    let mut out_buf = Vec::new();
                    let fake_header = MavHeader {
                        sequence: header.sequence,
                        system_id: header.system_id,
                        component_id: header.component_id,
                    };

                    if let Err(e) = write_v2_msg(&mut out_buf, fake_header, &msg) {
                        eprintln!("Failed to write message to buffer: {}", e);
                        continue;
                    }

                    self.stream.write_all(&out_buf)?;
                    println!("Replayed message: {:?}", msg);
                }
                Err(e) => {
                    eprintln!("Failed to decode message: {:?}", e);
                }
            }
        }

        println!("Replay complete.");
        Ok(())
    }
}