//! Types of the expected geometry: the placement, triangles, flags and keyed-mesh summary.

use super::*;

/// Where the object stands: map-frame position and heading (degrees, clockwise from north).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Placement {
    /// Map-frame position (x east, y north, z up).
    pub position: Vec3,
    /// Heading in degrees, clockwise from north.
    pub heading_deg: f32,
    /// Mirror the object's x axis (the rotation search).
    pub mirror_x: bool,
    /// Uniform scale of the object (1; fault seeding uses other values).
    pub scale: f32,
}

impl Placement {
    /// Object space (x right, y up, z forward) to the map frame.
    #[must_use]
    pub fn to_map(&self, p: Vec3) -> Vec3 {
        let (s, c) = self.heading_deg.to_radians().sin_cos();
        let p = p * self.scale;
        let x = if self.mirror_x { -p.x } else { p.x };
        Vec3::new(
            self.position.x + x * c + p.z * s,
            self.position.y - x * s + p.z * c,
            self.position.z + p.y,
        )
    }
}

/// Per-triangle properties that select the masks it belongs to.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct TriFlags {
    /// Part of `E_base` (no `[visible]`, highest-detail LOD group or no LOD).
    pub base: bool,
    /// Drawn alpha-tested or blended (coverage depends on the texture).
    pub alpha: bool,
    /// Carries a text texture.
    pub text: bool,
    /// Drawn from both sides (tree billboards); every other triangle is back-face culled.
    pub two_sided: bool,
}

/// One expected triangle in object space.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ExpectedTri {
    /// Vertices in object space (x right, y up, z forward).
    pub v: [Vec3; 3],
    /// Mask membership.
    pub flags: TriFlags,
    /// Index of the `[mesh]` of the `.sco` the triangle belongs to (0 for trees).
    pub mesh: usize,
}

/// Keyed (scrambled) meshes and how they were resolved.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct KeyedSummary {
    /// Keyed meshes referenced.
    pub total: usize,
    /// Decoded exactly (`o3d::exact`).
    pub decoded: usize,
    /// Not resolvable (the generator does not cover the file): no expected geometry for them.
    pub unresolved: usize,
}

/// Expected geometry of one object.
#[derive(Debug, Clone, Default)]
pub struct Expected {
    /// Triangles (object space).
    pub tris: Vec<ExpectedTri>,
    /// Keyed meshes.
    pub keyed: KeyedSummary,
    /// Distinct `[LOD]` groups.
    pub lod_groups: usize,
    /// Load-stage categories found while reading meshes.
    pub categories: Vec<String>,
}
