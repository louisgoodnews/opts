//! Built-in Gameplay category settings descriptors (traffic, pedestrians, passengers, tiles).

use super::*;

mod session;
use session::session;

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
