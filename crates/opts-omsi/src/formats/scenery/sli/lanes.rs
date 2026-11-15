//! Lane kinds of a spline profile: road, bike, pedestrian, rail and aircraft paths.

use super::*;

/// What a path lane is for, beyond OMSI's four `[path]` types (`docs/src/simulation/lane-kinds.md`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum LaneKind {
    /// Lane for cars, buses and trucks.
    Road,
    /// Lane for cyclists: a road-type path that is too narrow for a motor vehicle, or any
    /// pedestrian/road path of a profile whose file name says bike.
    Bike,
    /// Footpath.
    Pedestrian,
    /// Track (trams, trains).
    Rail,
    /// Aircraft path.
    Aircraft,
}

/// Widest road-type path (m) still read as a bike lane. The corpus has 59 road paths of width
/// 0.1 (e.g. `Invis_Human_Fahrrad.sli`, whose two ±0.85 m lanes are the bike lanes of a
/// footpath), 45 at 0.5; the 1,118 at 1.0 are bus-only guideways (`ÖPNV_BOT.sli`) and stay
/// road lanes ❓ (inferred, not documented by OMSI).
pub const BIKE_LANE_MAX_WIDTH_M: f32 = 0.6;

/// File-name fragments that mark a profile as a bike facility.
const BIKE_NAME_HINTS: [&str; 4] = ["fahrrad", "bike", "radweg", "cycle"];

impl SplinePath {
    /// Kind of lane; `profile_name` is the `.sli` file name (any case).
    #[must_use]
    pub fn kind(&self, profile_name: &str) -> LaneKind {
        let name = profile_name.to_lowercase();
        let named_bike = BIKE_NAME_HINTS.iter().any(|h| name.contains(h));
        match self.path_type {
            0 if self.width <= BIKE_LANE_MAX_WIDTH_M || (named_bike && self.width < 1.5) => {
                LaneKind::Bike
            }
            0 => LaneKind::Road,
            1 => LaneKind::Pedestrian,
            2 => LaneKind::Rail,
            _ => LaneKind::Aircraft,
        }
    }
}

impl SplineProfile {
    /// Lanes of the profile with their kind; `name` is the `.sli` file name.
    #[must_use]
    pub fn lane_kinds(&self, name: &str) -> Vec<(LaneKind, &SplinePath)> {
        self.paths.iter().map(|p| (p.kind(name), p)).collect()
    }

    /// Distinct lane kinds of the profile, in a fixed order. More than one kind means the spline
    /// is a *composite* (a street with a bike lane and a footpath, a tram street, ...): each kind
    /// is a sub-spline with its own traffic.
    #[must_use]
    pub fn composition(&self, name: &str) -> Vec<LaneKind> {
        let mut out: Vec<LaneKind> = Vec::new();
        for (k, _) in self.lane_kinds(name) {
            if !out.contains(&k) {
                out.push(k);
            }
        }
        out.sort_by_key(|k| *k as u8);
        out
    }

    /// Whether the profile carries bike lanes.
    #[must_use]
    pub fn has_bike_lane(&self, name: &str) -> bool {
        self.lane_kinds(name)
            .iter()
            .any(|(k, _)| *k == LaneKind::Bike)
    }
}
