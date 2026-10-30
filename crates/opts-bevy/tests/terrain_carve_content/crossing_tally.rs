//! Tallies of the bent-crossing content test: ground below the terrain, arms off their spline.

use super::*;

/// What the bent crossings of the map add up to.
#[derive(Default)]
pub(super) struct Tally {
    pub(super) placements: i32,
    pub(super) area: f64,
    pub(super) below: f64,
    pub(super) arms: i32,
    pub(super) arm_misses: Vec<(String, f32, f32)>,
}

impl Tally {
    /// Ground faces: centroid and three inner points, area-weighted.
    pub(super) fn add_ground(
        &mut self,
        map: &Map,
        key: (i32, i32),
        o: &MapObjectInstance,
        (meshes, base): (&[opts_core::formats::o3d::O3d], &BaseSurface),
        terrain: &mut HashMap<(i32, i32), Option<TileTerrain>>,
    ) {
        for f in crossing_ground_faces(meshes, Some(base)) {
            let a = 0.5 * (f[1] - f[0]).cross(f[2] - f[0]).length();
            let c = (f[0] + f[1] + f[2]) / 3.0;
            for q in [
                c,
                (2.0 * c + f[0]) / 3.0,
                (2.0 * c + f[1]) / 3.0,
                (2.0 * c + f[2]) / 3.0,
            ] {
                let Some(g) = map.ground_at(key, place(o, q.x, q.z), terrain) else {
                    continue;
                };
                self.area += f64::from(a) / 4.0;
                if g - (o.position.z + q.y) > 0.02 {
                    self.below += f64::from(a) / 4.0;
                }
            }
        }
    }

    /// Arms against the spline ends of the tile and its neighbours (`bent`: the bent object).
    pub(super) fn add_arms(
        &mut self,
        map: &mut Map,
        key: (i32, i32),
        o: &MapObjectInstance,
        bent: &SceneryObject,
    ) {
        for h in &bent.spline_helpers {
            let at = place(o, h.position.x, h.position.y);
            let z = o.position.z + h.position.z;
            let mut best: Option<(f32, f32)> = None;
            for dy in -1..=1 {
                for dx in -1..=1 {
                    let other = (key.0 + dx, key.1 + dy);
                    let Some(t) = map.tile(other) else { continue };
                    let off = map.grid.neighbour_offset(key, other);
                    for s in &t.splines {
                        for p in [spline_pose(s, 0.0), spline_pose(s, s.length_m)] {
                            let d = (Vec2::new(p.x, p.y) + off).distance(at);
                            if best.is_none_or(|(bd, _)| d < bd) {
                                best = Some((d, p.z));
                            }
                        }
                    }
                }
            }
            if let Some((d, sz)) = best.filter(|(d, _)| *d < 0.5) {
                self.arms += 1;
                if (sz - z).abs() > 0.01 {
                    self.arm_misses.push((o.sco_file.clone(), d, sz - z));
                }
            }
        }
    }
}
