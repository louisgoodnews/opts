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
    remaining -= hinge_pass(o, &mut done, &pairs, &mut report);
    remaining -= smooth_pass(o, &mut done, &pairs, &mut report);
    report.unresolved = remaining;
    report
}

/// Two triangles sharing an edge: `x` and `d` are the apexes, `b`/`c` the shared edge.
struct Hinge {
    v: [u32; 4], // b, c, x, d
    /// Weights predicting `uv(x)` from `b, c, d` (they sum to 1).
    wx: [f32; 3],
    /// Weights predicting `uv(d)` from `b, c, x`.
    wd: [f32; 3],
}

/// Barycentric-style weights predicting the apex `x` across edge `bc` from the triangle `b, c, d`
/// lying on the other side (the mapping is taken to be affine across the hinge).
fn hinge_weights(pb: Vec3, pc: Vec3, pd: Vec3, px: Vec3) -> Option<[f32; 3]> {
    let len = (pc - pb).length();
    if len < 1e-6 {
        return None;
    }
    let e = (pc - pb) / len;
    let rd = pd - pb;
    let sd = rd.dot(e);
    let td = (rd - e * sd).length();
    if td < 1e-3 * len {
        return None;
    }
    let rx = px - pb;
    let sx = rx.dot(e);
    let tx = -(rx - e * sx).length();
    let wc = sx / len - tx * sd / (td * len);
    let wd = tx / td;
    let wb = 1.0 - wc - wd;
    let w = [wb, wc, wd];
    w.iter().all(|x| x.abs() <= 8.0).then_some(w)
}

/// -ln P(offset) per entry of jitter_pairs(), measured on 785 k twin vertices: files whose vertex
/// count is not a multiple of 5 (u-jitter needs 5 | q) and files whose count is.
const PRIOR_NOT5: [f32; 79] = [
    0.59, 5.72, 6.35, 5.12, 5.83, 5.67, 6.36, 4.31, 4.97, 5.67, 6.39, 5.11, 5.8, 5.65, 6.33, 4.26,
    4.98, 5.69, 6.39, 4.74, 13.38, 5.64, 6.43, 4.28, 4.98, 5.71, 6.35, 5.13, 5.81, 5.7, 6.35, 4.3,
    4.98, 5.63, 6.37, 5.14, 5.78, 5.68, 6.39, 5.7, 4.63, 5.69, 4.63, 4.63, 5.68, 4.64, 5.67, 5.71,
    4.6, 5.61, 4.63, 4.66, 5.67, 4.61, 5.74, 5.68, 4.59, 5.71, 4.6, 4.6, 5.66, 4.62, 5.7, 5.71,
    4.62, 5.63, 4.61, 4.6, 5.72, 4.63, 5.63, 5.65, 4.59, 5.69, 4.6, 4.59, 5.67, 4.61, 5.7,
];
const PRIOR_FIVE: [f32; 79] = [
    1.5, 4.68, 5.29, 3.77, 4.51, 4.68, 5.37, 2.98, 3.61, 3.5, 4.27, 3.75, 4.47, 4.63, 5.32, 2.93,
    3.66, 4.68, 5.26, 2.22, 11.85, 4.66, 5.42, 2.94, 3.61, 4.62, 5.32, 3.69, 4.46, 3.52, 4.14,
    2.93, 3.6, 4.63, 5.32, 3.66, 4.31, 4.67, 5.3, 11.85, 11.85, 11.85, 11.85, 11.85, 11.85, 11.85,
    11.85, 11.85, 11.85, 11.85, 11.85, 11.85, 11.85, 11.85, 11.85, 11.85, 11.85, 11.85, 11.85,
    11.85, 11.85, 11.85, 11.85, 11.85, 11.85, 11.85, 11.85, 11.85, 11.85, 11.85, 11.85, 11.85,
    11.85, 11.85, 11.85, 11.85, 11.85, 11.85, 11.85,
];

const SMOOTH_SWEEPS: usize = 80;
/// Width of the robust residual (texture units).
const SMOOTH_SIGMA: f32 = 0.01;
/// Weight of the measured offset prior against the hinge residuals.
const SMOOTH_PRIOR: f32 = 0.4;
/// Starting temperature of the Gibbs sampling.
const SMOOTH_T0: f32 = 0.6;

