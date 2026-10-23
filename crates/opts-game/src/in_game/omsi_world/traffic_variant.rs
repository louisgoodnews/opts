//! The vehicle types of a map's AI lists: what each vehicle file says about the vehicle (its
//! [`AiProfile`]), its model, and the trailing units of a coupled vehicle (M-0095,
//! `docs/src/architecture/ai-vehicle-config.md`).

use std::path::{Path, PathBuf};

use bevy::math::Vec2;
use opts_core::formats::constfile::ConstFile;
use opts_core::formats::map::{
    parse_unsched_trafficdens, parse_unsched_vehgroups, AiGroup, AiLists, GroupDensity,
};
use opts_core::formats::vehicles::model::ModelConfig;
use opts_core::formats::vehicles::{AiKind, AiProfile, BusManifest, VehicleDomain};
use opts_sim::script::{load_vehicle_scripts, measure_ai_response, AiResponse};
use opts_traffic::{VehicleClass, VehicleKind, VehicleTraits};

use super::WorldLoadJob;

/// Most units (head and trailers) of one AI vehicle.
const MAX_UNITS: usize = 8;

/// One unit of an AI vehicle: the head, or a trailing unit coupled behind its predecessor.
pub(super) struct Unit {
    pub name: String,
    pub model: ModelConfig,
    pub model_dir: PathBuf,
    pub tex_dirs: Vec<PathBuf>,
    pub profile: AiProfile,
    /// What the vehicle's own AI script makes of full throttle and full brake, measured when the
    /// variant loads; `None` when the script gives neither (it needs inputs the host does not
    /// feed yet, or the vehicle has none).
    pub response: Option<AiResponse>,
    /// Body length and the distance from the model origin to its front bumper (m), measured when
    /// the meshes first load.
    pub size: Option<(f32, f32)>,
    /// Trailing units: the predecessor's rear coupling and this unit's own joint, both in their
    /// own vehicle frames (x lateral, y longitudinal).
    pub hitch: Vec2,
    pub joint: Vec2,
    /// Longitudinal position of the rearmost axle in this unit's frame.
    pub rear_axle_y: f32,
    /// Longitudinal position of the frontmost axle in this unit's own frame, when the file
    /// declares axles.
    pub front_axle_y: Option<f32>,
    /// Joint limit on the angle against the predecessor (degrees).
    pub max_alpha_deg: f32,
    /// The `[CTC]` colour schemes of the model (default and `.cti` items); each vehicle picks one.
    pub repaints: crate::new_game_screen::repaints::VehicleRepaints,
    /// The unit is coupled the other way round (its model faces backwards).
    pub reversed: bool,
    /// The sounds the vehicle can play as AI traffic (M-0140); `None`: silent.
    pub sound: Option<std::sync::Arc<crate::vehicle_sound::AiSoundSet>>,
}

/// A vehicle type of the map's AI list: the head unit and its trailers.
pub(super) struct Variant {
    pub name: String,
    pub units: Vec<Unit>,
}

