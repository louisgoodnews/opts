//! The hybrid streaming policy (M-0114): how the streamer behaves at the player's speed.
//!
//! Two regimes. At speed (tile regime) whole tiles are loaded ahead along the heading; when slow
//! (object regime) the tile being built places its nearest objects first. The blend between them
//! is continuous in speed; the discrete label has hysteresis. Spec: `docs/src/architecture/hybrid-streaming.md`.

/// Speed (m/s) at and below which the object regime has full say (about 11 km/h).
pub const SLOW_MPS: f32 = 3.0;
/// Speed (m/s) at and above which the tile regime has full say (36 km/h).
pub const FAST_MPS: f32 = 10.0;
/// Largest look-ahead in tile steps beyond the radius (tile regime).
pub const MAX_LOOKAHEAD: i32 = 2;
/// Heading weight of the priority at full tile regime; the object regime uses plain distance.
pub const MAX_HEADING_WEIGHT: f32 = 0.4;
/// Blend at which the label switches to the tile regime (hysteresis: leaves below [`LEAVE_TILE`]).
pub const ENTER_TILE: f32 = 0.6;
/// Blend at which the label switches back to the object regime.
pub const LEAVE_TILE: f32 = 0.4;
/// Tile steps of the tile regime that a tile is loaded ahead by (eviction hysteresis floor).
pub const MIN_HYSTERESIS: i32 = 1;

/// `streaming.mode`: the regime chosen by the speed, or one forced regime.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum StreamMode {
    /// The blend follows the speed.
    #[default]
    Auto,
    /// Always the tile regime.
    Tile,
    /// Always the object regime.
    Object,
}

impl StreamMode {
    /// Reads the setting; absent or unknown values mean [`StreamMode::Auto`].
    #[must_use]
    pub fn from_setting(raw: Option<&str>) -> Self {
        match raw.map(str::trim) {
            Some("tile") => Self::Tile,
            Some("object") => Self::Object,
            _ => Self::Auto,
        }
    }

    /// `auto`, `tile` or `object`, as the setting and the control protocol name them.
    #[must_use]
    pub fn name(self) -> &'static str {
        match self {
            Self::Auto => "auto",
            Self::Tile => "tile",
            Self::Object => "object",
        }
    }
}

/// The regime the streamer is in (the label of the blend, with hysteresis).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum Regime {
    /// Slow or stopped: near objects first.
    #[default]
    Object,
    /// At speed: whole tiles ahead along the heading.
    Tile,
}

impl Regime {
    /// `object` or `tile`.
    #[must_use]
    pub fn name(self) -> &'static str {
        match self {
            Self::Object => "object",
            Self::Tile => "tile",
        }
    }
}

/// Smoothstep of the speed: 0 is the object regime, 1 the tile regime. Non-finite speeds count as
/// stopped.
#[must_use]
pub fn speed_blend(speed_mps: f32) -> f32 {
    if !speed_mps.is_finite() {
        return 0.0;
    }
    let t = ((speed_mps - SLOW_MPS) / (FAST_MPS - SLOW_MPS)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

/// The blend the mode means at this speed; a forced mode is fully one regime.
#[must_use]
pub fn effective_blend(mode: StreamMode, speed_mps: f32) -> f32 {
    match mode {
        StreamMode::Auto => speed_blend(speed_mps),
        StreamMode::Tile => 1.0,
        StreamMode::Object => 0.0,
    }
}

/// Tile steps beyond the radius that are loaded ahead along the heading.
#[must_use]
pub fn lookahead(blend: f32) -> i32 {
    (blend.clamp(0.0, 1.0) * MAX_LOOKAHEAD as f32).round() as i32
}

/// Weight of the heading in the priority of a wanted tile.
#[must_use]
pub fn heading_weight(blend: f32) -> f32 {
    MAX_HEADING_WEIGHT * blend.clamp(0.0, 1.0)
}

/// Eviction hysteresis: never less than the look-ahead, so the prefetched tiles are not evicted
/// at once.
#[must_use]
pub fn hysteresis(ahead: i32) -> i32 {
    ahead.max(MIN_HYSTERESIS)
}

/// The regime after a new blend: it switches up at [`ENTER_TILE`] and back at [`LEAVE_TILE`], so
/// the label does not flicker at the threshold.
#[must_use]
pub fn latch(current: Regime, blend: f32) -> Regime {
    match current {
        Regime::Object if blend >= ENTER_TILE => Regime::Tile,
        Regime::Tile if blend <= LEAVE_TILE => Regime::Object,
        keep => keep,
    }
}

/// What the streamer reports about its regime (the control protocol and the overlay read it).
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct StreamStatus {
    /// The regime label.
    pub regime: Regime,
    /// The measured speed of the player in m/s.
    pub speed_mps: f32,
    /// The blend this frame (0 object, 1 tile).
    pub blend: f32,
}

#[cfg(test)]
mod tests;
