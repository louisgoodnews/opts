//! The registry's storage moved to `opts-game-state` (ISS-064); this module re-exports it at its
//! old path. The spawn-request transitions and the save section are part of that type there.

pub use opts_game_state::session_vehicles::SessionVehicles;