impl Variant {
    /// The traits the traffic model needs, from the profile of the head, the measured size of
    /// every unit (when known) and the masses and brake forces of all units.
    pub fn traits(&self) -> VehicleTraits {
        let head = &self.units[0].profile;
        let length: f64 = if self.units.iter().all(|u| u.size.is_some()) {
            self.units
                .iter()
                .filter_map(|u| u.size)
                .map(|(len, _)| f64::from(len))
                .sum()
        } else {
            f64::from(head.body_length_m.unwrap_or(4.4))
        };
        let masses: Option<Vec<f64>> = self
            .units
            .iter()
            .map(|u| u.profile.mass_kg.map(f64::from))
            .collect();
        let mass_kg = masses
            .as_ref()
            .map(|m| m.iter().sum())
            .or(head.mass_kg.map(f64::from));
        let brakes: Option<Vec<f64>> = self
            .units
            .iter()
            .map(|u| {
                u.response
                    .as_ref()
                    .map(|r| f64::from(r.brake_force))
                    .filter(|b| *b > 0.0)
                    .or(u.profile.brake_max_n.map(f64::from))
            })
            .collect();
        VehicleTraits {
            kind: head.kind.map(vehicle_kind),
            has_cabin: head.has_cabin,
            length,
            mass_kg,
            top_speed: head.top_speed_ms.map(f64::from),
            drive_torque: self.units[0]
                .response
                .as_ref()
                .map(|r| f64::from(r.peak_torque()))
                .filter(|t| *t > 0.0)
                .or(head.gas_max.map(f64::from)),
            wheel_radius: head.wheel_radius_m.map(f64::from),
            brake_force: brakes
                .map(|b| b.iter().sum())
                .or(head.brake_max_n.map(f64::from)),
            brake_performance: f64::from(head.brake_performance),
            rowdy: (f64::from(head.rowdy.0), f64::from(head.rowdy.1)),
            // The front axle sits `front_offset − y` behind the bumper; the wheelbase is the
            // head's own axle span (trailers follow through their joints).
            front_overhang: self.units[0]
                .size
                .zip(self.units[0].front_axle_y)
                .map(|((_, front), y)| f64::from(front - y)),
            wheelbase: head.wheelbase_m.map(f64::from),
        }
    }

    /// Class for lane restrictions (known before the meshes are measured).
    pub fn class(&self) -> VehicleClass {
        self.traits().class()
    }
}

fn vehicle_kind(k: AiKind) -> VehicleKind {
    match k {
        AiKind::Car => VehicleKind::Car,
        AiKind::Cab => VehicleKind::Cab,
        AiKind::Bus => VehicleKind::Bus,
        AiKind::Truck => VehicleKind::Truck,
    }
}

/// Reads one vehicle file: manifest, constant files, profile and model.
fn load_unit(job: &WorldLoadJob, path: &Path) -> Result<(Unit, BusManifest, PathBuf), String> {
    let bytes = std::fs::read(path).map_err(|e| e.to_string())?;
    let manifest = BusManifest::parse_bytes(&bytes).map_err(|e| e.to_string())?;
    let dir = path.parent().map(Path::to_path_buf).unwrap_or_default();
    let cfg_rel = manifest.model_cfg.clone().ok_or("no [model] entry")?;
    let cfg = job
        .install
        .find_in(&dir, &cfg_rel)
        .ok_or_else(|| format!("model config not found: {cfg_rel}"))?;
    let model = ModelConfig::parse_bytes(&std::fs::read(&cfg).map_err(|e| e.to_string())?)
        .map_err(|e| e.to_string())?;
    let model_dir = cfg.parent().map(Path::to_path_buf).unwrap_or_default();
    let consts: Vec<ConstFile> = manifest
        .constfiles
        .iter()
        .filter_map(|c| job.install.find_in(&dir, c))
        .filter_map(|p| std::fs::read(p).ok())
        .map(|b| ConstFile::parse(&b))
        .collect();
    let profile = AiProfile::from_manifest(&manifest, &consts);
    let response = measure_script(job, &manifest, &dir);
    let front_axle_y = manifest
        .axles
        .iter()
        .map(|a| a.longitudinal_m)
        .reduce(f32::max);
    let tex_dirs = vec![
        job.install
            .find_in(&dir, "texture")
            .unwrap_or_else(|| dir.join("texture")),
        model_dir.join("texture"),
    ];
    let default_livery = manifest
        .friendly_name
        .as_ref()
        .map(|f| f.livery.clone())
        .unwrap_or_default();
    let repaints = crate::new_game_screen::repaints::discover_for_model(
        &job.config.root,
        &dir,
        &model,
        &default_livery,
    );
    let unit = Unit {
        name: path
            .file_stem()
            .map(|s| s.to_string_lossy().into_owned())
            .unwrap_or_default(),
        model,
        model_dir,
        tex_dirs,
        profile,
        response,
        size: None,
        hitch: Vec2::ZERO,
        joint: Vec2::ZERO,
        rear_axle_y: 0.0,
        front_axle_y,
        max_alpha_deg: 52.5,
        repaints,
        reversed: false,
        sound: crate::vehicle_sound::ai_sound_set(job, &manifest, &dir),
    };
    Ok((unit, manifest, dir))
}

