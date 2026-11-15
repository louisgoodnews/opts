//! Activation reproduces the masks cached in `timeline.prt.cfg` (Chrono, `docs/src/formats/map/chrono.md`).

use std::collections::HashMap;
use std::path::PathBuf;

use opts_omsi::formats::chrono::{ChronoDate, ChronoSet};
use opts_omsi::formats::text::decode_bytes;

use super::maps_with_chrono;

/// `(rel path below Chrono/, lower-case)` of a `timeline.prt.cfg` path such as
/// `maps\Berlin-Spandau\Chrono\0100_Neuer13N\Chrono.cfg`.
fn rel_of_timeline_path(p: &str) -> Option<String> {
    let lower = p.replace('\\', "/").to_lowercase();
    let start = lower.find("/chrono/")? + "/chrono/".len();
    let rest = lower[start..].strip_suffix("/chrono.cfg")?;
    Some(rest.to_string())
}

#[test]
fn activation_reproduces_the_cached_timeline_masks() {
    let Some(root) = std::env::var_os("OMSI_PATH").map(PathBuf::from) else {
        eprintln!("OMSI_PATH not set; skipping corpus test");
        return;
    };
    let (mut rows, mut equal, mut maps, mut stale) = (0, 0, 0, 0);
    for map in maps_with_chrono(&root) {
        let Ok(bytes) = std::fs::read(map.join("timeline.prt.cfg")) else {
            continue;
        };
        let text = decode_bytes(&bytes);
        let lines: Vec<&str> = text.lines().map(str::trim).collect();
        let Some(hash) = lines.iter().position(|l| *l == "#") else {
            continue;
        };
        let set = ChronoSet::scan(&map);
        let by_rel: HashMap<String, usize> = set
            .entries()
            .iter()
            .enumerate()
            .map(|(i, e)| (e.rel.to_lowercase(), i))
            .collect();
        let columns: Option<Vec<usize>> = lines[..hash]
            .iter()
            .step_by(2)
            .map(|p| rel_of_timeline_path(p).and_then(|r| by_rel.get(&r).copied()))
            .collect();
        let Some(columns) = columns else {
            stale += 1;
            continue;
        };
        maps += 1;
        for pair in lines[hash + 1..].chunks(2) {
            let [day, mask] = pair else { continue };
            let Ok(day) = day.parse::<i64>() else {
                continue;
            };
            // Day 1 = 2000-01-01; the mask holds up to the day before the boundary.
            let date = ChronoDate::from_unix_days(10_957 + day - 1);
            let active = set.active(date);
            let ours: String = columns
                .iter()
                .map(|&i| if active.contains(i) { '1' } else { '0' })
                .collect();
            rows += 1;
            if ours == *mask {
                equal += 1;
            } else {
                eprintln!("{}: {date}: cached {mask}, ours {ours}", map.display());
            }
        }
    }
    eprintln!(
        "timeline masks: {equal} of {rows} rows equal on {maps} maps ({stale} with stale paths)"
    );
    if rows > 0 {
        assert!(maps >= 30, "{maps} maps");
        assert!(equal * 100 >= rows * 99, "{equal} of {rows}");
    }
}
