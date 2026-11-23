//! Where the player is, for log messages.
//!
//! Asset errors are raised deep inside loaders that know nothing about the camera; they add
//! [`position_tag`] as a field so a log line says where on the map it happened.

use std::sync::Mutex;

static TAG: Mutex<String> = Mutex::new(String::new());
static LOADING: Mutex<Option<(i32, i32)>> = Mutex::new(None);

/// Records the tile being built (`None` when no tile is loading).
pub fn set_loading_tile(tile: Option<(i32, i32)>) {
    if let Ok(mut l) = LOADING.lock() {
        *l = tile;
    }
}

/// Sets the text returned by [`position_tag`] (called every frame by the game).
pub fn set_position_tag(tag: String) {
    if let Ok(mut t) = TAG.lock() {
        if *t != tag {
            *t = tag;
        }
    }
}

/// Current position as `tile (x, y) local (x, y) world (x, y, z)`, plus the tile being
/// loaded, if any.
#[must_use]
pub fn position_tag() -> String {
    let player = TAG.lock().ok().filter(|t| !t.is_empty()).map_or_else(
        || "unknown (world still loading)".to_string(),
        |t| t.clone(),
    );
    match LOADING.lock().ok().and_then(|l| *l) {
        Some(tile) => format!("while loading tile {tile:?}; player at {player}"),
        None => format!("player at {player}"),
    }
}
