//! The roads of the whole map for the full-size map.
//!
//! The lane graph holds only the resident tiles, so on its own the full-size map would show the
//! neighbourhood of the player and nothing else. When the map is opened for the first time, a
//! background thread lists every tile of the map, reads its splines and scenery paths (the same
//! `[path]` records the traffic graph is built from) and sends the road lanes of each tile to the
//! game; the map draws them for every tile that is not resident, and fills in while the thread
//! works. The thread reads base tiles only (no Chrono overlays, no bent crossings, no terrain), which
//! is exact enough for a 2D overview, and sleeps between tiles so that it does not compete with the
//! streaming of the world.

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::mpsc::{channel, Receiver, TryRecvError};
use std::sync::{Arc, Mutex};

use bevy::math::DVec2;
use bevy::prelude::*;
use opts_bevy::OmsiInstall;
use opts_core::formats::map::MapTile;
use opts_core::formats::scenery::{SceneryObject, ScoPath, SplinePath, SplineProfile};
use opts_traffic::{segments_from_tile, LaneKind, PathSource};

use crate::hud::minimap::{may_touch, MapRoad};

mod load;
mod query;
mod reader;
mod types;
use reader::read_tiles;
use types::*;
pub use types::{ListedTile, OverviewRoads, StopSite, TileRoads};
