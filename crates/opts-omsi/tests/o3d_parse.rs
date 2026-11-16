//! Integration tests for `.o3d` parser against mutation kit fixtures.

use std::path::PathBuf;

use opts_omsi::formats::o3d::{parse, O3dParseError};

/// Fixture folder inside the OMSI installation (`$OMSI_PATH`); tests skip without it.
fn fixtures_dir() -> Option<PathBuf> {
    std::env::var_os("OPTS_X_FIXTURES")
        .map(PathBuf::from)
        .or_else(|| {
            std::env::var_os("OMSI_PATH")
                .map(|r| PathBuf::from(r).join("Sceneryobjects/OPTS_Test/model"))
        })
}

fn load_fixture(name: &str) -> Option<Vec<u8>> {
    let path = fixtures_dir()?.join(name);
    if path.exists() {
        Some(std::fs::read(&path).expect("failed to read fixture"))
    } else {
        None
    }
}

#[path = "o3d_parse/chunks.rs"]
mod chunks;
#[path = "o3d_parse/geometry.rs"]
mod geometry;
#[path = "o3d_parse/robustness.rs"]
mod robustness;
#[path = "o3d_parse/versions.rs"]
mod versions;
