//! Ground stages of a tile: terrain mesh (carved under the drawn ground) and spline ribbons.
//! The drawn ground and its neighbourhood are in [`surfaces`], the terrain stage in [`terrain`].

use bevy::prelude::*;
use opts_bevy::{ContentLoader, TileTerrain};

use super::{WorldLoadJob, WorldStats};

mod border_normals;
mod surfaces;
mod terrain;
mod water_splines;

pub(super) use border_normals::{beyond_of, stitch_built_tile, TerrainRecord};
pub(super) use surfaces::{ensure_surfaces, prune_tile_surfaces, TileGround, NEIGHBOURHOOD_LEN};
pub(super) use terrain::{load_terrain, spawn_terrain, TerrainInput};
pub(super) use water_splines::{spawn_splines, spawn_water};
