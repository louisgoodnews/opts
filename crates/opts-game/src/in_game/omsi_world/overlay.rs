//! Toggleable diagnostic overlay: the developer panel, its readout, position tracking and the
//! wheel-variable overlay.

mod panel;
mod position;
mod readout;
mod wheel_vars;

pub use panel::{
    spawn_debug_overlay, toggle_developer_detail, toggle_developer_panel, DebugOverlayText,
};
pub use position::{log_final_position, track_position};
pub use readout::update_debug_overlay;
pub use wheel_vars::{update_wheel_vars_overlay, DebugWheelVars, WheelVarsText};
