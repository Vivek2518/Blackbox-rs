use blackboxer::{BlackBoxer, BlackBoxerConfig};
use std::env;

fn main() -> std::io::Result<()> {
    let args: Vec<String> = env::args().collect();
    let armed_only = args.contains(&"--armed-only".to_string());
    let addr = args.get(1).map_or("127.0.0.1:14552".to_string(), |s| s.clone());

    let config = BlackBoxerConfig {
        armed_only,
        addr,
    };

    let mut blackboxer = BlackBoxer::new(config)?;
    blackboxer.capture_messages()?;
    Ok(())
}