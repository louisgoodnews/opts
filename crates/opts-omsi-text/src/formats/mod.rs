//! Text-format parsers of OMSI 2 (see the crate docs).

pub mod constfile;
pub mod cti;
pub mod engine_config;
pub mod envir;
pub mod humans;
pub mod money;
pub mod oft;
pub mod org;
pub mod ticketpack;
pub mod varlist;

// Split out (ISS-087): the same paths, re-exported from the crates that now hold them.
pub use opts_omsi_scan as text;
pub use opts_omsi_sound as sound;
pub use opts_omsi_timetable as timetable;
pub use opts_omsi_weather as weather;
