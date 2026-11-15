//! What a trailing unit and the vehicle's doors and script need from the vehicle before it.

use std::path::Path;

use opts_core::formats::vehicles::BusManifest;
use opts_sim::script::{measure_ai_response, AiResponse};

use super::*;

/// The forward position (m, vehicle frame) of the first `[entry]` door of the vehicle's passenger
/// cabin, `None` without cabin, paths or entry.
pub(super) fn front_door_y(job: &WorldLoadJob, manifest: &BusManifest, dir: &Path) -> Option<f32> {
    let read = |rel: Option<&str>| {
        rel.and_then(|r| job.install.find_in(dir, r))
            .and_then(|p| std::fs::read(p).ok())
    };
    let (cabin, paths) = (
        read(manifest.passengercabin_cfg.as_deref())?,
        read(manifest.paths_cfg.as_deref())?,
    );
    // Bevy axes: forward is -z.
    super::super::vehicle::doors::door_points_from_bytes(&cabin, &paths)
        .into_iter()
        .filter(|d| d.entry)
        .map(|d| -d.position.z)
        .reduce(f32::max)
}

/// Runs the vehicle's own AI script and records its response to full throttle and full brake.
pub(super) fn measure_script(job: &WorldLoadJob, manifest: &BusManifest, dir: &Path) -> Option<AiResponse> {
    if manifest.scripts.is_empty() {
        return None;
    }
    let (mut engine, _) = load_vehicle_scripts(manifest, |rel| job.install.find_in(dir, rel));
    Some(measure_ai_response(&mut engine)).filter(AiResponse::is_live)
}

/// The joint data of a trailing unit behind `parent`, as the player's chain computes it.
pub(super) fn attach(unit: &mut Unit, parent: &BusManifest, own: &BusManifest) -> Option<()> {
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
