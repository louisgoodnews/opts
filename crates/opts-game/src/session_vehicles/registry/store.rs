//! The registry's storage: the [`SessionVehicles`] resource, its fields and the plain queries.

use bevy::prelude::*;
use serde::{Deserialize, Serialize};

use super::types::{Lifecycle, SessionVehicleRecord, VehicleId};

#[derive(Resource, Debug, Clone, Default, Serialize, Deserialize)]
pub struct SessionVehicles {
    pub(super) next_id: VehicleId,
    pub(super) active: Option<VehicleId>,
    pub(super) records: Vec<SessionVehicleRecord>,
}

impl SessionVehicles {
    /// Forgets everything (a session starts or ends).
    pub fn clear(&mut self) {
        *self = Self::default();
    }

    /// All records, oldest first.
    #[must_use]
    pub fn records(&self) -> &[SessionVehicleRecord] {
        &self.records
    }

    /// The record with this id.
    #[must_use]
    pub fn get(&self, id: VehicleId) -> Option<&SessionVehicleRecord> {
        self.records.iter().find(|r| r.id == id)
    }

    /// The record with this id, mutable.
    pub fn get_mut(&mut self, id: VehicleId) -> Option<&mut SessionVehicleRecord> {
        self.records.iter_mut().find(|r| r.id == id)
    }

    /// The id of the active vehicle (the one that takes the driver's input).
    #[must_use]
    pub fn active_id(&self) -> Option<VehicleId> {
        self.active
    }

    /// The active vehicle's record.
    #[must_use]
    pub fn active(&self) -> Option<&SessionVehicleRecord> {
        self.active.and_then(|id| self.get(id))
    }

    /// Records a live change of an equipment option of a vehicle: kept for the save, a saved
    /// configuration and the status of the editor. A value equal to the repaint's own `default`
    /// removes the override; the list stays sorted by name. `false` when there is no such vehicle.
    pub fn set_setvar(&mut self, id: VehicleId, name: &str, value: i32, default: i32) -> bool {
        let Some(r) = self.get_mut(id) else {
            return false;
        };
        r.setvars.retain(|(n, _)| !n.eq_ignore_ascii_case(name));
        if value != default {
            r.setvars.push((name.to_owned(), value));
            r.setvars.sort();
        }
        true
    }

    /// Makes `id` the active vehicle (or none).
    pub fn set_active(&mut self, id: Option<VehicleId>) {
        self.active = id;
    }

    /// Ids of the vehicles in the world, oldest first.
    #[must_use]
    pub fn alive_ids(&self) -> Vec<VehicleId> {
        self.records
            .iter()
            .filter(|r| r.is_alive())
            .map(|r| r.id)
            .collect()
    }

    /// Number of vehicles that are in the world or on their way.
    #[must_use]
    pub fn live_count(&self) -> usize {
        self.records
            .iter()
            .filter(|r| matches!(r.lifecycle, Lifecycle::Alive | Lifecycle::Spawning))
            .count()
    }
}
