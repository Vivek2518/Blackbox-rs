use std::env;
use blackboxer::BbinReplayer;

fn main() -> std::io::Result<()> {
    let args: Vec<String> = env::args().collect();

    if args.len() < 3 {
        eprintln!("Usage: {} <file.bbin> <tcp_target> [--filter=MSG_TYPE] [--realtime] [--speed=VALUE]", args[0]);
        return Ok(());
    }

    let file_path = &args[1];
    let target = &args[2];
    let filter_arg = args.iter().find(|a| a.starts_with("--filter="));
    let filter = filter_arg.map(|s| s.trim_start_matches("--filter="));
    let realtime = args.contains(&"--realtime".to_string());
    let speed_arg = args.iter().find(|a| a.starts_with("--speed="));
    let speed: f32 = speed_arg
        .map(|s| s.trim_start_matches("--speed="))
        .and_then(|s| s.parse().ok())
        .unwrap_or(1.0);

    let mut replayer = BbinReplayer::new(file_path, target)?;
    replayer.replay_messages(filter, realtime, speed)?;

    Ok(())
}