//! ECS component, resource, and state definitions for the in-game simulation session.

use bevy::prelude::*;

/// Marker component attached to any entity spawned during an active in-game session.
///
/// Ensures comprehensive teardown and zero entity leakage when navigating back to menus.
#[derive(Component, Debug, Clone, Copy, Default)]
pub struct InGameEntity;

/// Marker component for the 3D in-game simulation camera.
#[derive(Component, Debug, Clone, Copy, Default)]
pub struct InGameCamera;

/// Staged lifecycle phases governing progressive world assembly and loading telemetry.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum LoadStage {
    /// Initializing engine coordinates, viewport, and physics timestep.
    #[default]
    InitEngine,
    /// Constructing 3D asphalt terrain, road networks, and scenery anchors.
    BuildTerrain,
    /// Synthesizing solar lighting angles, sky illumination, and atmospheric weather.
    SetupAtmosphere,
    /// Instantiating player vehicle chassis, livery materials, and Ackermann axles.
    SpawnVehicle,
    /// Calibrating driver controls, instrumentation, and tracking camera rig.
    CalibrateControls,
    /// World fully synthesized and ready for immediate driver control.
    Ready,
    /// Nothing usable was loaded; the loading screen offers retry, another choice and back.
    Failed,
}

/// Resource tracking asynchronous loading progression, telemetry text, and phase transitions.
#[derive(Resource, Debug, Clone)]
pub struct SimulationLoadState {
    /// Active loading phase.
    pub stage: LoadStage,
    /// Fractional completion ratio (0.0 to 1.0).
    pub progress: f32,
    /// Seconds elapsed in the active stage.
    pub stage_elapsed: f32,
    /// Map tiles loaded so far.
    pub tiles_done: usize,
    /// Map tiles asked for.
    pub tiles_total: usize,
    /// Protected vehicle models still being decoded.
    /// The catalogue is still being built (first start); nothing can be resolved yet.
    pub waiting_for_catalogue: bool,
    /// Whether all simulation assets and entities have been successfully synthesized.
    pub is_complete: bool,
    /// Grace period timer before auto-transitioning from ready to active simulation.
    pub auto_launch_delay: f32,
    /// Guard flags preventing redundant entity instantiation during stage updates.
    pub world_spawned: bool,
    pub atmosphere_spawned: bool,
    pub vehicle_spawned: bool,
    pub camera_spawned: bool,
    pub hud_spawned: bool,
}

impl Default for SimulationLoadState {
    fn default() -> Self {
        Self {
            stage: LoadStage::InitEngine,
            progress: 0.05,
            stage_elapsed: 0.0,
            tiles_done: 0,
            tiles_total: 0,
            waiting_for_catalogue: false,
            is_complete: false,
            auto_launch_delay: 0.6,
            world_spawned: false,
            atmosphere_spawned: false,
            vehicle_spawned: false,
            camera_spawned: false,
            hud_spawned: false,
        }
    }
}

/// Snapshot of the active transit session configuration parameters.
#[derive(Resource, Debug, Clone, Default)]
pub struct InGameSessionInfo {
    pub map_name: String,
    pub entry_point: String,
    /// The vehicle line of the loading screen, fixed when the session is entered.
    pub vehicle_name: String,
    /// The session starts without a vehicle (the loading screen says so).
    pub vehicle_none: bool,
    pub livery_name: String,
    pub weather_preset: String,
    pub start_hour: u32,
    pub start_minute: u32,
    pub sim_elapsed_seconds: f32,
}

// ============================================================================
// Loading Screen UI Markers
// ============================================================================

/// Marker component for the full-viewport root node of the loading screen.
#[derive(Component, Debug, Clone, Copy, Default)]
pub struct LoadingScreenRoot;

/// Marker component for the filled visual bar inside the linear progress indicator.
#[derive(Component, Debug, Clone, Copy, Default)]
pub struct ProgressBarFill;

/// Marker component for the "Cancel & Return to Menu" button on the loading screen.
#[derive(Component, Debug, Clone, Copy, Default)]
pub struct CancelLoadingBtn;

// ============================================================================
// In-Game Driving Telemetry HUD Markers
// ============================================================================

/// Marker component for the root container of the in-game HUD overlay.
#[derive(Component, Debug, Clone, Copy, Default)]
pub struct InGameHudRoot;

/// Marker component for the digital speedometer readout in the HUD.
#[derive(Component, Debug, Clone, Copy, Default)]
pub struct SpeedometerText;

/// Marker component for the engine RPM readout in the HUD.
#[derive(Component, Debug, Clone, Copy, Default)]
pub struct TachometerText;

/// Marker component for the transmission gear selector indicator in the HUD.
#[derive(Component, Debug, Clone, Copy, Default)]
pub struct GearIndicatorText;

/// Marker component for the parking brake indicator in the HUD.
#[derive(Component, Debug, Clone, Copy, Default)]
pub struct HandbrakeIndicatorText;

/// Marker component for the simulation clock readout in the HUD.
#[derive(Component, Debug, Clone, Copy, Default)]
pub struct HudClockText;

/// Marker component for the camera view mode indicator in the HUD.
#[derive(Component, Debug, Clone, Copy, Default)]
pub struct CameraModeText;

/// Marker component for the "Exit to Menu" button in the HUD.
#[derive(Component, Debug, Clone, Copy, Default)]
pub struct PauseMenuBtn;
