//! Pure helpers of the mini map: the speed-based zoom filter and the RGBA8 pixel buffer that the
//! drawing writes into. No Bevy, no world: the game's `hud::minimap` keeps the systems.

pub mod canvas;
pub mod zoom;
