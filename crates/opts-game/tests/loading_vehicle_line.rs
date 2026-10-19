//! The vehicle line of the loading screen (M-0076) and the map-only start (M-0068).

mod common;

use bevy::prelude::*;
use opts_game::{
    in_game::SimulationLoadState,
    new_game_screen::types::{GameCatalogue, NewGameSetup, VehicleItem},
};

use common::loading::{args_for, catalogue, loading_app_with, run, screen_count, texts};
use common::{fresh_root, write_map, MiniMap};

fn three_vehicles() -> GameCatalogue {
    let mut c = catalogue("Mini", "MiniBus");
    c.vehicles = vec![
        VehicleItem::named("Alpha_Bus", "Alpha", "Solo"),
        VehicleItem::named("Beta_Bus", "Beta", "Gelenk"),
        VehicleItem::named("Gamma_Tram", "Gamma", "Tramino"),
    ];
    c
}

fn setup_for(vehicle: &str, no_vehicle: bool) -> NewGameSetup {
    NewGameSetup {
        selected_map: "Mini".into(),
        selected_entry_point: "Mini start".into(),
        selected_vehicle: vehicle.into(),
        no_vehicle,
        ..NewGameSetup::default()
    }
}

/// Asserts that the vehicle tile names `expected` and none of the other vehicles.
fn assert_vehicle_line(app: &mut App, expected: &str, when: &str) {
    let all = texts(app);
    assert!(
        all.iter().any(|t| t == expected),
        "{when}: `{expected}` missing in {all:?}"
    );
    for other in ["Alpha Solo", "Beta Gelenk", "Gamma Tramino", "No vehicle"] {
        if other != expected {
            assert!(
                !all.iter().any(|t| t == other),
                "{when}: `{other}` shown in {all:?}"
            );
        }
    }
}

#[test]
fn the_first_loading_frame_names_the_selected_vehicle_or_none_and_never_changes() {
    let root = fresh_root("vehicle-line");
    write_map(&root, &MiniMap::simple("Mini"));
    let cases = [
        ("Alpha Solo", false, "Alpha Solo"),
        ("Beta Gelenk", false, "Beta Gelenk"),
        ("Gamma Tramino", false, "Gamma Tramino"),
        ("", true, "No vehicle"),
    ];
    for (vehicle, none, expected) in cases {
        let mut app = loading_app_with(three_vehicles(), setup_for(vehicle, none), args_for(&root));
        // `loading_app_with` ran exactly one frame: this is the first rendered loading frame.
        assert_vehicle_line(&mut app, expected, "first frame");
        // Through the whole load the line stays the same (it used to switch near 70 %).
        for _ in 0..40 {
            app.update();
            if screen_count(&mut app) == 1 {
                assert_vehicle_line(&mut app, expected, "during the load");
            }
        }
    }
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn a_session_without_a_vehicle_loads_and_runs_with_a_free_camera() {
    let root = fresh_root("no-vehicle");
    write_map(&root, &MiniMap::simple("Mini"));
    let mut app = loading_app_with(three_vehicles(), setup_for("", true), args_for(&root));
    run(&mut app, 60);
    assert!(
        app.world().resource::<SimulationLoadState>().hud_spawned,
        "the drive started"
    );
    assert_eq!(screen_count(&mut app), 0);
    // No vehicle anywhere.
    let registry = app
        .world()
        .resource::<opts_game::session_vehicles::SessionVehicles>();
    assert_eq!(registry.live_count(), 0);
    let players = app
        .world_mut()
        .query_filtered::<Entity, With<opts_bevy::PlayerVehicle>>()
        .iter(app.world())
        .count();
    assert_eq!(players, 0);
    assert_eq!(
        app.world().resource::<opts_bevy::CameraState>().mode,
        opts_bevy::CameraMode::FreeRoam
    );
    let cameras = app
        .world_mut()
        .query_filtered::<Entity, With<opts_bevy::MainCamera>>()
        .iter(app.world())
        .count();
    assert_eq!(cameras, 1, "the free camera exists");
    // The simulation keeps running for many frames without a vehicle (no panic, no stall).
    run(&mut app, 300);
    assert!(app.world().resource::<SimulationLoadState>().hud_spawned);
    let _ = std::fs::remove_dir_all(&root);
}
