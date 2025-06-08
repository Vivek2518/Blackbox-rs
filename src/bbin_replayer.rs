use crate::types::{BbinHeader, LoggedMessageHeader};
use mavlink::ardupilotmega::MavMessage;
use mavlink::{read_v2_msg, write_v2_msg, MavHeader};
use std::fs::File;
use std::io::{self, BufReader, Read, Write};
use std::net::TcpStream;
use std::time::{Duration, Instant};
use bincode::deserialize_from;

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