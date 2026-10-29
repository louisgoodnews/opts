//! Measures the AI response (torque over speed, brake force) of the script of every vehicle of
//! the map AI lists and compares it with the shared car script's law:
//! `ai_response <OMSI dir>` (`SHOW=1` lists the vehicles whose script deviates from the law).
use std::path::{Path, PathBuf};

use opts_core::formats::map::AiLists;
use opts_core::formats::vehicles::BusManifest;
use opts_sim::script::{load_vehicle_scripts, measure_ai_response};

fn find_ci(base: &Path, rel: &str) -> Option<PathBuf> {
    let mut cur = base.to_path_buf();
    for comp in rel
        .split(['\\', '/'])
        .filter(|c| !c.is_empty() && *c != ".")
    {
        if comp == ".." {
            cur = cur.parent()?.to_path_buf();
            continue;
        }
        let direct = cur.join(comp);
        if direct.exists() {
            cur = direct;
            continue;
        }
        cur = std::fs::read_dir(&cur)
            .ok()?
            .flatten()
            .find(|e| e.file_name().to_string_lossy().eq_ignore_ascii_case(comp))?
            .path();
    }
    Some(cur)
}

#[derive(Default)]
struct Counts {
    total: usize,
    no_script: usize,
    dead: usize,
    follows: usize,
    deviates: usize,
}

fn evaluate_vehicle(
    vehicle_name: &str,
    bus: &BusManifest,
    path: &Path,
    root: &Path,
    show: bool,
    counts: &mut Counts,
) {
    counts.total += 1;
    if bus.scripts.is_empty() {
        counts.no_script += 1;
        return;
    }
    let dir = path.parent().unwrap_or(root).to_path_buf();
    let (mut engine, _) =
        load_vehicle_scripts(bus, |rel| find_ci(&dir, rel).or_else(|| find_ci(root, rel)));
    let r = measure_ai_response(&mut engine);
    if !r.is_live() {
        counts.dead += 1;
        if show {
            println!("DEAD      {vehicle_name}");
        }
        return;
    }
    let c = |n: &str| {
        let mut e2 = engine_const(bus, &dir, root, n);
        e2.take()
    };
    let (gas, reach, brems) = (
        c("AI_M_gas_max"),
        c("AI_v_max_erreich"),
        c("AI_F_brems_max"),
    );
    let law = |v: f32| match (gas, reach) {
        (Some(g), Some(rch)) => Some(g * (1.0 - v / rch)),
        _ => None,
    };
    let close = |a: f32, b: f32| (a - b).abs() <= 0.05 * b.abs().max(1.0);
    let ok = law(0.0).is_some_and(|l| close(r.torque_at(0.0), l))
        && law(50.0).is_some_and(|l| close(r.torque_at(50.0), l))
        && brems.is_some_and(|b| close(r.brake_force, b));
    if ok {
        counts.follows += 1;
    } else {
        counts.deviates += 1;
        if show {
            println!(
                "DEVIATES  {vehicle_name} | torque 0/50/100 km/h {:.0}/{:.0}/{:.0} (law {:?}), brake {:.0} (const {:?})",
                r.torque_at(0.0),
                r.torque_at(50.0),
                r.torque_at(100.0),
                law(0.0),
                r.brake_force,
                brems
            );
        }
    }
}

fn main() {
    let root = PathBuf::from(std::env::args().nth(1).expect("OMSI directory"));
    let show = std::env::var_os("SHOW").is_some();
    let mut seen = std::collections::BTreeSet::new();
    let mut counts = Counts::default();
    for map in std::fs::read_dir(root.join("maps"))
        .into_iter()
        .flatten()
        .flatten()
    {
        let Ok(bytes) = std::fs::read(map.path().join("ailists.cfg")) else {
            continue;
        };
        for group in AiLists::parse(&bytes).groups {
            for entry in group.entries {
                let lower = entry.vehicle.to_lowercase();
                if !(lower.ends_with(".bus") || lower.ends_with(".ovh")) || !seen.insert(lower) {
                    continue;
                }
                let Some(path) = find_ci(&root, &entry.vehicle) else {
                    continue;
                };
                let Ok(bus) = std::fs::read(&path).map(|b| BusManifest::parse_bytes(&b)) else {
                    continue;
                };
                let Ok(bus) = bus else { continue };
                evaluate_vehicle(&entry.vehicle, &bus, &path, &root, show, &mut counts);
            }
        }
    }
    println!(
        "{} vehicles: {} without scripts, {} whose script gives no torque or brake force, \
         {} follow the shared law, {} deviate",
        counts.total, counts.no_script, counts.dead, counts.follows, counts.deviates
    );
}

/// A scalar constant of a vehicle's constant files.
fn engine_const(bus: &BusManifest, dir: &Path, root: &Path, name: &str) -> Option<f32> {
    bus.constfiles
        .iter()
        .rev()
        .filter_map(|c| find_ci(dir, c).or_else(|| find_ci(root, c)))
        .filter_map(|p| std::fs::read(p).ok())
        .map(|b| opts_core::formats::constfile::ConstFile::parse(&b))
        .find_map(|f| {
            f.scalars
                .iter()
                .rev()
                .find(|(k, _)| k.eq_ignore_ascii_case(name))
                .map(|(_, v)| *v)
        })
}
