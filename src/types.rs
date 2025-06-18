use mavlink::MavHeader;
use serde::{Serialize, Deserialize};

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

#[derive(Debug, Clone)]
pub struct LoggedMessage {
    pub timestamp: i64,
    pub message: mavlink::ardupilotmega::MavMessage,
    pub is_armed: bool,
    pub message_type: String,
}

#[derive(Serialize, Deserialize)]
pub struct BbinHeader {
    pub magic: [u8; 4], // "BBIN"
    pub version: u16,   // e.g., 1.0 as 10
    pub start_timestamp: i64, // Unix timestamp in milliseconds
}

#[derive(Serialize, Deserialize)]
pub struct BbinIndexEntry {
    pub message_type: String, // e.g., "GPS_RAW_INT"
    pub offset: u64,         // Byte offset in file
    pub timestamp: i64,      // Unix timestamp in milliseconds
}