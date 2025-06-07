use std::env;
use blackboxer::BbinReader;

fn main() -> std::io::Result<()> {
    let args: Vec<String> = env::args().collect();

    if args.len() < 2 {
        eprintln!("Usage: {} <file.bbin> [--show] [--filter=MSG_TYPE]", args[0]);
        return Ok(());
    }

    let file_path = &args[1];
    let show = args.contains(&"--show".to_string());
    let filter_arg = args.iter().find(|s| s.starts_with("--filter="));
    let filter_msg_type = filter_arg.map(|s| s.trim_start_matches("--filter="));

    let mut reader = BbinReader::new(file_path)?;
    reader.read_messages(filter_msg_type, show)?;

    Ok(())
}