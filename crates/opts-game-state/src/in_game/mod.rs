//! In-game state types (components, resources, load stages) shared with `opts-game`.

pub mod omsi_world;
// The in-game types live in `opts-in-game-types` (ISS-084); re-exported at the old path.
pub use opts_in_game_types as types;
