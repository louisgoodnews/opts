//! The depot groups of the map's AI lists: the timetable's vehicles, which wait until a run asks
//! for one (their slots start idle).

use opts_core::formats::map::AiLists;

use super::{PoolGroup, Slot, SlotState};

/// The depot groups: the vehicles of the timetable's tours (`.bus` and `.ovh`; trains are the
/// rail pool's). Their slots wait until a run asks for one. Returns the slots of each depot.
pub(super) fn add_depot_groups(
    lists: &AiLists,
    groups: &mut Vec<PoolGroup>,
    slots: &mut Vec<Slot>,
) -> std::collections::HashMap<String, Vec<(usize, u32)>> {
    let mut depots = std::collections::HashMap::new();
    for d in &lists.depot_groups {
        let mut files: Vec<(&str, u32)> = Vec::new();
        for f in &d.vehicles {
            let lower = f.to_ascii_lowercase();
            if !(lower.ends_with(".bus") || lower.ends_with(".ovh")) {
                continue;
            }
            match files.iter_mut().find(|(g, _)| g.eq_ignore_ascii_case(f)) {
                Some((_, n)) => *n += 1,
                None => files.push((f.as_str(), 1)),
            }
        }
        if files.is_empty() {
            continue;
        }
        let index = groups.len();
        groups.push(PoolGroup {
            name: d.name.clone(),
            density: None,
            density_group: 0,
            scheduled: true,
        });
        let entry = depots
            .entry(d.name.to_ascii_lowercase())
            .or_insert_with(Vec::new);
        for (file, n) in files {
            entry.push((slots.len(), n));
            slots.push(Slot {
                file: file.to_owned(),
                weight: 0.0,
                group: index,
                fleet: d
                    .fleet
                    .iter()
                    .filter(|(f, _)| f.eq_ignore_ascii_case(file))
                    .map(|(_, e)| e.clone())
                    .collect(),
                fails: 0,
                state: SlotState::Idle,
            });
        }
    }
    depots
}
