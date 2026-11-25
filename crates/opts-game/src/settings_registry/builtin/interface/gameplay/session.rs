//! Time scale, autosave, the door overlay and the passengers' weight.

use super::*;

/// Time scale, autosave, the door overlay and the passengers' weight.
pub(super) fn session() -> Vec<D> {
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
        D::new(
            "gameplay.abs",
            K::Bool,
            "false",
            C::Gameplay,
            98,
            "set-abs",
            "set-abs-desc",
        )
        .keywords(&["abs", "anti-lock", "brakes", "locking", "driver aid"]),
        D::new(
            "gameplay.asr",
            K::Bool,
            "false",
            C::Gameplay,
            99,
            "set-asr",
            "set-asr-desc",
        )
        .keywords(&["asr", "traction", "spin", "wheelspin", "driver aid"]),
    ]
}
