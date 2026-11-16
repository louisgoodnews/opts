//! The in-game HUD on fixture content (invented map, vehicles and roads, no OMSI assets), laid
//! out by Bevy's real UI layout in a headless app. Shared by the HUD matrix and the golden test.

#![allow(dead_code, unused_imports)]

mod assemble;
mod geometry;
mod setup;

pub use assemble::build_hud;
pub use geometry::{hidden, overlap, rect};
pub use setup::Extras;
