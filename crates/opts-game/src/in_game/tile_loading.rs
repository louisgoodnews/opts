//! Tile loading controls (M-0080): how much of the map stays resident.
//!
//! Two settings decide it: `gameplay.load_all_tiles` keeps every tile of the map resident, and
//! `gameplay.tile_radius` keeps the tiles within that Chebyshev radius of the centre tile (1 is
//! 3 x 3, 2 is 5 x 5, ...); `0` means *automatic*, the radius `opts-hardware` recommends for this
//! machine. This module holds the decisions as pure functions so they are testable without a
//! map; the streamer ([`stream_world_tiles`](super::omsi_world::stream_world_tiles)) and the
//! loading pipeline apply them. See `docs/src/architecture/map-streaming.md`.

use opts_hardware::recommend::{recommended_tile_radius, MapStats};
use opts_hardware::HardwareProfile;

use super::stream_budget::{chebyshev, evictable, TileKey};
use crate::settings::GameSettings;

/// Key of the *Load all tiles on start* setting.
pub const LOAD_ALL_KEY: &str = "gameplay.load_all_tiles";
/// Key of the *Tile radius* setting (`0`: automatic).
pub const RADIUS_KEY: &str = "gameplay.tile_radius";
/// Tiles beyond `radius + HYSTERESIS` are unloaded, so the radius does not thrash at borders.
pub const HYSTERESIS: i32 = 1;
/// Largest value the *Tile radius* setting accepts.
pub const RADIUS_SETTING_MAX: i64 = 16;
/// The radius used while no hardware profile is known: the 3 x 3 area of earlier versions.
pub const FALLBACK_RADIUS: u32 = 1;

/// What stays resident.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TileMode {
    /// The tiles within this Chebyshev radius of the centre tile.
    Radius(i32),
    /// Every tile of the map, never unloaded.
    All,
}

impl Default for TileMode {
    fn default() -> Self {
        Self::Radius(FALLBACK_RADIUS as i32)
    }
}

impl TileMode {
    /// `all` or `radius`, as the control protocol reports it.
    #[must_use]
    pub fn name(self) -> &'static str {
        match self {
            Self::All => "all",
            Self::Radius(_) => "radius",
        }
    }

    /// The radius of [`Self::Radius`]; `None` for [`Self::All`].
    #[must_use]
    pub fn radius(self) -> Option<i32> {
        match self {
            Self::Radius(r) => Some(r),
            Self::All => None,
        }
    }
}

/// The two settings as the user stored them.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct TileChoice {
    /// *Load all tiles on start*.
    pub load_all: bool,
    /// *Tile radius*; `0` is automatic.
    pub radius: u32,
}

impl TileChoice {
    /// Reads both settings (absent or unreadable values are the defaults: off, automatic).
    #[must_use]
    pub fn from_settings(settings: &GameSettings) -> Self {
        Self {
            load_all: settings.raw(LOAD_ALL_KEY).as_deref() == Some("true"),
            radius: settings
                .raw(RADIUS_KEY)
                .and_then(|v| v.trim().parse().ok())
                .unwrap_or(0),
        }
    }

    /// Whether the radius follows the hardware.
    #[must_use]
    pub fn is_automatic(self) -> bool {
        self.radius == 0
    }

    /// The mode this choice means on a machine whose recommended radius is `recommended`, for a
    /// map that `max_radius` tiles cover. *Load all* wins; a radius is at least 1 and at most
    /// `max_radius`.
    #[must_use]
    pub fn mode(self, recommended: u32, max_radius: u32) -> TileMode {
        if self.load_all {
            return TileMode::All;
        }
        let radius = if self.is_automatic() {
            recommended
        } else {
            self.radius
        };
        TileMode::Radius(radius.clamp(1, max_radius.max(1)) as i32)
    }
}

/// The radius `opts-hardware` recommends; [`FALLBACK_RADIUS`] while no profile is known.
#[must_use]
pub fn recommended(profile: Option<&HardwareProfile>, map: &MapStats) -> u32 {
    profile.map_or(FALLBACK_RADIUS, |p| recommended_tile_radius(p, map))
}

/// The mode the settings mean on this machine for a map that `map_radius` tiles cover.
#[must_use]
pub fn resolve(choice: TileChoice, profile: Option<&HardwareProfile>, map_radius: u32) -> TileMode {
    let stats = MapStats {
        max_radius: map_radius,
        ..MapStats::default()
    };
    choice.mode(recommended(profile, &stats), map_radius)
}

/// The radius at which the map is covered from `start`: the farthest listed tile.
#[must_use]
pub fn max_radius(listed: impl IntoIterator<Item = TileKey>, start: TileKey) -> u32 {
    listed
        .into_iter()
        .map(|k| chebyshev(k, start))
        .max()
        .unwrap_or(1)
        .max(1) as u32
}

/// Resident tiles to unload: in radius mode those beyond the radius plus [`HYSTERESIS`], with
/// *all* never any.
pub fn evict(
    mode: TileMode,
    centre: TileKey,
    resident: impl IntoIterator<Item = TileKey>,
) -> Vec<TileKey> {
    match mode {
        TileMode::All => Vec::new(),
        TileMode::Radius(r) => evictable(resident, centre, r, HYSTERESIS),
    }
}

/// Tiles to bring in, nearest to `centre` first: the square of the radius, or every `listed`
/// tile in *all* mode; `skip` drops those that are resident, known missing, and so on.
pub fn wanted(
    mode: TileMode,
    centre: TileKey,
    listed: impl IntoIterator<Item = TileKey>,
    skip: impl Fn(TileKey) -> bool,
) -> Vec<TileKey> {
    let mut keys: Vec<TileKey> = match mode {
        TileMode::All => listed.into_iter().collect(),
        TileMode::Radius(r) => (-r..=r)
            .flat_map(|dy| (-r..=r).map(move |dx| (centre.0 + dx, centre.1 + dy)))
            .collect(),
    };
    keys.retain(|k| !skip(*k));
    keys.sort_by_key(|k| (chebyshev(*k, centre), *k));
    keys
}

#[cfg(test)]
mod tests;
