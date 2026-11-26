//! In-game state types of the open-world session that the feature modules of `opts-game` share:
//! the ground index, the rail vehicles and the vehicle bodies. Types and pure helpers only;
//! systems, plugins and file I/O stay in `opts-game` (`in_game::omsi_world`).

// The ground index, the body box and the rail state live in `opts-ground-height` and
// `opts-rail-state` (ISS-084); re-exported at the old paths.
pub use opts_ground_height::{ground_height, plate_surf};
pub use opts_rail_state::{body, rail};
