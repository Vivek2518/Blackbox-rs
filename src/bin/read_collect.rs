use blackboxer::BbinReader;
use std::env;

fn main() -> std::io::Result<()> {
    let args: Vec<String> = env::args().collect();

    if args.len() < 2 {
        eprintln!("Usage: {} <file.bbin> [--filter=MSG_TYPE]", args[0]);
        return Ok(());
    }

    let file_path = &args[1];
    let filter_arg = args.iter().find(|s| s.starts_with("--filter="));
    let filter = filter_arg.map(|s| s.trim_start_matches("--filter="));

    let mut reader = BbinReader::new(file_path)?;
    let messages = reader.read_and_collect(filter)?;

    for msg in messages {
        println!("{} -> {:?}", msg.timestamp, msg.message);
    }

    Ok(())
}
