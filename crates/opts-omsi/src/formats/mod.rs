//! Format parser implementations for OMSI file formats.

pub use opts_omsi_map::formats::chrono;
pub use opts_omsi_text::formats::constfile;
pub use opts_omsi_text::formats::cti;
pub use opts_omsi_text::formats::engine_config;
pub use opts_omsi_text::formats::envir;
/// Depot (`.hof`) file parser and operational data models.
pub mod hof {
    pub use crate::formats::vehicles::hof::*;
}
pub use opts_omsi_binary::formats::o3d;
pub use opts_omsi_map::formats::map;
pub use opts_omsi_text::formats::humans;
pub use opts_omsi_text::formats::money;
pub use opts_omsi_text::formats::oft;
pub use opts_omsi_text::formats::org;
pub mod scenery;
pub use opts_omsi_text::formats::sound;
pub use opts_omsi_text::formats::text;
pub use opts_omsi_text::formats::ticketpack;
pub use opts_omsi_text::formats::timetable;
pub mod trains;
pub use opts_omsi_binary::formats::x;
pub use opts_omsi_text::formats::varlist;
pub use opts_omsi_text::formats::weather;
pub use opts_omsi_vehicles::formats::vehicles;
/// `.wav` sound decoder, now the `opts-wav` crate (same paths).
pub use opts_wav as wav;
pub use vehicles::script_texture::{CpuScriptTexture, ScriptTextureCanvas};

pub use vehicles::script_texture;