/// Runs the vehicle's own AI script and records its response to full throttle and full brake.
fn measure_script(job: &WorldLoadJob, manifest: &BusManifest, dir: &Path) -> Option<AiResponse> {
    if manifest.scripts.is_empty() {
        return None;
    }
    let (mut engine, _) = load_vehicle_scripts(manifest, |rel| job.install.find_in(dir, rel));
    Some(measure_ai_response(&mut engine)).filter(AiResponse::is_live)
}

/// The joint data of a trailing unit behind `parent`, as the player's chain computes it.
fn attach(unit: &mut Unit, parent: &BusManifest, own: &BusManifest) -> Option<()> {
    let reversed = parent.couple_back_reversed;
    let own_joint = if reversed {
        own.coupling_back.map(|c| -c)
    } else {
        own.coupling_front
    };
    let (b, f) = (parent.coupling_back?, own_joint?);
    let axles = own.axles.iter().map(|a| a.longitudinal_m);
    let rear = if reversed {
        -axles.fold(f32::NEG_INFINITY, f32::max)
    } else {
        axles.fold(f32::INFINITY, f32::min)
    };
    unit.hitch = Vec2::new(b.x, b.y);
    unit.joint = Vec2::new(f.x, f.y);
    unit.rear_axle_y = if rear.is_finite() { rear } else { f.y - 6.0 };
    unit.max_alpha_deg = own.coupling_front_limits.map_or(52.5, |l| l.max_alpha_deg);
    unit.reversed = reversed;
    Some(())
}

/// Loads a vehicle type of the AI list: the head and, through `[couple_back]`, its trailers.
///
/// A rail or air vehicle is refused (it does not belong on a road); a trailer that cannot be
/// loaded or coupled leaves the vehicle without it.
pub(super) fn load_variant(job: &WorldLoadJob, rel: &str) -> Result<Variant, String> {
    let path = job.install.find_file(rel).ok_or("file not found")?;
    let (head, mut manifest, mut dir) = load_unit(job, &path)?;
    if head.profile.domain != VehicleDomain::Road {
        return Err(format!("not a road vehicle ({:?})", head.profile.domain));
    }
    for note in &head.profile.notes {
        tracing::debug!(vehicle = %head.name, "{note}");
    }
    let name = head.name.clone();
    let mut units = vec![head];
    while units.len() < MAX_UNITS {
        let Some(next) = manifest.couple_back.clone() else {
            break;
        };
        let Some(path) = job.install.find_in(&dir, &next) else {
            tracing::debug!(vehicle = %name, trailer = %next, "AI trailer not found");
            break;
        };
        let Ok((mut unit, child, child_dir)) = load_unit(job, &path) else {
            tracing::debug!(vehicle = %name, trailer = %next, "AI trailer cannot be loaded");
            break;
        };
        if unit.profile.domain != VehicleDomain::Road
            || attach(&mut unit, &manifest, &child).is_none()
        {
            tracing::debug!(vehicle = %name, trailer = %next, "AI trailer not coupled");
            break;
        }
        let reversed = unit.reversed;
        units.push(unit);
        if reversed {
            break;
        }
        (manifest, dir) = (child, child_dir);
    }
    Ok(Variant { name, units })
}