/// Fourth pass: Gibbs sampling over the jitter offset of every still unresolved vertex. Energy:
/// per hinge, how badly the (corrected) texture coordinates of one apex disagree with the affine
/// map of the neighbouring triangle (robust), plus a prior favouring "no jitter" (about half of
/// all vertices). Returns the number of vertices settled.
fn smooth_pass(
    o: &mut O3d,
    done: &mut [bool],
    pairs: &[(f32, f32)],
    report: &mut UvFixReport,
) -> usize {
    use std::collections::HashMap;
    let n = o.vertices.len();
    let unresolved: Vec<usize> = (0..n).filter(|&i| !done[i]).collect();
    if unresolved.is_empty() {
        return 0;
    }
    let pos: Vec<Vec3> = o.vertices.iter().map(|v| v.position).collect();
    let mut edge_faces: HashMap<(u32, u32), Vec<u32>> = HashMap::new();
    for (fi, f) in o.indices.iter().enumerate() {
        for k in 0..3 {
            let (a, b) = (f[k], f[(k + 1) % 3]);
            edge_faces
                .entry((a.min(b), a.max(b)))
                .or_default()
                .push(fi as u32);
        }
    }
    let mut hinges: Vec<Hinge> = Vec::new();
    for (&(b, c), faces) in &edge_faces {
        if faces.len() != 2 || b as usize >= n || c as usize >= n {
            continue;
        }
        let apex = |fi: u32| {
            o.indices[fi as usize]
                .iter()
                .copied()
                .find(|&v| v != b && v != c)
        };
        let (Some(x), Some(d)) = (apex(faces[0]), apex(faces[1])) else {
            continue;
        };
        if x as usize >= n || d as usize >= n || x == d {
            continue;
        }
        let (pb, pc, px, pd) = (
            pos[b as usize],
            pos[c as usize],
            pos[x as usize],
            pos[d as usize],
        );
        let (Some(wx), Some(wd)) = (hinge_weights(pb, pc, pd, px), hinge_weights(pb, pc, px, pd))
        else {
            continue;
        };
        // Only hinges that touch an unresolved vertex matter.
        if [b, c, x, d].iter().any(|&v| !done[v as usize]) {
            hinges.push(Hinge {
                v: [b, c, x, d],
                wx,
                wd,
            });
        }
    }
    let mut of: Vec<Vec<u32>> = vec![Vec::new(); n];
    for (hi, h) in hinges.iter().enumerate() {
        for &v in &h.v {
            of[v as usize].push(hi as u32);
        }
    }
    let stored: Vec<[f32; 2]> = o.vertices.iter().map(|v| [v.uv.x, v.uv.y]).collect();
    let mut k = vec![0usize; n]; // chosen offset index per vertex
    let off = |kk: usize| pairs[kk];
    let uv_of = |k: &[usize], i: usize| {
        let (ju, jv) = off(k[i]);
        [stored[i][0] - ju, stored[i][1] - jv]
    };
    let table: &[f32; 79] = if n.is_multiple_of(5) {
        &PRIOR_FIVE
    } else {
        &PRIOR_NOT5
    };
    let (sweeps, sigma, prior, t0) = (SMOOTH_SWEEPS, SMOOTH_SIGMA, SMOOTH_PRIOR, SMOOTH_T0);
    let robust = |r: [f32; 2]| {
        let q = r[0] * r[0] + r[1] * r[1];
        q / (q + sigma * sigma)
    };
    let hinge_energy = |k: &[usize], h: &Hinge| {
        let [b, c, x, d] = h.v.map(|v| v as usize);
        let (ub, uc, ux, ud) = (uv_of(k, b), uv_of(k, c), uv_of(k, x), uv_of(k, d));
        let mix = |w: [f32; 3], p: [f32; 2], q: [f32; 2], r: [f32; 2]| {
            [
                w[0] * p[0] + w[1] * q[0] + w[2] * r[0],
                w[0] * p[1] + w[1] * q[1] + w[2] * r[1],
            ]
        };
        let px = mix(h.wx, ub, uc, ud);
        let pd = mix(h.wd, ub, uc, ux);
        robust([ux[0] - px[0], ux[1] - px[1]]) + robust([ud[0] - pd[0], ud[1] - pd[1]])
    };
    let mut rng = 0x9E37_79B9_u32 ^ n as u32;
    let mut next = || {
        rng ^= rng << 13;
        rng ^= rng >> 17;
        rng ^= rng << 5;
        rng
    };
    let mut order = unresolved.clone();
    for sweep in 0..sweeps {
        let t = (t0 * (1.0 - sweep as f32 / sweeps as f32)).max(0.0);
        for i in (1..order.len()).rev() {
            order.swap(i, next() as usize % (i + 1));
        }
        for &i in &order {
            let mut energies = vec![0.0f32; pairs.len()];
            for (kk, e) in energies.iter_mut().enumerate() {
                k[i] = kk;
                *e = prior * table[kk]
                    + of[i]
                        .iter()
                        .map(|&hi| hinge_energy(&k, &hinges[hi as usize]))
                        .sum::<f32>();
            }
            let min = energies.iter().copied().fold(f32::MAX, f32::min);
            let pick = if t < 1e-3 {
                energies.iter().position(|&e| e == min).unwrap_or(0)
            } else {
                let w: Vec<f32> = energies.iter().map(|&e| ((min - e) / t).exp()).collect();
                let total: f32 = w.iter().sum();
                let mut r = (next() as f32 / u32::MAX as f32) * total;
                let mut chosen = 0;
                for (kk, wk) in w.iter().enumerate() {
                    r -= wk;
                    if r <= 0.0 {
                        chosen = kk;
                        break;
                    }
                }
                chosen
            };
            k[i] = pick;
        }
    }
    let mut settled = 0;
    for &i in &unresolved {
        let (ju, jv) = off(k[i]);
        if ju != 0.0 || jv != 0.0 {
            o.vertices[i].uv.x -= ju;
            o.vertices[i].uv.y -= jv;
            report.corrected += 1;
        } else {
            report.clean += 1;
        }
        done[i] = true;
        settled += 1;
    }
    settled
}

