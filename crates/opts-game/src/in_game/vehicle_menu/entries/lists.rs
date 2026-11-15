//! The entry-point and bus-stop lists of the map, with unique labels.

use super::*;

/// Entry points of the map that no AI vehicle stands on, with unique labels.
pub(in crate::in_game::vehicle_menu) fn free_entries(
    job: &WorldLoadJob,
    index: &EntryIndex,
    traffic: &Query<&GlobalTransform, With<TrafficVehicle>>,
) -> Vec<(u32, String)> {
    let blockers: Vec<Vec3> = traffic.iter().map(GlobalTransform::translation).collect();
    let records = job.entry_records();
    let mut seen: HashMap<String, usize> = HashMap::new();
    let mut total: HashMap<&str, usize> = HashMap::new();
    for (_, n) in &records {
        *total.entry(n.as_str()).or_default() += 1;
    }
    let mut out = Vec::new();
    for (id, name) in &records {
        let n = seen.entry(name.clone()).or_default();
        *n += 1;
        let free = index.find(*id).is_none_or(|e| {
            let session = job.session_pos(e.tile, e.local);
            let p = opts_bevy::coords::omsi_to_bevy_pos(session);
            !blockers
                .iter()
                .any(|b| Vec2::new(b.x - p.x, b.z - p.z).length() < ENTRY_BLOCK_RADIUS_M)
        });
        if free {
            let label = if total.get(name.as_str()).copied().unwrap_or(1) > 1 {
                format!("{name} ({n})")
            } else {
                name.clone()
            };
            out.push((*id, label));
        }
    }
    out
}

/// The map's bus stops by name (`Busstops.cfg` of the timetables), a name that several stops
/// share numbered like the entry points.
pub(in crate::in_game::vehicle_menu) fn stop_list(job: &mut WorldLoadJob) -> Vec<(u32, String)> {
    let Some(tt) = job.timetables() else {
        return Vec::new();
    };
    let mut stops: Vec<(u32, String)> = tt
        .stops
        .iter()
        .filter(|s| !s.name.trim().is_empty())
        .map(|s| (s.id, s.name.trim().to_string()))
        .collect();
    stops.sort_by(|a, b| {
        a.1.to_lowercase()
            .cmp(&b.1.to_lowercase())
            .then(a.0.cmp(&b.0))
    });
    numbered(stops)
}

/// Labels that several rows share get their number: `Rathaus (1)`, `Rathaus (2)`.
pub(super) fn numbered(rows: Vec<(u32, String)>) -> Vec<(u32, String)> {
    let mut total: HashMap<String, usize> = HashMap::new();
    for (_, n) in &rows {
        *total.entry(n.clone()).or_default() += 1;
    }
    let mut seen: HashMap<String, usize> = HashMap::new();
    rows.into_iter()
        .map(|(id, name)| {
            if total[&name] > 1 {
                let n = seen.entry(name.clone()).or_default();
                *n += 1;
                (id, format!("{name} ({n})"))
            } else {
                (id, name)
            }
        })
        .collect()
}
