//! Camera perspective system: F-key modes, free roam, HUD hint.
//!
//! See `docs/src/architecture/camera-system.md`.

pub mod hud;
pub mod input;
pub mod mode;
pub mod plugin;
pub mod reset;
pub mod state;
pub mod viewpoint;

pub use hud::camera_hint_text;
pub use plugin::{rig_drives_camera, vehicle_accepts_input, OptsCameraPlugin};
pub use state::{
    scale_free_roam_speed, CameraAvailability, CameraMode, CameraState, FREE_ROAM_MIN_SPEED,
};
pub use viewpoint::{
    not_editing_viewpoint, Viewpoint, ViewpointRequest, ViewpointRow, ViewpointState,
};
