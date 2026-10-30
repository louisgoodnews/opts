//! The system parameters of [`spawn_traffic`](super::spawn_traffic), bundled to keep its signature short.

use bevy::prelude::*;

use super::*;

/// The timers of the spawner, kept between runs.
#[derive(bevy::ecs::system::SystemParam)]
pub struct SpawnLocals<'s> {
    /// Seconds since the last spawn attempt.
    pub(super) elapsed: Local<'s, f32>,
    /// Seconds since the diagnostic line.
    pub(super) since_log: Local<'s, f32>,
    /// How often an attempt found no place, by reason.
    pub(super) rejected: Local<'s, [u32; 5]>,
}

/// The traffic state, settings and session facts the spawner reads and writes.
#[derive(bevy::ecs::system::SystemParam)]
pub struct SpawnContext<'w> {
    /// The simulation.
    pub(super) state: ResMut<'w, TrafficState>,
    /// The vehicle variants and their slots.
    pub(super) pool: ResMut<'w, TrafficPool>,
    /// Density settings.
    pub(super) settings: Res<'w, TrafficSettings>,
    /// Scheduled traffic.
    pub(super) sched: ResMut<'w, ScheduledTraffic>,
    /// The session environment.
    pub(super) env: Option<Res<'w, opts_bevy::SessionEnvironment>>,
    /// The date and time source.
    pub(super) chrono: Option<Res<'w, crate::in_game::omsi_world::ChronoState>>,
}

/// The asset stores the spawned vehicles' meshes are added to.
#[derive(bevy::ecs::system::SystemParam)]
pub struct SpawnStores<'w> {
    /// Meshes.
    pub(super) meshes: ResMut<'w, Assets<Mesh>>,
    /// Materials.
    pub(super) materials: ResMut<'w, Assets<StandardMaterial>>,
    /// Images.
    pub(super) images: ResMut<'w, Assets<Image>>,
}
