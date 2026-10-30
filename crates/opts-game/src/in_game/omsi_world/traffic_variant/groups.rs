//! AI traffic pool groups and vehicle slots parsed from map AI lists.

use opts_core::formats::map::{
    parse_unsched_trafficdens, parse_unsched_vehgroups, AiGroup, AiLists, GroupDensity,
};

use super::super::WorldLoadJob;
use super::Variant;

/// A group of the map's AI lists that takes part in the traffic, with its density curve.
pub(crate) struct PoolGroup {
    pub name: String,
    /// The group's `unsched_trafficdens.txt` entry (factor and curves); `None`: the map's
    /// `global.cfg` `[trafficdensity_road]` curve applies.
    pub density: Option<GroupDensity>,
    /// 0-based index in `unsched_vehgroups.txt`: the `flag` of the `[rule] trafficdensity`
    /// records that address this group.
    pub density_group: usize,
    /// A depot group of the AI lists (`[aigroup_depot]`): its vehicles drive timetable runs and
    /// never as unscheduled traffic.
    pub scheduled: bool,
}

/// A vehicle entry of a group.
pub(crate) struct Slot {
    pub file: String,
    pub weight: f32,
    /// Index into the pool's groups.
    pub group: usize,
    /// The depot fleet lines of this vehicle (`number`, `plate`, `repaint`): a scheduled run
    /// takes one and wears its repaint. Empty for the groups of unscheduled traffic.
    pub fleet: Vec<opts_core::formats::map::FleetEntry>,
    /// Spawn attempts whose meshes were not ready (still decoding, or unreadable).
    pub fails: u32,
    pub state: SlotState,
}

pub(crate) enum SlotState {
    /// A vehicle of a depot group that nothing has asked for yet: parsed when a run needs it.
    Idle,
    /// Not parsed yet.
    Pending,
    Ready(Box<Variant>),
    Failed,
}

/// The groups that take part in the traffic, each with its index in `unsched_vehgroups.txt`:
/// every group listed there that `ailists.cfg` defines, whatever its default density (a group
/// with default `0` drives only where a rule gives it a density, e.g. Berlin-Spandau's `GDRCars`);
/// without usable group data `NormalCars`, else the first group, as group `0`.
pub(crate) fn participating<'a>(
    lists: &'a AiLists,
    vehgroups: &[opts_core::formats::map::UnschedGroup],
) -> Vec<(usize, &'a AiGroup)> {
    let by_name = |n: &str| lists.groups.iter().find(|g| g.name.eq_ignore_ascii_case(n));
    let mut out: Vec<(usize, &AiGroup)> = Vec::new();
    for (i, g) in vehgroups.iter().enumerate() {
        if let Some(found) = by_name(&g.name) {
            if !out.iter().any(|(_, o)| std::ptr::eq(*o, found)) {
                out.push((i, found));
            }
        }
    }
    if out.is_empty() {
        out.extend(
            by_name("NormalCars")
                .or_else(|| lists.groups.first())
                .map(|g| (0, g)),
        );
    }
    out
}

/// What the traffic pool needs from the map's AI lists.
pub(crate) struct PoolSetup {
    pub groups: Vec<PoolGroup>,
    pub slots: Vec<Slot>,
    /// Default density of each `unsched_vehgroups.txt` group (`[1]` without the file).
    pub defaults: Vec<u32>,
    /// The vehicles of each depot group, by group name in lower case: slot index and how often
    /// the list names it (a repeat weighs the choice).
    pub depots: std::collections::HashMap<String, Vec<(usize, u32)>>,
}

/// The depot groups: the vehicles of the timetable's tours (`.bus` and `.ovh`; trains are the
/// rail pool's). Their slots wait until a run asks for one. Returns the slots of each depot.
fn add_depot_groups(
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

/// Reads the map's AI lists and builds the groups and vehicle slots of the traffic pool.
pub(crate) fn read_groups(job: &WorldLoadJob) -> PoolSetup {
    // The files in force on the map date (Chrono replacements and group updates).
    let read = |name: &str| job.data_bytes(name);
    let lists = read("ailists.cfg")
        .map(|b| AiLists::parse(&b))
        .unwrap_or_default();
    let vehgroups = read("unsched_vehgroups.txt")
        .map(|b| parse_unsched_vehgroups(&b))
        .unwrap_or_default();
    let densities = read("unsched_trafficdens.txt")
        .map(|b| parse_unsched_trafficdens(&b))
        .unwrap_or_default();
    let defaults: Vec<u32> = if vehgroups.is_empty() {
        vec![1]
    } else {
        vehgroups.iter().map(|g| g.default_density).collect()
    };
    let (mut groups, mut slots) = (Vec::new(), Vec::new());
    for (density_group, g) in participating(&lists, &vehgroups) {
        let density = densities
            .iter()
            .find(|d| d.name.eq_ignore_ascii_case(&g.name))
            .cloned();
        tracing::info!(
            group = %g.name,
            density_group,
            default = defaults.get(density_group).copied().unwrap_or(0),
            factor = density.as_ref().map(|d| d.factor),
            vehicles = g.entries.len(),
            "AI traffic group chosen"
        );
        let index = groups.len();
        groups.push(PoolGroup {
            name: g.name.clone(),
            density,
            density_group,
            scheduled: false,
        });
        slots.extend(
            g.entries
                .iter()
                .filter(|e| {
                    let f = e.vehicle.to_ascii_lowercase();
                    f.ends_with(".bus") || f.ends_with(".ovh")
                })
                .map(|e| Slot {
                    file: e.vehicle.clone(),
                    weight: e.weight.unwrap_or(1.0).max(0.0),
                    group: index,
                    fleet: Vec::new(),
                    fails: 0,
                    state: SlotState::Pending,
                }),
        );
    }
    if groups.is_empty() {
        tracing::error!(at = %opts_bevy::position_tag(), map = %job.config.map_dir.display(), "no AI group in ailists.cfg; no traffic");
    }
    let depots = add_depot_groups(&lists, &mut groups, &mut slots);
    PoolSetup {
        groups,
        slots,
        defaults,
        depots,
    }
}
