//! The `[maplight]` component and the spawning of one per lamp of a placed object.

use bevy::prelude::*;
use opts_core::formats::scenery::SceneryObject;

use crate::coords::omsi_to_bevy_pos;

/// One `[maplight]` of a placed object (its entity sits at the light's position).
#[derive(Component, Debug, Clone, Copy, PartialEq)]
pub struct MapLight {
    /// Colour (0–1 per channel, as written).
    pub color: Vec3,
    /// Radius at full intensity (m).
    pub radius_m: f32,
}

/// Spawns a [`MapLight`] child of `root` for every `[maplight]` of `sco`; returns how many.
pub fn spawn_map_lights(commands: &mut Commands, root: Entity, sco: &SceneryObject) -> usize {
    for light in &sco.map_lights {
        let e = commands
            .spawn((
                Transform::from_translation(omsi_to_bevy_pos(light.position)),
                MapLight {
                    color: light.color,
                    radius_m: light.radius.max(0.1),
                },
                Name::new("MapLight"),
            ))
            .id();
        commands.entity(root).add_child(e);
    }
    sco.map_lights.len()
}
