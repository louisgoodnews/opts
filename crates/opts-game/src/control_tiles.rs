//! Control protocol of the tile loading controls (M-0080): `scene.tiles` reports which tiles are
//! resident, the mode (`all` or `radius`), the effective radius and what the automatic radius
//! would be on this machine. The modes are set through `settings.set` on
//! `gameplay.load_all_tiles` and `gameplay.tile_radius`.

use bevy::prelude::*;
use opts_bevy::control::state::{ControlRegistry, Outcome};
use opts_cli::protocol::{code, RpcError};
use opts_hardware::recommend::MapStats;
use serde_json::{json, Value};

use crate::in_game::omsi_world::WorldLoadJob;
use crate::in_game::tile_loading::{self, TileChoice};
use crate::settings::GameSettings;

/// Registers `scene.tiles`.
pub fn register(registry: &mut ControlRegistry) {
    registry.register("scene.tiles", tiles);
}

fn tiles(world: &mut World, _: &Value) -> Outcome {
    let Some(job) = world.get_resource::<WorldLoadJob>() else {
        return Outcome::err(RpcError::new(
            code::NOT_READY,
            "no session has been started",
        ));
    };
    let choice = world
        .get_resource::<GameSettings>()
        .map(TileChoice::from_settings)
        .unwrap_or_default();
    let profile = world
        .get_resource::<crate::hardware::HardwareState>()
        .and_then(|h| h.profile.as_ref());
    let map_radius = job.map_radius();
    let stats = MapStats {
        max_radius: map_radius,
        ..MapStats::default()
    };
    let recommended = tile_loading::recommended(profile, &stats).clamp(1, map_radius.max(1));
    let mode = job.tile_mode();
    let resident = job.resident_tiles();
    Outcome::ok(json!({
        "mode": mode.name(),
        "radius": mode.radius(),
        "automatic": choice.is_automatic(),
        "setting_radius": choice.radius,
        "recommended_radius": recommended,
        "load_all": choice.load_all,
        "map_radius": map_radius,
        "start_tile": [job.start_tile().0, job.start_tile().1],
        "listed": job.listed_tiles().count(),
        "listed_missing": job.tiles_listed_missing(),
        "resident_count": resident.len(),
        "resident": resident.iter().map(|k| json!([k.0, k.1])).collect::<Vec<_>>(),
        "queued": job.queued_tiles().len(),
    }))
}
