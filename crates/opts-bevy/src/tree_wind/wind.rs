//! The wind of the trees: its strength and direction from the weather and the setting.

use bevy::prelude::*;

use super::material::TreeMaterial;

use crate::environment::SessionEnvironment;

/// Crown displacement of a 10 m tree in calm weather (m).
pub(super) const CALM_M: f32 = 0.04;
/// Additional crown displacement per m/s of wind (m).
const PER_MS_M: f32 = 0.025;
/// The largest crown displacement of a 10 m tree (m).
pub(super) const MAX_M: f32 = 0.6;
/// The slow sway's frequency (Hz).
const SWAY_HZ: f32 = 0.4;
/// The direction a calm day sways along (Bevy x, z).
pub(super) const CALM_DIRECTION: Vec2 = Vec2::new(0.6, 0.8);

/// Whether trees move (`graphics.tree_wind`), and the wind they move in now.
#[derive(Resource, Debug, Clone, Copy, PartialEq)]
pub struct TreeWindState {
    /// The setting.
    pub enabled: bool,
    /// The uniform every tree material carries.
    pub wind: Vec4,
}

impl Default for TreeWindState {
    fn default() -> Self {
        Self {
            enabled: true,
            wind: wind_uniform(true, Vec2::ZERO, 0.0),
        }
    }
}

/// The uniform for a wind blowing towards `to` (unit vector on the ground, Bevy x and z) at
/// `speed` m/s; zero strength when the setting is off.
#[must_use]
pub fn wind_uniform(enabled: bool, to: Vec2, speed: f32) -> Vec4 {
    if !enabled {
        return Vec4::new(CALM_DIRECTION.x, CALM_DIRECTION.y, 0.0, SWAY_HZ);
    }
    let dir = if speed > 0.0 {
        to.try_normalize().unwrap_or(CALM_DIRECTION)
    } else {
        CALM_DIRECTION
    };
    let strength = (CALM_M + PER_MS_M * speed.max(0.0)).min(MAX_M);
    Vec4::new(dir.x, dir.y, strength, SWAY_HZ)
}

/// Follows the weather's wind and the setting: when the wind uniform changes, every tree material
/// gets it (one material per tree texture, so a handful).
pub fn update_tree_wind(
    env: Option<Res<SessionEnvironment>>,
    mut state: ResMut<TreeWindState>,
    mut materials: ResMut<Assets<TreeMaterial>>,
) {
    let (to, speed) = env.map_or((Vec2::ZERO, 0.0), |e| {
        let c = e.conditions();
        (c.wind_to(), c.wind_speed)
    });
    let wind = wind_uniform(state.enabled, to, speed);
    if wind == state.wind {
        return;
    }
    state.wind = wind;
    tracing::debug!(
        ?wind,
        speed,
        materials = materials.len(),
        "tree wind changed"
    );
    let ids: Vec<_> = materials.ids().collect();
    for id in ids {
        if let Some(mut m) = materials.get_mut(id) {
            m.extension.wind = wind;
        }
    }
}
