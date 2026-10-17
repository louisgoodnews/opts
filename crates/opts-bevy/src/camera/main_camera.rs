//! The [`MainCamera`] marker.

use bevy::prelude::*;

/// The one camera that shows the player's view of the simulation.
///
/// Rule: every system that means "the player's view" (camera input and modes, sky dome and tile
/// streaming centres, traffic/pedestrian/rail spawn centres, minimap, LOD and light culling,
/// the look presets, HDR, reflections) queries `With<MainCamera>`, never `With<Camera3d>`.
/// `Camera3d` also marks auxiliary cameras (the reflection prototype's [`MirrorCamera`] renders
/// to a texture) and a query on it would find two cameras, move the wrong one or stop with a
/// `single()` error. Only code that deliberately means "any 3D camera" may query `Camera3d`, and it
/// says so in a comment.
///
/// Spawn exactly one entity with this marker per session (together with `Camera3d`).
///
/// [`MirrorCamera`]: crate::reflections
#[derive(Component, Debug, Clone, Copy, Default, Reflect)]
#[reflect(Component)]
pub struct MainCamera;
