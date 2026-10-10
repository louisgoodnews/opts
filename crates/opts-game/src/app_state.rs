//! Top-level application lifecycle states for OPTS.

use bevy::prelude::*;

/// Primary state machine governing navigation and lifecycle in OPTS.
#[derive(States, Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum AppState {
    /// Initial asset and resource loading phase.
    #[default]
    Loading,
    /// Interactive main menu entry screen.
    MainMenu,
    /// Bus and livery selection screen (destination stub).
    VehicleSelection,
    /// Saved game loader (destination stub).
    LoadGame,
    /// Simulation and audio/display settings (destination stub).
    Settings,
    /// Attributions, licensing, and contributor roster (destination stub).
    Credits,
    /// Active 3D driving simulation gameplay.
    InGame,
}