/// A group of the map's AI lists that takes part in the traffic, with its density curve.
pub(super) struct PoolGroup {
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
pub(super) struct Slot {
    pub file: String,
    pub weight: f32,
    /// Index into the pool's groups.
    pub group: usize,
    /// Spawn attempts whose meshes were not ready (still decoding, or unreadable).
    pub fails: u32,
    pub state: SlotState,
}

pub(super) enum SlotState {
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
pub(super) fn participating<'a>(
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
pub(super) struct PoolSetup {
    pub groups: Vec<PoolGroup>,
    pub slots: Vec<Slot>,
    /// Default density of each `unsched_vehgroups.txt` group (`[1]` without the file).
    pub defaults: Vec<u32>,
    /// The vehicles of each depot group, by group name in lower case: slot index and how often
    /// the list names it (a repeat weighs the choice).
    pub depots: std::collections::HashMap<String, Vec<(usize, u32)>>,
}

/// Reads the map's AI lists and builds the groups and vehicle slots of the traffic pool.
pub(super) fn read_groups(job: &WorldLoadJob) -> PoolSetup {
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
                    fails: 0,
                    state: SlotState::Pending,
                }),
        );
    }
    if groups.is_empty() {
        tracing::error!(at = %opts_bevy::position_tag(), map = %job.config.map_dir.display(), "no AI group in ailists.cfg; no traffic");
    }
    // The depot groups: the vehicles of the timetable's tours (`.bus` and `.ovh`; trains are
    // the rail pool's). Their slots wait until a run asks for one.
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
                fails: 0,
                state: SlotState::Idle,
            });
        }
    }
    PoolSetup {
        groups,
        slots,
        defaults,
        depots,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use opts_core::formats::map::UnschedGroup;

    fn lists() -> AiLists {
        AiLists::parse(
            b"[aigroup_2]\nNormalCars\n\nvehicles\\a.bus\t5\n[end]\n\n[aigroup_2]\nTrucks\n\nvehicles\\b.bus\t1\n[end]\n\n\
[aigroup_2]\nEmergency\n\nvehicles\\c.bus\t1\n[end]\n",
        )
    }

    fn vg(name: &str, d: u32) -> UnschedGroup {
        UnschedGroup {
            name: name.into(),
            default_density: d,
        }
    }

    /// Every listed group takes part with its index in `unsched_vehgroups.txt` (the rule flag),
    /// also those with default `0`; groups `ailists.cfg` lacks are skipped.
    #[test]
    fn listed_groups_take_part_with_their_rule_index() {
        let l = lists();
        let g = participating(
            &l,
            &[
                vg("NormalCars", 1),
                vg("Taxi", 1),
                vg("trucks", 0),
                vg("Emergency", 0),
            ],
        );
        let names: Vec<(usize, &str)> = g.iter().map(|(i, g)| (*i, g.name.as_str())).collect();
        assert_eq!(names, [(0, "NormalCars"), (2, "Trucks"), (3, "Emergency")]);
    }

    #[test]
    fn without_usable_group_data_the_old_choice_stays() {
        let l = lists();
        assert_eq!(participating(&l, &[])[0].1.name, "NormalCars");
        assert_eq!(
            participating(&l, &[vg("Bicycles", 0)])[0],
            (0, &l.groups[0])
        );
        let only = AiLists::parse(b"[aigroup_2]\nTaxi\n\nvehicles\\t.bus\t1\n[end]\n");
        assert_eq!(participating(&only, &[])[0].1.name, "Taxi");
        assert!(participating(&AiLists::default(), &[]).is_empty());
    }

    #[test]
    fn traits_sum_the_units_of_a_coupled_vehicle() {
        let unit = |mass: f32, brake: f32, len: f32| Unit {
            name: "u".into(),
            model: ModelConfig::default(),
            model_dir: PathBuf::new(),
            tex_dirs: vec![],
            response: None,
            profile: {
                let mut p =
                    AiProfile::from_manifest(&BusManifest::parse("[mass]\n1\n").unwrap(), &[]);
                p.mass_kg = Some(mass);
                p.brake_max_n = Some(brake);
                p
            },
            size: Some((len, 1.0)),
            hitch: Vec2::ZERO,
            joint: Vec2::ZERO,
            rear_axle_y: 0.0,
            front_axle_y: None,
            max_alpha_deg: 52.5,
            repaints: Default::default(),
            reversed: false,
            sound: None,
        };
        let v = Variant {
            name: "bus".into(),
            units: vec![unit(11_000.0, 80_000.0, 11.0), unit(8000.0, 60_000.0, 7.0)],
        };
        let t = v.traits();
        assert_eq!(t.mass_kg, Some(19_000.0));
        assert_eq!(t.brake_force, Some(140_000.0));
        assert!((t.length - 18.0).abs() < 1e-9);
        assert_eq!(
            v.class(),
            VehicleClass::Truck,
            "no kind and no cabin: by length"
        );
    }
}
