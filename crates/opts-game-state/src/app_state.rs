//! Top-level application lifecycle states for OPTS.

use bevy::prelude::*;

/// Primary state machine governing navigation and lifecycle in OPTS.
#[derive(States, Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum AppState {
    /// Initial asset and resource loading phase.
    #[default]
    Loading,
    /// Home, the first screen of the shell.
    MainMenu,
    /// *Drive*: planning a drive (map, vehicle, time).
    VehicleSelection,
    /// Simulation and audio/display settings.
    Settings,
    /// Help: controls, guides, report an issue, About.
    Help,
    /// Active 3D driving simulation gameplay.
    InGame,
}
