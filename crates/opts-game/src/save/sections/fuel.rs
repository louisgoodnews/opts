//! The `fuel` section (M-0123): the fuel or battery charge of each session vehicle. A file
//! without it keeps the vehicles' fresh models (the player's fuel share at the start).

use bevy::prelude::*;
use opts_bevy::VehicleController;
use opts_sim::vehicle::fuel::FuelState;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::{parse, to_value};
use crate::session_vehicles::{SessionVehicles, VehicleId};

/// Data of the `fuel` section.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FuelData {
    /// The charge of each session vehicle that has a model.
    pub vehicles: Vec<VehicleFuel>,
}

/// The charge of one session vehicle.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct VehicleFuel {
    /// The session vehicle.
    pub id: VehicleId,
    /// Its fuel model as saved (the consumption mode in it is the one of the saving session).
    pub fuel: FuelState,
}

/// The model a restored charge takes: the saved content and kind, the consumption mode of the
/// live model (the setting decides it, not the file).
#[must_use]
pub fn restored(saved: FuelState, live: Option<&FuelState>) -> FuelState {
    let mut fuel = saved;
    if let Some(live) = live {
        fuel.mode = live.mode;
    }
    fuel
}

pub(super) fn fuel_capture(world: &mut World) -> Result<Option<Value>, String> {
    let reg = world.resource::<SessionVehicles>();
    let vehicles = reg
        .records()
        .iter()
        .filter(|r| r.is_alive())
        .filter_map(|r| {
            let entity = r.entity?;
            let fuel = world.get::<VehicleController>(entity)?.sim.fuel.clone()?;
            Some(VehicleFuel { id: r.id, fuel })
        })
        .collect();
    to_value(&FuelData { vehicles })
}

pub(super) fn fuel_apply(world: &mut World, data: &Value) -> Result<(), String> {
    let saved: FuelData = parse(data, "fuel")?;
    for v in saved.vehicles {
        let entity = world
            .resource::<SessionVehicles>()
            .get(v.id)
            .filter(|r| r.is_alive())
            .and_then(|r| r.entity);
        let Some(entity) = entity else { continue };
        let Some(mut controller) = world.get_mut::<VehicleController>(entity) else {
            continue;
        };
        let fuel = restored(v.fuel, controller.sim.fuel.as_ref());
        controller.sim.fuel = Some(fuel);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use opts_sim::vehicle::fuel::{FuelKind, FuelMode};

    #[test]
    fn the_saved_content_is_kept_and_the_live_mode_wins() {
        let mut saved = FuelState::full(FuelKind::Diesel, 250.0);
        saved.set_level(0.25);
        saved.mode = FuelMode::Off;
        let mut live = FuelState::full(FuelKind::Diesel, 250.0);
        live.mode = FuelMode::Accelerated;
        let fuel = restored(saved, Some(&live));
        assert!((fuel.level() - 0.25).abs() < 1e-6);
        assert_eq!(fuel.mode, FuelMode::Accelerated);
    }

    #[test]
    fn without_a_live_model_the_saved_mode_is_taken() {
        let mut saved = FuelState::full(FuelKind::Electric, 100.0);
        saved.mode = FuelMode::Off;
        assert_eq!(restored(saved, None).mode, FuelMode::Off);
    }

    #[test]
    fn the_section_round_trips_through_json() {
        let data = FuelData {
            vehicles: vec![VehicleFuel {
                id: 3,
                fuel: FuelState::full(FuelKind::Diesel, 250.0),
            }],
        };
        let text = serde_json::to_value(&data).unwrap();
        let back: FuelData = parse(&text, "fuel").unwrap();
        assert_eq!(back, data);
    }
}
