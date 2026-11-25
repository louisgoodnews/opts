//! ECS component, resource, and state definitions for the in-game simulation session.
//!
//! The types live in the `opts-game-state` crate and are re-exported here under their old paths.

pub use opts_game_state::in_game::types::{
    CameraModeText, CancelLoadingBtn, GearIndicatorText, HandbrakeIndicatorText, HudClockText,
    InGameCamera, InGameEntity, InGameHudRoot, InGameSessionInfo, LoadStage, LoadingScreenRoot,
    PauseMenuBtn, ProgressBarFill, SimulationLoadState, SpeedometerText, TachometerText,
};
