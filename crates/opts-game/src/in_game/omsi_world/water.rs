//! Water in the drive: where the surface lies, and a warning when the player's vehicle is under it.
//!
//! Water is drawn, not simulated: nothing floats, nothing is blocked, vehicles and AI drive on the
//! terrain below it as before (`docs/src/formats/map/tiles.md`, `.map.water`). The one consequence
//! is a log line when the player's vehicle sinks below the surface, so a drive into a lake does not
//! pass unnoticed.

use bevy::prelude::*;
use opts_bevy::PlayerVehicle;

use super::WorldLoadJob;

/// How far below the surface the vehicle's origin must be before it counts as in the water (m).
const SUBMERGED_M: f32 = 0.3;

impl WorldLoadJob {
    /// Height of the water surface at Bevy world position `(x, z)`, or `None` where the tile has
    /// no water file loaded.
    #[must_use]
    pub fn water_level_at(&self, x: f32, z: f32) -> Option<f32> {
        let size = opts_bevy::tile_size();
        let (tx, ty) = ((x / size).floor(), (-z / size).floor());
        let key = (self.start_tile.0 + tx as i32, self.start_tile.1 + ty as i32);
        let water = self.tile_water.get(&key)?;
        Some(water.level_at(x / size - tx, -z / size - ty))
    }
}

/// Warns when the player's vehicle goes under water and notes when it comes out.
pub fn warn_vehicle_in_water(
    job: Option<Res<WorldLoadJob>>,
    vehicles: Query<&Transform, With<PlayerVehicle>>,
    mut inside: Local<bool>,
) {
    let (Some(job), Some(vehicle)) = (job, vehicles.iter().next()) else {
        return;
    };
    // The vehicle root has no parent, so its local transform is its world position.
    let p = vehicle.translation;
    let depth = job.water_level_at(p.x, p.z).map(|level| level - p.y);
    let under = depth.is_some_and(|d| d > SUBMERGED_M);
    if under && !*inside {
        tracing::warn!(
            depth_m = depth.unwrap_or(0.0),
            "the player's vehicle is under water (water is drawn only: no buoyancy, no blocking)"
        );
    } else if !under && *inside {
        tracing::info!("the player's vehicle is out of the water");
    }
    *inside = under;
}

#[cfg(test)]
mod tests {
    use opts_core::formats::map::MapWater;

    #[test]
    fn the_level_blends_between_the_corners_of_a_tile() {
        let w = MapWater {
            version: 1,
            levels: [0.0, 4.0, 0.0, 4.0],
        };
        // A quarter of the way east and anywhere north-south.
        assert!((w.level_at(0.25, 0.7) - 1.0).abs() < 1e-5);
    }
}
