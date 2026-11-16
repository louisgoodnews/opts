//! Axis-aligned bounds of an object in the map frame.

use bevy::math::Vec3;

/// Axis-aligned bounds in the map frame.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Bounds {
    /// Minimum corner.
    pub min: Vec3,
    /// Maximum corner.
    pub max: Vec3,
}

impl Bounds {
    /// Bounds of a point set; `None` when it is empty.
    #[must_use]
    pub fn of(points: impl IntoIterator<Item = Vec3>) -> Option<Self> {
        let mut it = points.into_iter().filter(|p| p.is_finite());
        let first = it.next()?;
        Some(it.fold(
            Self {
                min: first,
                max: first,
            },
            |b, p| Self {
                min: b.min.min(p),
                max: b.max.max(p),
            },
        ))
    }

    /// Centre.
    #[must_use]
    pub fn centre(&self) -> Vec3 {
        (self.min + self.max) * 0.5
    }

    /// Radius of the enclosing sphere around [`Self::centre`] (at least 0.25 m).
    #[must_use]
    pub fn radius(&self) -> f32 {
        ((self.max - self.min).length() * 0.5).max(0.25)
    }
}
