//! Removal of the scrambler's texture-coordinate jitter.
//!
//! About half of a keyed mesh's vertices carry an additive `(u, v)` offset taken from two small
//! discrete sets (`u += (5k/100)²`, `k < 20`; `v += (2k/100)²`, `k < 50`, see
//! `docs/src/formats/binary/o3d.md`, Addendum 4). The generator that decides which vertices are
//! affected is unknown, but the true texture mapping of a modelled surface is *affine within each
//! planar patch*: so the jitter is recovered by fitting plane-affine UV models (RANSAC) to
//! patches of coplanar vertices and accepting a vertex as "jittered" only when its residual equals
//! a member of both discrete sets.

#![allow(clippy::needless_range_loop)]

use glam::Vec3;

use super::O3d;

const JITTER_TOLERANCE: f32 = 1e-4;
const MIN_PATCH_VERTICES: usize = 4;
const MIN_CLEAN: usize = 3;
const POOL_ALL: bool = true;
const POOL_MIN: u32 = 3;
const MIN_CLEAN_PERCENT: usize = 40;
/// Work budget: `trials × vertices × rounds`.
const WORK_BUDGET: usize = 400_000_000;

/// Outcome of [`unjitter_uv`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct UvFixReport {
    /// Vertices whose jitter was identified and removed.
    pub corrected: usize,
    /// Vertices that fit a planar UV model without needing correction.
    pub clean: usize,
    /// Vertices that matched no model (their texture coordinates may still be jittered).
    pub unresolved: usize,
}

/// All `(du, dv)` offsets the scrambler can add. Both derive from one state value `q`:
/// `u += ((q mod 100)/100)²` when `q mod 5 = 0`, `v += ((2q mod 100)/100)²` when `q mod 3 = 0`,
/// so a vertex has no jitter, u only, v only (with `2q mod 100` not a multiple of 10), or both
/// with `m_v = 2·m_u mod 100`.
fn jitter_pairs() -> Vec<(f32, f32)> {
    let sq = |m: u32| (m as f32 / 100.0).powi(2);
    let mut out = vec![(0.0, 0.0)];
    for k in 1..20u32 {
        let mu = 5 * k;
        out.push((sq(mu), 0.0));
        out.push((sq(mu), sq((2 * mu) % 100)));
    }
    for mv in (2..100u32).step_by(2).filter(|m| m % 10 != 0) {
        out.push((0.0, sq(mv)));
    }
    out
}

struct Plane {
    origin: Vec3,
    n: Vec3,
    e1: Vec3,
    e2: Vec3,
    /// `u = a[0] + a[1] s + a[2] t`
    u: [f32; 3],
    v: [f32; 3],
}

fn solve3(m: [[f32; 3]; 3], rhs: [f32; 3]) -> Option<[f32; 3]> {
    let det = |a: [[f32; 3]; 3]| {
        a[0][0] * (a[1][1] * a[2][2] - a[1][2] * a[2][1])
            - a[0][1] * (a[1][0] * a[2][2] - a[1][2] * a[2][0])
            + a[0][2] * (a[1][0] * a[2][1] - a[1][1] * a[2][0])
    };
    let d = det(m);
    if d.abs() < 1e-12 {
        return None;
    }
    let mut out = [0.0; 3];
    for c in 0..3 {
        let mut mc = m;
        for r in 0..3 {
            mc[r][c] = rhs[r];
        }
        out[c] = det(mc) / d;
    }
    Some(out)
}

fn plane_from(p: [Vec3; 3], uv: [[f32; 2]; 3]) -> Option<Plane> {
    let n = (p[1] - p[0]).cross(p[2] - p[0]);
    if n.length_squared() < 1e-10 {
        return None;
    }
    let n = n.normalize();
    let e1 = (p[1] - p[0]).normalize();
    let e2 = n.cross(e1);
    let st = |q: Vec3| ((q - p[0]).dot(e1), (q - p[0]).dot(e2));
    let m: [[f32; 3]; 3] = std::array::from_fn(|k| {
        let (s, t) = st(p[k]);
        [1.0, s, t]
    });
    let u = solve3(m, [uv[0][0], uv[1][0], uv[2][0]])?;
    let v = solve3(m, [uv[0][1], uv[1][1], uv[2][1]])?;
    Some(Plane {
        origin: p[0],
        n,
        e1,
        e2,
        u,
        v,
    })
}

impl Plane {
    fn predict(&self, q: Vec3) -> (f32, f32) {
        let s = (q - self.origin).dot(self.e1);
        let t = (q - self.origin).dot(self.e2);
        (
            self.u[0] + self.u[1] * s + self.u[2] * t,
            self.v[0] + self.v[1] * s + self.v[2] * t,
        )
    }
}

/// Whether residual `(du, dv)` equals one of the offsets the scrambler can add.
fn is_jitter(du: f32, dv: f32, pairs: &[(f32, f32)], tol: f32) -> bool {
    pairs
        .iter()
        .any(|(a, b)| (du - a).abs() <= tol && (dv - b).abs() <= tol)
}

