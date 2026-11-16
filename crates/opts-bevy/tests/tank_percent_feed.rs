//! The fuel level reaches the scripts as `tank_percent` (M-0123).

use opts_bevy::vehicle_script::host_inputs;
use opts_bevy::VehicleController;
use opts_sim::vehicle::fuel::{FuelKind, FuelMode};

#[test]
fn the_fuel_level_is_the_tank_percent_the_scripts_read() {
    let mut c = VehicleController::standard_bus(11_500.0, 5.85);
    assert_eq!(
        host_inputs(&c, None).tank_percent,
        100.0,
        "a vehicle without a model reads full"
    );
    c.sim
        .ensure_fuel(FuelKind::Diesel, 250.0, FuelMode::Realistic);
    if let Some(fuel) = c.sim.fuel.as_mut() {
        fuel.set_level(0.25);
    }
    assert!((host_inputs(&c, None).tank_percent - 25.0).abs() < 0.5);
    c.sim.refuel();
    assert!((host_inputs(&c, None).tank_percent - 100.0).abs() < 1e-4);
}
