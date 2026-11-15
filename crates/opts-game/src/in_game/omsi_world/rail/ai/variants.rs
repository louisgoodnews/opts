//! Reads the AI lists once: every rail vehicle with its coupling chain.

use super::*;

/// Reads the AI lists once and keeps every rail vehicle (`[type] 2`) with its coupling chain.
pub fn load_rail_variants(
    job: Option<ResMut<WorldLoadJob>>,
    mut pool: ResMut<RailAiPool>,
    mut bikes: ResMut<BikeAiPool>,
) {
    let Some(job) = job else { return };
    if pool.started {
        return;
    }
    pool.started = true;
    pool.rng = 0x7A41_C0DE;
    bikes.0.started = true;
    bikes.0.rng = 0xB1CE_0001;
    // The lists in force on the map date (Chrono group updates merged in).
    let Some(bytes) = job.data_bytes("ailists.cfg") else {
        return;
    };
    let lists = opts_core::formats::map::AiLists::parse(&bytes);
    for g in &lists.groups {
        pool.group_vehicles.insert(
            g.name.to_ascii_lowercase(),
            g.entries.iter().map(|e| e.vehicle.clone()).collect(),
        );
    }
    for d in &lists.depot_groups {
        pool.group_vehicles
            .insert(d.name.to_ascii_lowercase(), d.vehicles.clone());
    }
    let mut seen = std::collections::HashSet::new();
    // A map names its own fleet: the depot vehicles (EVAG's NF6D on the tram map) or groups called
    // after trams. The rest of the lists (U-Bahn cars, regional trains) are traffic of other
    // networks; they are used only when the map names no fleet of its own.
    let tram_groups = |name: &str| {
        let n = name.to_ascii_lowercase();
        ["tram", "strassenbahn", "variobahn", "stadtbahn"]
            .iter()
            .any(|k| n.contains(k))
    };
    let mut candidates: Vec<(bool, &str)> = lists
        .depot_vehicles
        .iter()
        .map(|v| (true, v.as_str()))
        .collect();
    for g in &lists.groups {
        let own = tram_groups(&g.name);
        candidates.extend(g.entries.iter().map(|e| (own, e.vehicle.as_str())));
    }
    let mut trains: Vec<(bool, TrainSpec)> = Vec::new();
    for (own, vehicle) in candidates {
        let f = vehicle.to_ascii_lowercase();
        let zug = f.ends_with(".zug");
        if !(zug || f.ends_with(".bus") || f.ends_with(".ovh")) || !seen.insert(f.clone()) {
            continue;
        }
        let loaded = if zug {
            load_zug_spec(&job, vehicle)
        } else {
            load_spec(&job, vehicle)
        };
        if let Some((spec, light)) = loaded {
            if !light {
                pool.rail_files.insert(vehicle.to_ascii_lowercase());
            }
            if light {
                bikes.0.specs.push(spec);
            } else {
                trains.push((own, spec));
            }
        }
    }
    // Depot buses are road vehicles: only a rail fleet of the map's own counts as "named".
    let any_own = trains.iter().any(|(own, _)| *own);
    pool.specs = trains
        .into_iter()
        .filter(|(own, _)| *own || !any_own)
        .map(|(_, spec)| spec)
        .collect();
    tracing::info!(
        trains = pool.specs.len(),
        riders = bikes.0.specs.len(),
        "rail vehicles and cycles in the AI lists"
    );
}
