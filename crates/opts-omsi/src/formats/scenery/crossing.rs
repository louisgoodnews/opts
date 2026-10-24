//! `[crossing_heightdeformation]`: a crossing bent over its base mesh
//! (`docs/src/formats/text/sco.md`, `docs/src/meta/berlin-spandau-2026-10-17.md` System 2).
//!
//! The keyword names a small base mesh (`…_base.o3d`, a ruled strip without materials). Each
//! vertex of the object's meshes is raised by the base surface's height at its `(x, z)`, 0
//! outside the base's footprint; the object's `[path]`s and `[splinehelper]` arms follow the same
//! surface, so the arms meet their splines again (Berlin-Spandau: 0.0 % of the 11 bent
//! placements below the terrain, every arm within 1 cm of its spline). All coordinates here are
//! the mesh file's own axes: `x` right, `y` up, `z` forward.

use glam::{Vec2, Vec3};

use super::SceneryObject;
use crate::formats::o3d::O3d;

/// Tolerance of the point-in-triangle test on the base surface (barycentric).
const INSIDE_TOLERANCE: f32 = 1e-3;
/// Steepest a crossing face may be and still count as ground (`|n_y|` of its unit normal).
const GROUND_FACE_MIN_NY: f32 = 0.7;
/// Smallest crossing face that counts as ground (m²).
const GROUND_FACE_MIN_AREA_M2: f32 = 0.05;
/// Height band of ground faces above the base surface (m): kerbs and islands yes, roofs no.
const GROUND_FACE_BAND_M: (f32, f32) = (-1.5, 3.0);

/// The height surface of a `[crossing_heightdeformation]` base mesh.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct BaseSurface {
    /// Triangles of the base mesh (mesh axes).
    triangles: Vec<[Vec3; 3]>,
}

impl BaseSurface {
    /// The surface of a base mesh whose positions are decoded (plain or `o3d::exact`).
    #[must_use]
    pub fn from_meshes(meshes: &[O3d]) -> Self {
        let triangles = meshes
            .iter()
            .flat_map(|m| {
                m.indices.iter().filter_map(|t| {
                    let v = |i: u32| m.vertices.get(i as usize).map(|v| v.position);
                    Some([v(t[0])?, v(t[1])?, v(t[2])?])
                })
            })
            .collect();
        Self { triangles }
    }

    /// A surface from triangles in mesh axes.
    #[must_use]
    pub fn from_triangles(triangles: Vec<[Vec3; 3]>) -> Self {
        Self { triangles }
    }

    /// Whether the base has no triangles (bends nothing).
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.triangles.is_empty()
    }

    /// Height of the base surface at `(x, z)` (mesh axes), 0 outside its footprint. Where
    /// triangles overlap in plan, the one the point lies deepest inside wins.
    #[must_use]
    pub fn height(&self, x: f32, z: f32) -> f32 {
        let p = Vec2::new(x, z);
        let mut best: Option<(f32, f32)> = None;
        for t in &self.triangles {
            let (a, b, c) = (
                Vec2::new(t[0].x, t[0].z),
                Vec2::new(t[1].x, t[1].z),
                Vec2::new(t[2].x, t[2].z),
            );
            let d = (b - c).perp_dot(a - c);
            if d.abs() < 1e-9 {
                continue;
            }
            let l1 = (b - c).perp_dot(p - c) / d;
            let l2 = (c - a).perp_dot(p - c) / d;
            let l3 = 1.0 - l1 - l2;
            let inside = l1.min(l2).min(l3);
            if inside > -INSIDE_TOLERANCE && best.is_none_or(|(m, _)| inside > m) {
                best = Some((inside, l1 * t[0].y + l2 * t[1].y + l3 * t[2].y));
            }
        }
        best.map_or(0.0, |(_, h)| h)
    }

    /// Raises every vertex of `mesh` by the surface height under it. Normals are kept (the
    /// base slopes by a few per cent).
    pub fn bend_mesh(&self, mesh: &mut O3d) {
        if self.is_empty() {
            return;
        }
        for v in &mut mesh.vertices {
            v.position.y += self.height(v.position.x, v.position.z);
        }
    }
}

