//! The ground height of the open-world session: the ground index of the resident tiles (road
//! samples, terrain and surface plates by cell). Split out of `opts-game-state` (ISS-084), which
//! re-exports both modules so that `opts_game_state::in_game::omsi_world` keeps its paths. The
//! systems that fill and drop the index stay in `opts-game`.

pub mod ground_height;
pub mod plate_surf;
