//! The vehicle types of a map's AI lists: what each vehicle file says about the vehicle (its
//! [`AiProfile`]), its model, and the trailing units of a coupled vehicle (M-0095,
//! `docs/src/architecture/ai-vehicle-config.md`).

use std::path::PathBuf;

use bevy::math::Vec2;
use opts_core::formats::constfile::ConstFile;
use opts_core::formats::vehicles::model::ModelConfig;
use opts_core::formats::vehicles::{AiKind, AiProfile, VehicleDomain};
use opts_sim::script::{load_vehicle_scripts, AiResponse};
use opts_traffic::VehicleKind;

use super::WorldLoadJob;

mod groups;
pub(super) use groups::*;
mod load;
mod traits;
use load::load_unit;
mod couple;
use couple::{attach, front_door_y, measure_script};

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
    pub repaints: opts_repaints::VehicleRepaints,
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
fn vehicle_kind(k: AiKind) -> VehicleKind {
    match k {
        AiKind::Car => VehicleKind::Car,
        AiKind::Cab => VehicleKind::Cab,
        AiKind::Bus => VehicleKind::Bus,
        AiKind::Truck => VehicleKind::Truck,
    }
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
