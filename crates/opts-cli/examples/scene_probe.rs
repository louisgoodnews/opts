//! Prints a summary of the running game: state, scene counts, camera, simulation and recent warnings.
//!
//! ```text
//! cargo run -p opts-cli --example scene_probe
//! ```

mod common;

use serde_json::{json, Value};

fn main() {
    let mut c = common::connect();
    let mut get = |method: &str, params: Value| match c.call(method, params) {
        Ok(v) => v,
        Err(e) => common::die(method, &e),
    };
    let state = get("state", Value::Null);
    println!("app state : {}", state["app_state"]);
    println!(
        "camera    : {} (fps {})",
        state["camera_mode"], state["fps"]
    );
    if let Some(map) = state["map"].as_str().filter(|m| !m.is_empty()) {
        let scene = get("scene", Value::Null);
        println!(
            "map       : {map}, {} tiles resident",
            scene["tiles_loaded"]
        );
        println!(
            "entities  : {}, assets: {}",
            scene["entity_count"], scene["asset_count"]
        );
        let s = &scene["stats"];
        println!(
            "objects   : {} loaded, {} failed, {} protected",
            s["objects_loaded"], s["objects_failed"], s["objects_protected"]
        );
        println!(
            "splines   : {} loaded, {} failed",
            s["splines_loaded"], s["splines_failed"]
        );
    }
    let sim = get("sim.get", Value::Null);
    println!(
        "simulation: paused={} t={:.1}s x{}",
        sim["paused"],
        sim["sim_time"].as_f64().unwrap_or(0.0),
        sim["time_scale"]
    );
    let warnings = get("log.filter", json!({ "level": "warn", "n": 5 }));
    for line in warnings["lines"].as_array().into_iter().flatten() {
        println!("warning   : {} {}", line["target"], line["message"]);
    }
}