/// Plan position (`x` right, `y` forward) of a point `s` metres along an object `[path]`.
fn path_point(p: &super::ScoPath, s: f32) -> Vec2 {
    let h = p.heading_deg.to_radians();
    let r = p.radius_m;
    // Straight: radius 0 or the "−999…" marker.
    if r.abs() < 1e-3 || r <= -900.0 {
        Vec2::new(p.start.x + s * h.sin(), p.start.y + s * h.cos())
    } else {
        let h2 = h + s / r;
        Vec2::new(
            p.start.x - r * (h2.cos() - h.cos()),
            p.start.y + r * (h2.sin() - h.sin()),
        )
    }
}

impl SceneryObject {
    /// The object with its `[path]`s and `[splinehelper]` arms bent over `base`: an arm is
    /// raised by the base height under it; a path's start height by the height under its start,
    /// and its two gradients so that its height profile (quadratic in the arc length) passes
    /// through the bent heights at its middle and end as well. The gradients are added in the
    /// unit the lanes read them in (rise per metre).
    #[must_use]
    pub fn bent(&self, base: &BaseSurface) -> Self {
        let mut out = self.clone();
        if base.is_empty() {
            return out;
        }
        for p in &mut out.paths {
            let len = p.length_m;
            let at = |s: f32| {
                let q = path_point(p, s);
                base.height(q.x, q.y)
            };
            let (b0, bm, b1) = (at(0.0), at(len * 0.5), at(len));
            p.start.z += b0;
            if len > 1e-3 {
                p.slope_start += (4.0 * bm - 3.0 * b0 - b1) / len;
                p.slope_end += (3.0 * b1 + b0 - 4.0 * bm) / len;
            }
        }
        for h in &mut out.spline_helpers {
            h.position.z += base.height(h.position.x, h.position.y);
        }
        out
    }
}

