//! Watches one file and asks the running game to reload it whenever it changes — the edit/observe
//! loop of an author or an agent that rewrites a file.
//!
//! ```text
//! cargo run -p opts-cli --example reload_iterate -- "/path/to/Sceneryobjects/Pack/object.sco" 3
//! ```
//!
//! Saves three times (waiting at most 60 s each), printing the outcome of each reload. A broken save
//! prints the parser's message and keeps the previous content in the game.

mod common;

use std::path::PathBuf;
use std::time::{Duration, SystemTime};

use serde_json::json;

fn modified(path: &PathBuf) -> Option<SystemTime> {
    std::fs::metadata(path).and_then(|m| m.modified()).ok()
}

fn main() {
    let mut args = std::env::args().skip(1);
    let Some(file) = args.next().map(PathBuf::from) else {
        eprintln!("usage: reload_iterate <file> [times=3]");
        std::process::exit(1);
    };
    let times: usize = args.next().and_then(|s| s.parse().ok()).unwrap_or(3);
    let mut c = common::connect();
    println!(
        "watching {} — edit and save it ({times} times)",
        file.display()
    );
    let mut last = modified(&file);
    for round in 1..=times {
        let deadline = std::time::Instant::now() + Duration::from_secs(60);
        while modified(&file) == last {
            if std::time::Instant::now() > deadline {
                eprintln!("no change within 60 s");
                std::process::exit(1);
            }
            std::thread::sleep(Duration::from_millis(200));
        }
        last = modified(&file);
        match c.call("reload.asset", json!({ "path": file })) {
            Ok(v) => println!("#{round} reloaded: {}", v["result"]),
            Err(e) => println!("#{round} not applied: {e}"),
        }
        if let Ok(v) = c.call(
            "log.filter",
            json!({ "target": "opts_bevy::hot_reload", "n": 2 }),
        ) {
            for line in v["lines"].as_array().into_iter().flatten() {
                println!("   log: {} {}", line["level"], line["message"]);
            }
        }
    }
}
