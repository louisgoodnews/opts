//! Spawn requests and the transitions of a vehicle's lifecycle (queued, alive, failed, gone).

use bevy::prelude::*;

use crate::in_game::omsi_world::CameraSetup;

use super::store::SessionVehicles;
use super::types::{DespawnReason, Lifecycle, SessionVehicleRecord, SpawnParams, SpawnPoint, VehicleId};

impl SessionVehicles {
    fn new_record(&mut self, params: SpawnParams, lifecycle: Lifecycle) -> VehicleId {
        self.next_id += 1;
        let id = self.next_id;
        self.records.push(SessionVehicleRecord {
            id,
            catalogue_id: params.catalogue_id,
            name: params.name,
            bus_file: params.bus_file,
            repaint: params.repaint,
            setvars: params.setvars,
            entry: params.entry,
            spawn_entry_id: None,
            spawn_entry_name: String::new(),
            spawn_point: None,
            lifecycle,
            despawn_reason: None,
            error: None,
            saved_state: None,
            bus_rel: None,
            formation: params.formation,
            entity: None,
            trailers: Vec::new(),
            camera: None,
            place_dismissed: false,
            missing_frames: 0,
        });
        id
    }

    /// Queues a vehicle for the spawner and returns its id.
    pub fn request_spawn(&mut self, params: SpawnParams) -> VehicleId {
        self.new_record(params, Lifecycle::Spawning)
    }

    /// The oldest request nobody works on yet.
    #[must_use]
    pub fn next_pending(&self, in_flight: Option<VehicleId>) -> Option<VehicleId> {
        self.records
            .iter()
            .find(|r| r.lifecycle == Lifecycle::Spawning && Some(r.id) != in_flight)
            .map(|r| r.id)
    }

    /// Records where a vehicle was put and that it exists now.
    pub fn mark_alive(
        &mut self,
        id: VehicleId,
        entity: Entity,
        entry: Option<(u32, String)>,
        point: Option<SpawnPoint>,
        camera: Option<CameraSetup>,
    ) {
        if let Some(r) = self.get_mut(id) {
            r.lifecycle = Lifecycle::Alive;
            r.entity = Some(entity);
            r.spawn_entry_id = entry.as_ref().map(|(i, _)| *i);
            r.spawn_entry_name = entry.map(|(_, n)| n).unwrap_or_default();
            r.spawn_point = point;
            r.camera = camera;
            r.place_dismissed = false;
            r.missing_frames = 0;
            r.error = None;
        }
    }

    /// Records that a spawn failed.
    pub fn mark_failed(&mut self, id: VehicleId, error: impl Into<String>) {
        if let Some(r) = self.get_mut(id) {
            r.lifecycle = Lifecycle::Failed;
            r.error = Some(error.into());
            r.entity = None;
        }
        if self.active == Some(id) {
            self.active = None;
        }
    }

    /// Records that a vehicle is gone; it stops being the active one.
    pub fn mark_despawned(&mut self, id: VehicleId, reason: DespawnReason) {
        if let Some(r) = self.get_mut(id) {
            r.lifecycle = Lifecycle::Despawned;
            r.despawn_reason = Some(reason);
            r.entity = None;
            r.trailers.clear();
        }
        if self.active == Some(id) {
            self.active = None;
        }
    }

    /// The vehicle that becomes active when `gone` (the active one) is removed: the most recently
    /// spawned other vehicle that is in the world, or none.
    #[must_use]
    pub fn successor_of(&self, gone: VehicleId) -> Option<VehicleId> {
        self.records
            .iter()
            .rev()
            .find(|r| r.is_alive() && r.id != gone)
            .map(|r| r.id)
    }

    /// The vehicle `step` places after the active one among the living ones (wrapping); with no
    /// active vehicle the first (or last, for a negative step).
    #[must_use]
    pub fn cycle(&self, step: i32) -> Option<VehicleId> {
        let alive = self.alive_ids();
        if alive.is_empty() {
            return None;
        }
        let len = alive.len() as i32;
        let at = self
            .active
            .and_then(|a| alive.iter().position(|id| *id == a))
            .map(|p| p as i32);
        let next = match at {
            Some(p) => (p + step).rem_euclid(len),
            None if step < 0 => len - 1,
            None => 0,
        };
        alive.get(next as usize).copied()
    }

    /// Registers a vehicle that was built outside the spawn queue (the session's first vehicle).
    pub fn register_alive(
        &mut self,
        params: SpawnParams,
        entity: Entity,
        entry: Option<(u32, String)>,
        point: Option<SpawnPoint>,
        camera: Option<CameraSetup>,
    ) -> VehicleId {
        let id = self.new_record(params, Lifecycle::Alive);
        self.mark_alive(id, entity, entry, point, camera);
        id
    }
}
