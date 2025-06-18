use blackboxer::{BlackBoxer, BlackBoxerConfig, LoggedMessage};
use mavlink::Message;
use std::env;
use std::sync::{mpsc, Arc, atomic::{AtomicBool, Ordering}};
use std::thread;
use std::io::{self, Read};

fn main() -> std::io::Result<()> {
    let args: Vec<String> = env::args().collect();
    let armed_only = args.contains(&"--armed-only".to_string());
    let addr = args.get(1).map_or("127.0.0.1:14552".to_string(), |s| s.clone());

    let config = BlackBoxerConfig {
        armed_only,
        addr,
    };

    let (tx, rx) = mpsc::channel::<LoggedMessage>();
    let stop_flag = Arc::new(AtomicBool::new(false));
    let stop_flag_clone = stop_flag.clone();

    // Spawn the capture in a background thread
    thread::spawn(move || {
        let mut blackboxer = BlackBoxer::new(config).unwrap();
        blackboxer.capture_messages(tx, stop_flag_clone).unwrap();
    });

    // Simulate UI: print messages
    thread::spawn(move || {
        while let Ok(msg) = rx.recv() {
            println!("[UI] {} @ {} | Armed: {} | Type: {}", msg.message_type, msg.timestamp, msg.is_armed, msg.message_type);
        }
    });

    println!("Press Enter to stop...");
    let _ = io::stdin().read(&mut [0u8]).unwrap();
    stop_flag.store(true, Ordering::Relaxed);

    // Wait a bit to ensure clean shutdown
    thread::sleep(std::time::Duration::from_secs(2));
    println!("Stopped capture.");

    Ok(())
}
