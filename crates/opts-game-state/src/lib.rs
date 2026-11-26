//! Shared state types of OPTS: the application states, the persisted settings type and the
//! in-game component and resource types. This is a Bevy-facing type crate: it holds types and
//! pure helpers only. Systems, plugins, UI and file I/O stay in `opts-game`.

pub mod alerts;
pub mod app_state;
pub mod in_game;
pub mod look;
// The session registry types live in `opts-session-vehicles` (ISS-084); re-exported at the old path.
pub use opts_session_vehicles as session_vehicles;
pub mod settings;
pub mod viewpoint_keys;
