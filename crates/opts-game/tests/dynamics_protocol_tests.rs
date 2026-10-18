//! `vehicle.dynamics` (M-0073): the spec of the player's vehicle with its sources, the axles and
//! the live loads.

use bevy::prelude::*;
use opts_bevy::control::channel::dispatch;
use opts_bevy::control::state::{ControlRegistry, Outcome};
use opts_bevy::{PlayerVehicle, VehicleController};
use opts_cli::protocol::{code, RpcError};
use opts_core::formats::vehicles::bus::BusManifest;
use opts_sim::VehicleSimulation;
use serde_json::{json, Value};

fn app() -> App {
    let mut app = App::new();
    let mut registry = ControlRegistry::default();
    opts_game::control_dynamics::register(&mut registry);
    app.insert_resource(registry);
    app
}

fn call(app: &mut App) -> Result<Value, RpcError> {
    match dispatch(app.world_mut(), "vehicle.dynamics", &json!({})) {
        Outcome::Done(r) => r,
        Outcome::Wait(_) => panic!("answers at once"),
    }
}

#[test]
fn without_a_player_vehicle_it_is_not_ready() {
    let mut app = app();
    assert_eq!(
        call(&mut app).expect_err("no vehicle").code,
        code::NOT_READY
    );
}

#[test]
fn the_spec_axles_and_loads_of_the_player_vehicle_are_listed_with_their_sources() {
    let manifest = BusManifest::parse(
        "[mass]\n1.2\n0\n0\n0\n\n[schwerpunkt]\n0.55\n\n[rollwiderstand]\n100\n\n\
         [newachse]\nachse_long\n1.2\nachse_raddurchmesser\n0.6\nachse_feder\n40\n\
         achse_maxforce\n10\nachse_daempfer\n3\n\n\
         [newachse]\nachse_long\n-1.3\nachse_raddurchmesser\n0.6\nachse_feder\n40\n\
         achse_maxforce\n10\nachse_daempfer\n3\nachse_antrieb\n1\n",
    )
    .expect("parses");
    let mut app = app();
    app.world_mut().spawn((
        PlayerVehicle,
        VehicleController::new(VehicleSimulation::from_manifest(&manifest)),
    ));
    let d = call(&mut app).expect("answers");
    assert_eq!(
        d["spec"]["mass_kg"],
        json!({ "value": 1200.0, "source": "file" })
    );
    assert_eq!(d["spec"]["cg_height_m"]["source"], "file");
    assert_eq!(d["spec"]["yaw_inertia_kg_m2"]["source"], "default");
    assert_eq!(d["spec"]["rolling_coefficient"]["source"], "file");
    let axles = d["axles"].as_array().expect("axles");
    assert_eq!(axles.len(), 2);
    assert_eq!(axles[0]["steered"], true);
    assert_eq!(axles[1]["driven"], true);
    assert_eq!(axles[0]["spring_n_per_m"], 40_000.0);
    assert_eq!(
        axles[0]["wheel_diameter_m"]
            .as_f64()
            .map(|x| (x * 100.0).round()),
        Some(60.0)
    );
    // The static loads add up to the weight.
    let total: f64 = axles
        .iter()
        .filter_map(|a| a["static_load_n"].as_f64())
        .sum();
    assert!((total - 1_200.0 * 9.81).abs() < 5.0, "{total}");
    assert_eq!(d["state"]["consist_mass_kg"], 1200.0);
    assert_eq!(d["state"]["units"], 1);
}
