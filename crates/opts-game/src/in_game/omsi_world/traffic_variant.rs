//! The vehicle types of a map's AI lists: what each vehicle file says about the vehicle (its
//! [`AiProfile`]), its model, and the trailing units of a coupled vehicle (M-0095,
//! `docs/src/architecture/ai-vehicle-config.md`).

use std::path::{Path, PathBuf};

use bevy::math::Vec2;
use opts_core::formats::constfile::ConstFile;
use opts_core::formats::vehicles::model::ModelConfig;
use opts_core::formats::vehicles::{AiKind, AiProfile, BusManifest, VehicleDomain};
use opts_sim::script::{load_vehicle_scripts, measure_ai_response, AiResponse};
use opts_traffic::{VehicleClass, VehicleKind, VehicleTraits};

use super::WorldLoadJob;

mod groups;
pub(super) use groups::*;

/// Most units (head and trailers) of one AI vehicle.
const MAX_UNITS: usize = 8;

/// One unit of an AI vehicle: the head, or a trailing unit coupled behind its predecessor.
pub(super) struct Unit {
    pub name: String,
    /// The vehicle file the unit was read from.
    pub bus_path: PathBuf,
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
    /// Where the first passenger door is, forward from the model origin (m), from the
    /// passenger cabin and paths files; `None` for a vehicle without them. A scheduled bus stops
    /// with this door at the stop.
    pub front_door_y: Option<f32>,
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
    let model_dir = super::mesh_dir::model_mesh_dir(&job.install, &dir, &cfg, &model);
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
    // Every `texture` folder, whatever the case (`Texture` and `texture` can sit side by side).
    let tex_dirs = crate::new_game_screen::repaints::texture_dirs(&dir, &model_dir);
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
        bus_path: path.to_path_buf(),
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
        sound: crate::vehicle_sound::ai_sound_set(job, &manifest, &dir, path),
        front_door_y: front_door_y(job, &manifest, &dir),
    };
    Ok((unit, manifest, dir))
}

/// The forward position (m, vehicle frame) of the first `[entry]` door of the vehicle's passenger
/// cabin, `None` without cabin, paths or entry.
fn front_door_y(job: &WorldLoadJob, manifest: &BusManifest, dir: &Path) -> Option<f32> {
    let read = |rel: Option<&str>| {
        rel.and_then(|r| job.install.find_in(dir, r))
            .and_then(|p| std::fs::read(p).ok())
    };
    let (cabin, paths) = (
        read(manifest.passengercabin_cfg.as_deref())?,
        read(manifest.paths_cfg.as_deref())?,
    );
    // Bevy axes: forward is -z.
    super::vehicle::doors::door_points_from_bytes(&cabin, &paths)
        .into_iter()
        .filter(|d| d.entry)
        .map(|d| -d.position.z)
        .reduce(f32::max)
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

#[cfg(test)]
mod tests;
