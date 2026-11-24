//! The readers under test: each with a function that builds its valid sample and a parse check.
//! Text readers take the digit-run replacement; binary readers take the fixed-width window
//! replacement (`mutate::windows`).

use opts_omsi::formats::map::terrain::MapTerrain;
use opts_omsi::formats::map::tile::MapTile;
use opts_omsi::formats::o3d;
use opts_omsi::formats::scenery::sli::SplineProfile;
use opts_omsi::formats::vehicles::bus::BusManifest;
use opts_omsi::formats::vehicles::hof::HofFile;
use opts_omsi::formats::x::parse_x;

use super::samples;

pub struct Reader {
    pub name: &'static str,
    /// Builds the valid sample (in code; no game file).
    pub sample: fn() -> Vec<u8>,
    /// `true` for a binary format: fixed-width windows are replaced, not digit runs.
    pub binary: bool,
    pub parse: fn(&[u8]) -> bool,
}

fn text(sample: &str) -> Vec<u8> {
    sample.as_bytes().to_vec()
}

fn x_sample() -> Vec<u8> {
    text(samples::X)
}
fn bus_sample() -> Vec<u8> {
    text(samples::BUS)
}
fn sli_sample() -> Vec<u8> {
    text(samples::SLI)
}
fn hof_sample() -> Vec<u8> {
    text(samples::HOF)
}
fn map_tile_sample() -> Vec<u8> {
    text(samples::MAP_TILE)
}
fn o3d_sample() -> Vec<u8> {
    samples::o3d()
}
fn terrain_sample() -> Vec<u8> {
    samples::terrain()
}

fn x_ok(bytes: &[u8]) -> bool {
    parse_x(bytes).is_ok()
}

fn bus_ok(bytes: &[u8]) -> bool {
    BusManifest::parse_bytes(bytes).is_ok()
}

fn sli_ok(bytes: &[u8]) -> bool {
    SplineProfile::parse(bytes).is_ok()
}

fn hof_ok(bytes: &[u8]) -> bool {
    HofFile::parse_bytes(bytes).is_ok()
}

fn map_tile_ok(bytes: &[u8]) -> bool {
    MapTile::parse(bytes).is_ok()
}

fn o3d_ok(bytes: &[u8]) -> bool {
    o3d::parse(bytes).is_ok()
}

fn terrain_ok(bytes: &[u8]) -> bool {
    MapTerrain::parse(bytes).is_ok()
}

pub const X_READER: Reader = Reader {
    name: "x",
    sample: x_sample,
    binary: false,
    parse: x_ok,
};
pub const BUS_READER: Reader = Reader {
    name: "bus",
    sample: bus_sample,
    binary: false,
    parse: bus_ok,
};
pub const SLI_READER: Reader = Reader {
    name: "sli",
    sample: sli_sample,
    binary: false,
    parse: sli_ok,
};
pub const HOF_READER: Reader = Reader {
    name: "hof",
    sample: hof_sample,
    binary: false,
    parse: hof_ok,
};
pub const MAP_TILE_READER: Reader = Reader {
    name: "map-tile",
    sample: map_tile_sample,
    binary: false,
    parse: map_tile_ok,
};
pub const O3D_READER: Reader = Reader {
    name: "o3d",
    sample: o3d_sample,
    binary: true,
    parse: o3d_ok,
};
pub const TERRAIN_READER: Reader = Reader {
    name: "map-terrain",
    sample: terrain_sample,
    binary: true,
    parse: terrain_ok,
};
