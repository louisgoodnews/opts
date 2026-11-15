//! Measures the AI traffic density model around the entry points of real maps, for calibrating
//! `L0` (metres of lane per vehicle at density 1) against the former fixed target.
//!
//! Usage: `cargo run --release -p opts-traffic --example density_census -- <map dir>...`
//!
//! For each entry point (up to five per map) on the 3 × 3 tiles around it, at 12:00 on a weekday:
//! * `old`: the former target `min(30, resident road length / 250 m × Σ group factors)`, factors
//!   clamped to `0..=2`, only groups with a default density of 1 or more;
//! * `S`: `Σ_groups Σ_lanes length inside 320 m × D(lane, group) × f_group`, the new model's
//!   numerator (target = `S / L0`).

mod census;
mod files;
mod tiles;
mod traffic;
mod weights;

use std::path::PathBuf;

use census::census;

fn main() {
    for arg in std::env::args().skip(1) {
        let map_dir = PathBuf::from(arg);
        let name = map_dir
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_default();
        for (ep, km, old, s) in census(&map_dir) {
            println!(
                "{name:<28} {ep:<32} resident {km:>6.1} km  old {old:>5.1}  S {s:>8.0}  S/30 {:>6.0}  L0=S/old {:>6.0}",
                s / 30.0,
                if old > 0.0 { s / old } else { 0.0 }
            );
        }
    }
}
