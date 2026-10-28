//! Discovery of depot (`.hof`) and timetable (`TTData`) files for a vehicle and map.

use super::*;

/// Finds the depot (`.hof`) for a vehicle. `map_names` are the map's names to match, most
/// telling first (its friendly name, then its folder name). For each name in turn: a file of the
/// vehicle's folder named like it, then one whose name contains it (or the other way round);
/// among several the one with the most routes wins, then the longer name: for `Berlin-Spandau`,
/// `Spandau.hof` (the map's lines) and not `Berlin.hof` (destinations only), which came first by
/// name. Next a file of the vehicle's folder sharing tokens with the names (`Hamburg109` and
/// `1_HH109_HHA.hof` share `109`): shared numbers before shared words, then tokens of the
/// vehicle's folder, then the fewest tokens neither names. Then the shared folders by name, and
/// last the first file of the vehicle's folder in name order. Files that do not parse are
/// skipped. The token step is OPTS's guess; OMSI lets the player pick the depot.
#[must_use]
pub fn find_depot(bus_dir: &Path, omsi_root: &Path, map_names: &[&str]) -> Option<HofFile> {
    let maps: Vec<String> = map_names
        .iter()
        .map(|n| n.trim().to_ascii_lowercase())
        .filter(|n| !n.is_empty())
        .collect();
    let parse = |p: &Path| {
        std::fs::read(p)
            .ok()
            .and_then(|b| HofFile::parse_bytes(&b).ok())
    };
    let hofs_in = |dir: &Path| -> Vec<(String, std::path::PathBuf)> {
        let mut v: Vec<_> = std::fs::read_dir(dir)
            .into_iter()
            .flatten()
            .flatten()
            .map(|e| e.path())
            .filter(|p| p.extension().is_some_and(|e| e.eq_ignore_ascii_case("hof")))
            .map(|p| {
                let stem = p
                    .file_stem()
                    .unwrap_or_default()
                    .to_string_lossy()
                    .to_ascii_lowercase();
                (stem, p)
            })
            .collect();
        v.sort();
        v
    };
    // The best parsed file among `candidates`, by `rank`, then the number of routes, then the
    // longer name.
    let best = |candidates: Vec<([i64; 5], &(String, std::path::PathBuf))>| {
        candidates
            .into_iter()
            .filter_map(|(rank, (stem, p))| {
                parse(p).map(|h| ((rank, h.routes.len(), stem.len()), h))
            })
            .max_by_key(|(key, _)| *key)
            .map(|(_, h)| h)
    };
    let own = hofs_in(bus_dir);
    for map in &maps {
        for (stem, p) in &own {
            if stem == map {
                if let Some(h) = parse(p) {
                    return Some(h);
                }
            }
        }
        let named = own
            .iter()
            .filter(|(stem, _)| stem.contains(map.as_str()) || map.contains(stem.as_str()))
            .map(|f| ([0; 5], f))
            .collect();
        if let Some(h) = best(named) {
            return Some(h);
        }
    }
    let wanted: std::collections::HashSet<String> =
        maps.iter().flat_map(|m| name_tokens(m)).collect();
    let folder = bus_dir
        .file_name()
        .map(|n| n.to_string_lossy().to_ascii_lowercase())
        .unwrap_or_default();
    let vehicle: std::collections::HashSet<String> = name_tokens(&folder).collect();
    let shared = own
        .iter()
        .filter_map(|f| {
            // Shared numbers (a line or edition) first, then shared words, then tokens of the
            // vehicle's folder (the operator: `HHA` in `HH109_Stadtbus_HHA`); then the fewest
            // numbers and words neither names (`1_HH109-2_HHA` and `1_HH109-VAN_HHA` lose to
            // `1_HH109_HHA` for `Hamburg109`).
            let mut rank = [0_i64; 5];
            for t in name_tokens(&f.0) {
                let number = t.chars().all(char::is_numeric);
                match (wanted.contains(&t), vehicle.contains(&t), number) {
                    (true, _, true) => rank[0] += 1,
                    (true, _, false) => rank[1] += 1,
                    (false, true, _) => rank[2] += 1,
                    (false, false, true) => rank[3] -= 1,
                    (false, false, false) => rank[4] -= 1,
                }
            }
            (rank[0] + rank[1] > 0).then_some((rank, f))
        })
        .collect();
    if let Some(h) = best(shared) {
        return Some(h);
    }
    if let Some(h) = own.iter().find_map(|(_, p)| parse(p)) {
        return Some(h);
    }
    for map in &maps {
        for dir in [
            omsi_root.join("Vehicles"),
            omsi_root.join("Vehicles/.hof Dateien"),
        ] {
            for (stem, p) in hofs_in(&dir) {
                if stem == *map || stem.contains(map.as_str()) {
                    if let Some(h) = parse(&p) {
                        return Some(h);
                    }
                }
            }
        }
    }
    None
}

/// The word and number tokens of a lower-case name: runs of letters (two or more) and runs of
/// digits, so `1_hh109-2_hha` gives `1`, `hh`, `109`, `2` and `hha`.
fn name_tokens(name: &str) -> impl Iterator<Item = String> + '_ {
    name.split(|c: char| !c.is_alphanumeric())
        .flat_map(|word| {
            let mut runs = Vec::new();
            let mut cur = String::new();
            for c in word.chars() {
                if !cur.is_empty() && cur.chars().all(char::is_numeric) != c.is_numeric() {
                    runs.push(std::mem::take(&mut cur));
                }
                cur.push(c);
            }
            runs.push(cur);
            runs
        })
        .filter(|t| t.chars().all(char::is_numeric) && !t.is_empty() || t.chars().count() >= 2)
}

/// The timetable files of a map: its `TTData` folder (any letter case), `None` when it has none.
#[must_use]
pub fn find_timetables(map_dir: &Path) -> Option<Timetables> {
    let dir = std::fs::read_dir(map_dir)
        .ok()?
        .flatten()
        .map(|e| e.path())
        .find(|p| {
            p.is_dir()
                && p.file_name()
                    .is_some_and(|n| n.to_string_lossy().eq_ignore_ascii_case("TTData"))
        })?;
    let t = Timetables::load_dir(&dir);
    (!t.is_empty()).then_some(t)
}