/// Removes texture jitter where a planar UV model proves it; returns what was done.
pub fn unjitter_uv(o: &mut O3d) -> UvFixReport {
    let n = o.vertices.len();
    let mut report = UvFixReport::default();
    if n < MIN_PATCH_VERTICES || o.indices.is_empty() {
        report.unresolved = n;
        return report;
    }
    let pos: Vec<Vec3> = o.vertices.iter().map(|v| v.position).collect();
    let (mut lo, mut hi) = (Vec3::splat(f32::MAX), Vec3::splat(f32::MIN));
    for p in &pos {
        lo = lo.min(*p);
        hi = hi.max(*p);
    }
    let plane_tol = ((hi - lo).length() * 2e-5).max(1e-5);
    let pairs = jitter_pairs();
    let mut done = vec![false; n];
    let mut remaining = n;
    let mut rng = 0x2545_F491_u32 ^ n as u32;
    let mut next = |bound: usize| {
        rng ^= rng << 13;
        rng ^= rng >> 17;
        rng ^= rng << 5;
        rng as usize % bound.max(1)
    };
    let trials = 40usize;
    let max_rounds = (WORK_BUDGET / (trials * n)).clamp(4, 400);
    for _ in 0..max_rounds {
        if remaining < MIN_PATCH_VERTICES {
            break;
        }
        let mut best: Option<(usize, Plane)> = None;
        for _ in 0..trials {
            let f = o.indices[next(o.indices.len())];
            let ids = [f[0] as usize, f[1] as usize, f[2] as usize];
            if ids.iter().any(|&i| i >= n || done[i]) {
                continue;
            }
            let uv: [[f32; 2]; 3] = std::array::from_fn(|k| {
                let t = o.vertices[ids[k]].uv;
                [t.x, t.y]
            });
            let Some(plane) = plane_from([pos[ids[0]], pos[ids[1]], pos[ids[2]]], uv) else {
                continue;
            };
            let (mut inliers, mut clean) = (0usize, 0usize);
            for i in 0..n {
                if done[i] || (pos[i] - plane.origin).dot(plane.n).abs() > plane_tol {
                    continue;
                }
                let (pu, pv) = plane.predict(pos[i]);
                let (ru, rv) = (o.vertices[i].uv.x - pu, o.vertices[i].uv.y - pv);
                if ru.abs() <= 6e-5 && rv.abs() <= 6e-5 {
                    inliers += 1;
                    clean += 1;
                } else if is_jitter(ru, rv, &pairs, JITTER_TOLERANCE) {
                    inliers += 1;
                }
            }
            // A patch must be mostly explained without correction: coincidental jitter matches
            // (texture-atlas offsets alias with the jitter sets) must not outvote the evidence.
            let score = if clean * 100 >= inliers * MIN_CLEAN_PERCENT && clean >= MIN_CLEAN {
                inliers
            } else {
                0
            };
            if best.as_ref().is_none_or(|b| score > b.0) {
                best = Some((score, plane));
            }
        }
        let Some((count, plane)) = best else {
            continue;
        };
        if count < MIN_PATCH_VERTICES {
            continue;
        }
        for i in 0..n {
            if done[i] || (pos[i] - plane.origin).dot(plane.n).abs() > plane_tol {
                continue;
            }
            let (pu, pv) = plane.predict(pos[i]);
            let (ru, rv) = (o.vertices[i].uv.x - pu, o.vertices[i].uv.y - pv);
            if ru.abs() <= 6e-5 && rv.abs() <= 6e-5 {
                done[i] = true;
                remaining -= 1;
                report.clean += 1;
            } else if is_jitter(ru, rv, &pairs, JITTER_TOLERANCE) {
                o.vertices[i].uv.x -= ru;
                o.vertices[i].uv.y -= rv;
                done[i] = true;
                remaining -= 1;
                report.corrected += 1;
            }
        }
    }
    // Second pass: texture coordinates of real models repeat (box and planar mappings share
    // u or v values). A leftover vertex is fixed when exactly one offset makes *both* of its
    // coordinates equal values already confirmed elsewhere.
    let key = |x: f32| (x * 1.0e5).round() as i64;
    for _pass in 0..6 {
        let mut pool_u: std::collections::HashMap<i64, u32> = std::collections::HashMap::new();
        let mut pool_v: std::collections::HashMap<i64, u32> = std::collections::HashMap::new();
        let before = remaining;
        for i in 0..n {
            if done[i] || POOL_ALL {
                *pool_u.entry(key(o.vertices[i].uv.x)).or_default() += 1;
                *pool_v.entry(key(o.vertices[i].uv.y)).or_default() += 1;
            }
        }
        if pool_u.len() >= 2 {
            for i in 0..n {
                if done[i] {
                    continue;
                }
                let (u, v) = (o.vertices[i].uv.x, o.vertices[i].uv.y);
                let mut hit: Option<(f32, f32)> = None;
                let mut ambiguous = false;
                for &(ju, jv) in &pairs {
                    let (cu, cv) = (u - ju, v - jv);
                    let mu = (key(cu) - 1..=key(cu) + 1)
                        .any(|k| pool_u.get(&k).is_some_and(|c| *c >= POOL_MIN));
                    let mv = (key(cv) - 1..=key(cv) + 1)
                        .any(|k| pool_v.get(&k).is_some_and(|c| *c >= POOL_MIN));
                    if mu && mv {
                        if ju == 0.0 && jv == 0.0 {
                            hit = None;
                            ambiguous = true; // already consistent as it stands
                            break;
                        }
                        if hit.is_some() {
                            ambiguous = true;
                            break;
                        }
                        hit = Some((ju, jv));
                    }
                }
                if let (Some((ju, jv)), false) = (hit, ambiguous) {
                    o.vertices[i].uv.x -= ju;
                    o.vertices[i].uv.y -= jv;
                    done[i] = true;
                    remaining -= 1;
                    report.corrected += 1;
                }
            }
        }
        if remaining == before {
            break;
        }
    }
    report.unresolved = remaining;
    report
}
