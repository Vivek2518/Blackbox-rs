use crate::types::{BbinHeader, LoggedMessage, LoggedMessageHeader};
use mavlink::ardupilotmega::MavMessage;
use mavlink::read_v2_msg;
use std::fs::File;
use std::io::{self, BufReader, Read};
use bincode::deserialize_from;

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
            match read_v2_msg::<MavMessage, _>(&mut packet) {
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

    pub fn read_and_collect(&mut self, filter_msg_type: Option<&str>) -> io::Result<Vec<LoggedMessage>> {
        let mut messages = Vec::new();

        loop {
            let header: LoggedMessageHeader = match deserialize_from(&mut self.reader) {
                Ok(h) => h,
                Err(e) => {
                    match *e {
                        bincode::ErrorKind::Io(ref io_err) if io_err.kind() == io::ErrorKind::UnexpectedEof => {
                            break;
                        }
                        _ => return Err(io::Error::new(io::ErrorKind::InvalidData, e)),
                    }
                }
            };

            let mut msg_buf = vec![0u8; header.msg_len as usize];
            self.reader.read_exact(&mut msg_buf)?;

            let mut packet: &[u8] = &msg_buf;
            match read_v2_msg::<MavMessage, _>(&mut packet) {
                Ok((_hdr, msg)) => {
                    let msg_type_str = format!("{:?}", msg);
                    if let Some(filter) = filter_msg_type {
                        if !msg_type_str.contains(filter) {
                            continue;
                        }
                    }
                    messages.push(LoggedMessage {
                        timestamp: header.timestamp,
                        message: msg,
                        is_armed: false,
                        message_type: msg_type_str,
                    });
                }
                Err(e) => {
                    eprintln!("Failed to decode MAVLink message: {:?}", e);
                }
            }
        }

        Ok(messages)
    }
}