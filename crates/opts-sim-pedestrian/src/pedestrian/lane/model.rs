//! The footpath lane type: its travel directions, its polyline and its rail-points gate.

use glam::Vec3;

/// Directions a lane may be walked in, relative to its own direction of travel.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Travel {
    /// Only from the start to the end (`direction` 0).
    Forward,
    /// Only from the end to the start (`direction` 1).
    Reverse,
    /// Both ways (`direction` 2).
    Both,
}

impl Travel {
    /// Reads the `direction` field of a `[path]`.
    #[must_use]
    pub fn from_field(direction: u32) -> Self {
        match direction {
            0 => Self::Forward,
            1 => Self::Reverse,
            _ => Self::Both,
        }
    }
}

/// A walkable polyline in the world frame (x east, y north, z up, metres).
#[derive(Debug, Clone, PartialEq)]
pub struct FootLane {
    /// Polyline vertices.
    pub points: Vec<Vec3>,
    /// Cumulative length at each vertex (`cum[0] = 0`).
    pub cum: Vec<f32>,
    /// Allowed directions.
    pub travel: Travel,
    /// Lane width (m).
    pub width: f32,
    /// The rail points position this lane belongs to, if any.
    pub gate: Option<Gate>,
}

/// A lane that is passable only while rail points are in one position (`[switchdir]`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Gate {
    /// The points, one key per placed object.
    pub switch: u64,
    /// The position of the points (`[switchdir]`) in which the lane is passable.
    pub position: u8,
}

impl FootLane {
    /// Builds a lane; `None` when it has fewer than two distinct points.
    #[must_use]
    pub fn new(points: Vec<Vec3>, travel: Travel, width: f32) -> Option<Self> {
        let mut cum = Vec::with_capacity(points.len());
        let mut total = 0.0;
        for (i, p) in points.iter().enumerate() {
            if i > 0 {
                total += (*p - points[i - 1]).length();
            }
            cum.push(total);
        }
        (points.len() >= 2 && total > 0.05).then_some(Self {
            points,
            cum,
            travel,
            width,
            gate: None,
        })
    }

    /// The same lane, passable only in one position of rail points.
    #[must_use]
    pub fn with_gate(mut self, gate: Gate) -> Self {
        self.gate = Some(gate);
        self
    }

    /// Total length (m).
    #[must_use]
    pub fn length(&self) -> f32 {
        self.cum.last().copied().unwrap_or(0.0)
    }

    /// Position and heading (radians, clockwise from north) at arc length `s` (clamped).
    #[must_use]
    pub fn at(&self, s: f32) -> (Vec3, f32) {
        let s = s.clamp(0.0, self.length());
        let i = self
            .cum
            .partition_point(|c| *c <= s)
            .clamp(1, self.points.len() - 1);
        let (a, b) = (self.points[i - 1], self.points[i]);
        let seg = (self.cum[i] - self.cum[i - 1]).max(1e-6);
        let t = (s - self.cum[i - 1]) / seg;
        let d = b - a;
        (a + d * t, d.x.atan2(d.y))
    }

    /// Start (`false`) or end (`true`) point.
    #[must_use]
    pub fn end_point(&self, end: bool) -> Vec3 {
        if end {
            self.points[self.points.len() - 1]
        } else {
            self.points[0]
        }
    }
}
