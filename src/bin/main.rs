use blackboxer::{BlackBoxer, BlackBoxerConfig, LoggedMessage};
use mavlink::Message;
use std::env;
use std::sync::mpsc;
use std::thread;

fn main() -> std::io::Result<()> {
    let args: Vec<String> = env::args().collect();
    let armed_only = args.contains(&"--armed-only".to_string());
    let addr = args.get(1).map_or("127.0.0.1:14552".to_string(), |s| s.clone());

    let config = BlackBoxerConfig {
        armed_only,
        addr,
    };

    let (tx, rx) = mpsc::channel::<LoggedMessage>();

    // Spawn thread to receive and print messages (simulate UI)
   thread::spawn(move || {
    while let Ok(msg) = rx.recv() {
        println!(
            "[UI DEBUG] {} @ {}\n    Data: {:?}",
            msg.message.message_name(),
            msg.timestamp,
            msg.message
        );
    }
});


    let mut blackboxer = BlackBoxer::new(config)?;
    blackboxer.capture_messages(tx)?;

    Ok(())
}
