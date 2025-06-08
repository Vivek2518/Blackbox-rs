use crate::types::{BbinHeader, BbinIndexEntry, LoggedMessageHeader};
use mavlink::MavHeader;
use std::fs::File;
use std::io::{self, Write};
use chrono::Utc;
use bincode::serialize;

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