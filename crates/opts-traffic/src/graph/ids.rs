//! Lane identifiers, kinds and travel directions of the lane graph.

/// Index of a lane in a [`LaneGraph`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct LaneId(pub u32);

/// What may use a path (`[path]` type `0`/`1`/`2`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum LaneKind {
    /// AI road traffic.
    Road,
    /// AI pedestrians.
    Pedestrian,
    /// Rail vehicles (AI or player).
    Rail,
}

impl LaneKind {
    /// From the `[path]` type number.
    #[must_use]
    pub fn from_path_type(t: u32) -> Option<Self> {
        match t {
            0 => Some(Self::Road),
            1 => Some(Self::Pedestrian),
            2 => Some(Self::Rail),
            _ => None,
        }
    }
}

/// Allowed travel direction of a path (`0` forward only, `1` reverse only, `2` both).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Direction {
    /// Along the path's start→end direction.
    #[default]
    Forward,
    /// Against it.
    Reverse,
    /// Both ways.
    Both,
}

impl Direction {
    /// From the `[path]` direction number (unknown values count as forward).
    #[must_use]
    pub fn from_number(n: u32) -> Self {
        match n {
            1 => Self::Reverse,
            2 => Self::Both,
            _ => Self::Forward,
        }
    }

    /// The direction seen from the other end of the path.
    #[must_use]
    pub fn reversed(self) -> Self {
        match self {
            Self::Forward => Self::Reverse,
            Self::Reverse => Self::Forward,
            Self::Both => Self::Both,
        }
    }
}
