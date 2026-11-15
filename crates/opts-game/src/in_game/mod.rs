//! In-game transit driving simulation lifecycle, loading orchestration, and telemetry HUD.

pub mod attachments;
pub mod camera;
pub mod change_map;
pub mod hot_reload;
pub mod hud;
pub mod instruments;
pub mod load_report;
pub mod loading_pipeline;
pub mod loading_ui;
pub mod moon;
pub mod omsi_world;
pub mod screenshot;
pub mod sky;
pub mod sky_cube;
pub mod stream_budget;
pub mod tile_loading;
pub mod types;
pub mod vehicle_spawner;
pub mod weather;
pub mod world;

use crate::app_state::AppState;
use crate::in_game::camera::{deactivate_menu_camera, reactivate_menu_camera};
use crate::in_game::hud::handle_hud_interactions;
use crate::in_game::loading_pipeline::{handle_loading_interactions, update_loading_progress};
use crate::in_game::loading_ui::spawn_loading_screen_ui;
use crate::new_game_screen::types::NewGameSetup;
use bevy::prelude::*;
use pause_toggle::toggle_pause;

mod formation_menu;
mod pause_toggle;
mod session;
mod vehicle_menu;
pub use camera::MenuCamera;
pub use formation_menu::{FormationCommands, FormationEditor, FormationMenuPlugin};
pub(crate) use session::setup_in_game_session;
#[cfg(feature = "stream-trace")]
use session::stream_trace_frame;
use session::{cleanup_in_game_session, debug_frame_log};
pub use types::{
    CancelLoadingBtn, InGameCamera, InGameEntity, InGameHudRoot, InGameSessionInfo, LoadStage,
    LoadingScreenRoot, PauseMenuBtn, ProgressBarFill, SimulationLoadState, SpeedometerText,
    TachometerText,
};
pub use vehicle_menu::{MenuCommands, VehicleMenu, VehicleMenuPlugin};

mod frame;
mod late;
mod lifecycle;
mod resources;

use frame::add_frame_systems;
use late::add_late_systems;
use lifecycle::add_lifecycle;
use resources::add_resources;

/// Bevy plugin managing the active transit simulation lifecycle, loading sequence, and driving HUD.
#[derive(Default)]
pub struct InGamePlugin;

impl Plugin for InGamePlugin {
    fn build(&self, app: &mut App) {
        add_resources(app);
        add_lifecycle(app);
        add_frame_systems(app);
        add_late_systems(app);
    }
}