/// Tolerance of the hinge prediction (texture units).
const HINGE_TOL: f32 = 3.0e-3;

/// Third pass: a vertex whose neighbours are resolved gets its texture coordinates predicted by
/// unfolding its triangle across a shared edge into the plane of the neighbouring triangle and
/// evaluating that triangle's affine UV map there. The residual identifies the jitter when it
/// matches exactly one allowed offset (nothing is changed when predictions disagree, e.g. at
/// texture seams with a kinked mapping). Returns the number of vertices resolved.
fn hinge_pass(
    o: &mut O3d,
    done: &mut [bool],
    pairs: &[(f32, f32)],
    report: &mut UvFixReport,
) -> usize {
    use std::collections::HashMap;
    let n = o.vertices.len();
    let mut edge_faces: HashMap<(u32, u32), Vec<u32>> = HashMap::new();
    let mut vfaces: Vec<Vec<u32>> = vec![Vec::new(); n];
    for (fi, f) in o.indices.iter().enumerate() {
        for k in 0..3 {
            let (a, b) = (f[k], f[(k + 1) % 3]);
            edge_faces
                .entry((a.min(b), a.max(b)))
                .or_default()
                .push(fi as u32);
            if (a as usize) < n {
                vfaces[a as usize].push(fi as u32);
            }
        }
    }
    let pos: Vec<Vec3> = o.vertices.iter().map(|v| v.position).collect();
    let mut resolved = 0usize;
    for _sweep in 0..10 {
        let mut changed = 0usize;
        for a in 0..n {
            if done[a] {
                continue;
            }
            let mut preds: Vec<(f32, f32)> = Vec::new();
            for &fi in &vfaces[a] {
                let f = o.indices[fi as usize];
                let others: Vec<u32> = f.iter().copied().filter(|&v| v as usize != a).collect();
                if others.len() != 2 {
                    continue;
                }
                let (b, c) = (others[0], others[1]);
                if b as usize >= n || c as usize >= n || !done[b as usize] || !done[c as usize] {
                    continue;
                }
                let Some(list) = edge_faces.get(&(b.min(c), b.max(c))) else {
                    continue;
                };
                for &gi in list {
                    if gi == fi {
                        continue;
                    }
                    let g = o.indices[gi as usize];
                    let Some(&d) = g.iter().find(|&&v| v != b && v != c) else {
                        continue;
                    };
                    if d as usize >= n || !done[d as usize] {
                        continue;
                    }
                    let (pb, pc, pd, pa) =
                        (pos[b as usize], pos[c as usize], pos[d as usize], pos[a]);
                    let len = (pc - pb).length();
                    if len < 1e-6 {
                        continue;
                    }
                    let e = (pc - pb) / len;
                    let rd = pd - pb;
                    let sd = rd.dot(e);
                    let td = (rd - e * sd).length();
                    if td < 1e-4 * len {
                        continue;
                    }
                    let ra = pa - pb;
                    let sa = ra.dot(e);
                    let ta = -(ra - e * sa).length();
                    let (ub, uc, ud) = (
                        o.vertices[b as usize].uv,
                        o.vertices[c as usize].uv,
                        o.vertices[d as usize].uv,
                    );
                    let da = (uc - ub) / len;
                    let db = (ud - ub - da * sd) / td;
                    let pred = ub + da * sa + db * ta;
                    preds.push((pred.x, pred.y));
                }
            }
            if preds.is_empty() {
                continue;
            }
            let (u, v) = (o.vertices[a].uv.x, o.vertices[a].uv.y);
            // Offsets each prediction supports (None: residual matches no offset).
            let mut votes: Vec<Option<usize>> = Vec::new();
            for &(pu, pv) in &preds {
                let (ru, rv) = (u - pu, v - pv);
                let mut best: Option<(usize, f32)> = None;
                let mut second = f32::MAX;
                for (k, &(ju, jv)) in pairs.iter().enumerate() {
                    let d = (ru - ju).abs().max((rv - jv).abs());
                    match best {
                        Some((_, bd)) if d >= bd => second = second.min(d),
                        _ => {
                            if let Some((_, bd)) = best {
                                second = second.min(bd);
                            }
                            best = Some((k, d));
                        }
                    }
                }
                votes.push(match best {
                    Some((k, d)) if d <= HINGE_TOL && second > 2.0 * HINGE_TOL => Some(k),
                    _ => None,
                });
            }
            let matched: Vec<usize> = votes.iter().flatten().copied().collect();
            let Some(&first) = matched.first() else {
                continue;
            };
            if matched.iter().any(|&k| k != first) {
                continue;
            }
            let (ju, jv) = pairs[first];
            if ju != 0.0 || jv != 0.0 {
                o.vertices[a].uv.x -= ju;
                o.vertices[a].uv.y -= jv;
                report.corrected += 1;
            } else {
                report.clean += 1;
            }
            done[a] = true;
            changed += 1;
        }
        resolved += changed;
        if changed == 0 {
            break;
        }
    }
    resolved
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::formats::o3d::Vertex;
    use glam::{Vec2, Vec3};

    /// A planar grid with affine texture coordinates; about half of the
    /// vertices get an allowed offset. The planar and hinge passes must restore all of them.
    fn bent_grid() -> (O3d, Vec<Vec2>) {
        let (cols, rows) = (7usize, 6usize);
        let mut vertices = Vec::new();
        let mut truth = Vec::new();
        for j in 0..rows {
            for i in 0..cols {
                let (x, y) = (i as f32 * 0.5, j as f32 * 0.5);
                let pos = Vec3::new(x, y, 0.0);
                let uv = Vec2::new(x * 0.31 + 0.07, y * 0.23 + 0.11);
                truth.push(uv);
                vertices.push(Vertex {
                    position: pos,
                    normal: Vec3::Z,
                    uv,
                    color: None,
                });
            }
        }
        let mut indices = Vec::new();
        for j in 0..rows - 1 {
            for i in 0..cols - 1 {
                let a = (j * cols + i) as u32;
                let (b, c, d) = (a + 1, a + cols as u32, a + cols as u32 + 1);
                indices.push([a, b, c]);
                indices.push([b, d, c]);
            }
        }
        let faces = indices.len();
        for (k, v) in vertices.iter_mut().enumerate() {
            // Allowed u offsets (k/100)^2 for k = 20 and 15; the rest stays clean.
            if k % 3 == 1 {
                v.uv.x += 0.04;
            } else if k % 5 == 2 {
                v.uv.x += 0.0225;
            }
        }
        (
            O3d {
                version: 7,
                header_flags: 0,
                vertices,
                indices,
                face_materials: vec![0; faces],
                materials: Vec::new(),
                transform: None,
                encryption_key: Some(1),
                descrambled: false,
                unknown_chunks: Vec::new(),
            },
            truth,
        )
    }

    #[test]
    fn jittered_texture_coordinates_are_restored() {
        let (mut o, truth) = bent_grid();
        let report = unjitter_uv(&mut o);
        assert_eq!(report.unresolved, 0);
        let bad: Vec<(usize, Vec2, Vec2)> = o
            .vertices
            .iter()
            .zip(&truth)
            .enumerate()
            .filter(|(_, (v, t))| (v.uv - **t).abs().max_element() >= 2e-4)
            .map(|(i, (v, t))| (i, v.uv, *t))
            .collect();
        assert!(bad.is_empty(), "{report:?} {bad:?}");
    }
}
