//! Built-in Gameplay category settings descriptors (traffic, pedestrians, passengers, tiles).

use super::*;

pub(super) fn gameplay() -> Vec<D> {
    [density(), session(), tiles()].concat()
}

/// Traffic, parked cars, pedestrians and passengers.
fn density() -> Vec<D> {
    vec![
        D::new(
            "gameplay.traffic",
            K::Int {
                min: 0,
                max: 1000,
                step: 10,
            },
            "150",
            C::Gameplay,
            10,
            "set-traffic",
            "set-traffic-desc",
        )
        .keywords(&["ai", "vehicles", "cars", "density", "baseline"])
        .since("0.2"),
        D::new(
            "gameplay.parked",
            K::Int {
                min: 0,
                max: 100,
                step: 5,
            },
            "100",
            C::Gameplay,
            12,
            "set-parked",
            "set-parked-desc",
        )
        .keywords(&["parked", "cars", "bays", "density"])
        .since("0.5"),
        D::new(
            "gameplay.pedestrians",
            K::Int {
                min: 0,
                max: 1000,
                step: 10,
            },
            "120",
            C::Gameplay,
            20,
            "set-pedestrians",
            "set-pedestrians-desc",
        )
        .keywords(&["people", "walkers", "density"])
        .since("0.2"),
        D::new(
            "gameplay.passengers",
            K::Bool,
            "true",
            C::Gameplay,
            25,
            "set-passengers",
            "set-passengers-desc",
        )
        .keywords(&["people", "boarding", "bus stop", "tickets", "passengers"])
        .since("0.4"),
        D::new(
            "gameplay.passenger_rate",
            K::Int {
                min: 0,
                max: 1000,
                step: 10,
            },
            "120",
            C::Gameplay,
            26,
            "set-passenger-rate",
            "set-passenger-rate-desc",
        )
        .keywords(&["people", "boarding", "bus stop", "crowd", "load", "full"])
        .since("0.5"),
    ]
}

/// Time scale, autosave, the door overlay and the passengers' weight.
fn session() -> Vec<D> {
    vec![
        D::new(
            "gameplay.time_scale",
            K::Float {
                min: 0.0,
                max: 60.0,
                step: 1.0,
            },
            "1",
            C::Gameplay,
            30,
            "set-time-scale",
            "set-time-scale-desc",
        )
        .keywords(&["clock", "speed", "time"])
        .not_yet(&SETTINGS_TIME_SCALE),
        D::new(
            "gameplay.autosave_minutes",
            K::Int {
                min: 0,
                max: 60,
                step: 5,
            },
            "0",
            C::Gameplay,
            40,
            "set-autosave",
            "set-autosave-desc",
        )
        .keywords(&["save", "backup", "interval"])
        .since("0.3"),
        D::new(
            "camera.door_overlay",
            K::Bool,
            "false",
            C::Gameplay,
            95,
            "set-door-overlay",
            "set-door-overlay-desc",
        )
        .keywords(&[
            "door",
            "camera",
            "overlay",
            "picture in picture",
            "boarding",
        ])
        .since("0.3"),
        D::new(
            "gameplay.passenger_weight",
            K::Bool,
            "true",
            C::Gameplay,
            96,
            "set-passenger-weight",
            "set-passenger-weight-desc",
        )
        .keywords(&["passengers", "mass", "weight", "load", "braking"])
        .since("0.3"),
        D::new(
            "gameplay.passenger_mass_kg",
            K::Int {
                min: 40,
                max: 120,
                step: 5,
            },
            "75",
            C::Gameplay,
            97,
            "set-passenger-mass",
            "set-passenger-mass-desc",
        )
        .keywords(&["passengers", "mass", "weight", "kilogram"])
        .since("0.3"),
    ]
}

/// Which tiles stay loaded.
fn tiles() -> Vec<D> {
    vec![
        D::new(
            "gameplay.load_all_tiles",
            K::Bool,
            "false",
            C::Gameplay,
            50,
            "set-load-all-tiles",
            "set-load-all-tiles-desc",
        )
        .keywords(&[
            "map",
            "tiles",
            "streaming",
            "memory",
            "preload",
            "everything",
        ])
        .since("0.3"),
        D::new(
            "gameplay.tile_radius",
            K::Int {
                min: 0,
                max: 16,
                step: 1,
            },
            "0",
            C::Gameplay,
            60,
            "set-tile-radius",
            "set-tile-radius-desc",
        )
        .keywords(&["map", "tiles", "streaming", "distance", "view", "automatic"])
        .since("0.3"),
    ]
}
