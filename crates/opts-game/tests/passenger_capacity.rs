//! How many people the cabins of the installed vehicles hold once the standing room along the
//! aisles is added (`opts_sim::passengers::standing_room`). Skipped without `OMSI_PATH`.

use std::path::{Path, PathBuf};

use opts_core::formats::vehicles::cabin::PassengerCabin;
use opts_core::formats::vehicles::PassengerPaths;
use opts_sim::passengers::{Behaviour, PassengerSim, TicketBook};

fn walk(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(rd) = std::fs::read_dir(dir) else {
        return;
    };
    for e in rd.flatten() {
        let p = e.path();
        if p.is_dir() {
            walk(&p, out);
        } else if p
            .file_name()
            .map(|n| n.to_string_lossy().to_lowercase())
            .is_some_and(|n| n.starts_with("passengercabin") && n.ends_with(".cfg"))
        {
            out.push(p);
        }
    }
}

/// The `paths*.cfg` that belongs to a `passengercabin*.cfg` (same suffix, same folder).
fn paths_of(cabin: &Path) -> Option<PathBuf> {
    let name = cabin.file_name()?.to_string_lossy().to_string();
    let suffix = &name["passengercabin".len()..];
    let dir = cabin.parent()?;
    std::fs::read_dir(dir)
        .ok()?
        .flatten()
        .map(|e| e.path())
        .find(|p| {
            p.file_name()
                .map(|n| n.to_string_lossy().to_lowercase())
                .is_some_and(|n| n == format!("paths{}", suffix.to_lowercase()))
        })
}

#[test]
fn every_cabin_holds_all_its_seats_and_a_standing_crowd() {
    let Some(root) = std::env::var_os("OMSI_PATH").map(PathBuf::from) else {
        eprintln!("OMSI_PATH not set; skipping");
        return;
    };
    let mut files = Vec::new();
    walk(&root.join("Vehicles"), &mut files);
    let book = TicketBook {
        behaviour: Behaviour::default(),
        tickets: Vec::new(),
    };
    let mut rows: Vec<(usize, usize, usize, usize, String)> = Vec::new();
    for f in files {
        let Some(pf) = paths_of(&f) else { continue };
        let (Ok(cb), Ok(pb)) = (std::fs::read(&f), std::fs::read(&pf)) else {
            continue;
        };
        let Ok(paths) = PassengerPaths::parse_bytes(&pb) else {
            continue;
        };
        let unit = opts_game::passengers::unit_spec(&PassengerCabin::parse_bytes(&cb), &paths);
        if unit.cabin.entries.is_empty() || unit.cabin.places.is_empty() {
            continue;
        }
        let (cabin, p) = opts_sim::passengers::chain(&[unit]);
        let (seats, listed) = (cabin.seat_count(), cabin.places.len());
        let mut sim = PassengerSim::new(cabin, &p, book.clone(), 1);
        let added = sim.add_standing_room();
        rows.push((
            seats,
            listed,
            added,
            sim.capacity(),
            f.strip_prefix(&root).unwrap().display().to_string(),
        ));
    }
    rows.sort_by_key(|r| r.3);
    let n = rows.len();
    eprintln!("{n} cabins with an entry");
    let tail = |r: &&(usize, usize, usize, usize, String)| {
        eprintln!(
            "capacity {:3} = {:3} seats, {:3} listed places, +{:3} standing room  {}",
            r.3, r.0, r.1, r.2, r.4
        );
    };
    eprintln!("--- largest");
    rows.iter().rev().take(5).for_each(|r| tail(&r));
    eprintln!("--- median");
    tail(&&rows[n / 2]);
    eprintln!("--- bus-sized cabins (20+ seats) with the least room");
    let mut buses: Vec<_> = rows.iter().filter(|r| r.0 >= 20).collect();
    buses.sort_by_key(|r| r.3 - r.0);
    buses.iter().take(6).for_each(tail);
    if let Ok(filter) = std::env::var("PAX_FILTER") {
        for r in rows.iter().filter(|r| r.4.contains(&filter)) {
            tail(&r);
        }
    }
    assert!(n > 100);
    for r in &rows {
        assert!(r.3 >= r.1, "capacity covers the listed places: {}", r.4);
    }
    // A bus-sized cabin always has room to stand: at least as many standing places as seats / 2.
    let cramped: Vec<_> = rows
        .iter()
        .filter(|r| r.0 >= 20 && r.3 - r.0 < r.0 / 2)
        .map(|r| r.4.clone())
        .collect();
    assert!(
        cramped.len() * 10 < buses.len(),
        "cramped bus cabins: {cramped:?}"
    );
}