/// The ground faces of a crossing's (bent) meshes, mesh axes: near-level faces (`|n_y| ≥ 0.7`) of
/// at least 0.05 m² lying between 1.5 m below and 3 m above the base surface (0 without one).
/// The terrain is carved under them like under a spline's surface.
#[must_use]
pub fn crossing_ground_faces(meshes: &[O3d], base: Option<&BaseSurface>) -> Vec<[Vec3; 3]> {
    let mut out = Vec::new();
    for m in meshes {
        for t in &m.indices {
            let v = |i: u32| m.vertices.get(i as usize).map(|v| v.position);
            let (Some(a), Some(b), Some(c)) = (v(t[0]), v(t[1]), v(t[2])) else {
                continue;
            };
            let n = (b - a).cross(c - a);
            let len = n.length();
            if len < 1e-8
                || n.y.abs() / len < GROUND_FACE_MIN_NY
                || len * 0.5 < GROUND_FACE_MIN_AREA_M2
            {
                continue;
            }
            let centre = (a + b + c) / 3.0;
            let rel = centre.y - base.map_or(0.0, |s| s.height(centre.x, centre.z));
            if !(GROUND_FACE_BAND_M.0 < rel && rel < GROUND_FACE_BAND_M.1) {
                continue;
            }
            out.push([a, b, c]);
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::formats::o3d::Vertex;
    use crate::formats::scenery::{ScoPath, ScoSplineHelper};

    /// A 40 × 40 m base rising 2 m from z = −20 to z = +20 (x across).
    fn tilted() -> BaseSurface {
        let p = |x: f32, z: f32| Vec3::new(x, 1.0 + z / 20.0, z);
        BaseSurface::from_triangles(vec![
            [p(-20.0, -20.0), p(20.0, -20.0), p(-20.0, 20.0)],
            [p(20.0, -20.0), p(20.0, 20.0), p(-20.0, 20.0)],
        ])
    }

    fn mesh(points: &[Vec3]) -> O3d {
        O3d {
            version: 3,
            header_flags: 0,
            vertices: points
                .iter()
                .map(|&position| Vertex {
                    position,
                    normal: Vec3::Y,
                    uv: Vec2::ZERO,
                    color: None,
                })
                .collect(),
            indices: vec![[0, 1, 2]],
            face_materials: vec![0],
            materials: Vec::new(),
            transform: None,
            encryption_key: None,
            exact_decoded: false,
            unknown_chunks: Vec::new(),
        }
    }

    #[test]
    fn the_keyword_names_the_base_mesh() {
        let text = "[crossing_heightdeformation]\r\nKreuz_base.o3d\r\n\r\n[surface]\r\n";
        let sco = SceneryObject::parse(text.as_bytes()).expect("parses");
        assert_eq!(sco.crossing_deformation.as_deref(), Some("Kreuz_base.o3d"));
        assert!(sco.surface);
    }

    #[test]
    fn the_base_surface_bends_vertices_and_is_zero_outside() {
        let base = tilted();
        assert!((base.height(0.0, 0.0) - 1.0).abs() < 1e-5);
        assert!((base.height(5.0, 10.0) - 1.5).abs() < 1e-5);
        assert_eq!(base.height(30.0, 0.0), 0.0);
        let mut m = mesh(&[
            Vec3::new(0.0, 0.1, -20.0),
            Vec3::new(0.0, 0.1, 20.0),
            Vec3::new(50.0, 0.1, 0.0),
        ]);
        base.bend_mesh(&mut m);
        let y: Vec<f32> = m.vertices.iter().map(|v| v.position.y).collect();
        assert!((y[0] - 0.1).abs() < 1e-5 && (y[1] - 2.1).abs() < 1e-5);
        assert!(
            (y[2] - 0.1).abs() < 1e-5,
            "outside the footprint: unchanged"
        );
    }

    #[test]
    fn paths_and_arms_follow_the_base() {
        let base = tilted();
        let mut sco = SceneryObject::default();
        // A straight path north from (0, −10) for 20 m, 0.1 m up, flat as written.
        sco.paths.push(ScoPath {
            start: Vec3::new(0.0, -10.0, 0.1),
            heading_deg: 0.0,
            radius_m: 0.0,
            length_m: 20.0,
            slope_start: 0.0,
            slope_end: 0.0,
            path_type: 0,
            width_m: 3.0,
            direction: 0,
            extra: None,
            traffic_light_index: None,
            approach_distance_m: None,
            switch_position: None,
        });
        sco.spline_helpers.push(ScoSplineHelper {
            profile_file: "a.sli".into(),
            position: Vec3::new(0.0, 20.0, 0.0),
            rotation_deg: 180.0,
            extra: Vec::new(),
        });
        let bent = sco.bent(&base);
        let p = &bent.paths[0];
        // Height along the path: z0 + s·(g0 + (g1 − g0)·t/2).
        let z = |s: f32| p.start.z + s * (p.slope_start + (p.slope_end - p.slope_start) * s / 40.0);
        assert!((z(0.0) - 0.6).abs() < 1e-5);
        assert!((z(10.0) - 1.1).abs() < 1e-4);
        assert!((z(20.0) - 1.6).abs() < 1e-4);
        assert!((bent.spline_helpers[0].position.z - 2.0).abs() < 1e-5);
        // Without a base nothing changes.
        assert_eq!(sco.bent(&BaseSurface::default()), sco);
    }

    #[test]
    fn ground_faces_are_level_faces_near_the_base() {
        let base = tilted();
        let floor = mesh(&[
            Vec3::new(0.0, 1.1, 0.0),
            Vec3::new(0.0, 1.1, 2.0),
            Vec3::new(2.0, 1.1, 0.0),
        ]);
        let wall = mesh(&[
            Vec3::new(0.0, 0.0, 0.0),
            Vec3::new(0.0, 2.0, 0.0),
            Vec3::new(2.0, 0.0, 0.0),
        ]);
        let roof = mesh(&[
            Vec3::new(0.0, 9.0, 0.0),
            Vec3::new(0.0, 9.0, 2.0),
            Vec3::new(2.0, 9.0, 0.0),
        ]);
        let faces = crossing_ground_faces(&[floor, wall, roof], Some(&base));
        assert_eq!(faces.len(), 1);
    }
}
