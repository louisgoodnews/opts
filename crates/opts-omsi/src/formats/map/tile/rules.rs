//! The traffic rules of a path (the `[rule]` records): owner, kind and the record.

/// What a `[rule]` is attached to: the spline or object whose block precedes it in the tile.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RuleOwner {
    /// A spline: `index` into [`super::MapTile::splines`].
    Spline {
        /// Position in [`super::MapTile::splines`].
        index: usize,
        /// The spline's `id_code`.
        id_code: u64,
    },
    /// A scenery object (a junction, say): `index` into [`super::MapTile::objects`].
    Object {
        /// Position in [`super::MapTile::objects`].
        index: usize,
        /// The object's `id_code`.
        id_code: u64,
    },
}

/// The known rule types of `[rule]` records (`docs/src/formats/map/tiles.md`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RuleKind {
    /// `trafficdensity`: AI traffic density on the path (0 = none).
    TrafficDensity,
    /// `speedlimit`: speed limit on the path.
    SpeedLimit,
    /// `priority`: priority of the path at a junction.
    Priority,
    /// `trucks`: vehicle-type restriction.
    Trucks,
    /// `overtaking_prohib`: overtaking prohibited.
    OvertakingProhibited,
    /// `bus`: vehicle-type restriction.
    Bus,
    /// `no_cars`: vehicle-type restriction.
    NoCars,
}

/// Traffic rule constraint on a path of a spline or object (`[rule]`).
#[derive(Debug, Clone, PartialEq)]
pub struct MapTrafficRule {
    /// The spline or object the rule follows in the file; `None` for a rule before any of them.
    pub owner: Option<RuleOwner>,
    /// Index of the path (lane) among the owner's `[path]` records.
    pub path_index: usize,
    /// Rule type identifier (e.g. `"trafficdensity"`, `"speedlimit"`, `"priority"`).
    pub rule_type: String,
    /// Numeric rule value (speed limit, priority mask, etc.).
    pub value: f32,
    /// Additional rule flag.
    pub flag: u32,
}

impl MapTrafficRule {
    /// The rule type, if it is one of the known ones (case-insensitive).
    #[must_use]
    pub fn kind(&self) -> Option<RuleKind> {
        Some(match self.rule_type.to_ascii_lowercase().as_str() {
            "trafficdensity" => RuleKind::TrafficDensity,
            "speedlimit" => RuleKind::SpeedLimit,
            "priority" => RuleKind::Priority,
            "trucks" => RuleKind::Trucks,
            "overtaking_prohib" => RuleKind::OvertakingProhibited,
            "bus" => RuleKind::Bus,
            "no_cars" => RuleKind::NoCars,
            _ => return None,
        })
    }
}

impl super::MapTile {
    /// `[spline_terrain_align]` / `[spline_terrain_align_2]`: with the value `1` the spline right
    /// before it is aligned ([`super::MapTile::terrain_aligned`]).
    pub(super) fn mark_aligned(&mut self, block: &crate::formats::text::OmsiBlock) {
        let on = block.parameters.first().is_some_and(|v| v.trim() == "1");
        if let (true, Some(spline)) = (on, self.splines.last()) {
            self.terrain_aligned.push(spline.id_code);
        }
    }
}
