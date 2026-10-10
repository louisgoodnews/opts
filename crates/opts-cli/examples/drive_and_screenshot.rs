//! Loads a map, drives forward for a few seconds and saves a screenshot.
//!
//! ```text
//! opts-game --cli-port 44201 &                 # start the game with the control server
//! cargo run -p opts-cli --example drive_and_screenshot -- "/path/to/OMSI 2/maps/BRT Berlin" 5
//! ```
//!
//! The screenshot goes to the reports directory; its path is printed.

mod common;

use std::time::Duration;

use serde_json::{json, Value};

fn main() {
    let mut args = std::env::args().skip(1);
    let Some(map) = args.next() else {
        eprintln!("usage: drive_and_screenshot <map folder> [seconds=5]");
        std::process::exit(1);
    };
    let seconds: u64 = args.next().and_then(|s| s.parse().ok()).unwrap_or(5);
    let mut c = common::connect();
    let mut call = |method: &str, params: Value| match c.call(method, params) {
        Ok(v) => v,
        Err(e) => common::die(method, &e),
    };

    println!("server: {}", call("version", Value::Null)["version"]);
    println!("loading {map} …");
    let loaded = call("scene.load_map", json!({ "path": map }));
    println!(
        "map up: {} tiles, vehicle {}",
        loaded["tiles_loaded"], loaded["vehicle"]
    );

    call("camera.set_mode", json!({ "mode": "external" }));
    call("sim.resume", Value::Null);
    call("input.key_hold", json!({ "key": "W" }));
    std::thread::sleep(Duration::from_secs(seconds));
    call("input.key_release", json!({ "key": "W" }));
    call("sim.pause", Value::Null);

    let shot = call("screenshot", Value::Null);
    println!("screenshot: {}", shot["path"]);
    let state = call("state", Value::Null);
    println!(
        "after {seconds} s: fps {}, camera {}",
        state["fps"], state["camera_mode"]
    );
}
