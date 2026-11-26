//! The persisted game settings and the colour look of OPTS (split out of `opts-game-state`, ISS-084):
//! [`settings::GameSettings`] with its defaults, the session overlay and the baseline, and
//! [`look::Look`] with its parameters. The file I/O (`settings.toml`, `settings.db`, the background
//! writer) stays in `opts-settings-io`; the systems that apply the look stay in `opts-game`.

pub mod look;
pub mod settings;
