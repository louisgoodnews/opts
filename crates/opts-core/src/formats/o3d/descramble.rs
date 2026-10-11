//! Heuristic recovery of scrambled (keyed) `.o3d` vertex positions.
//!
//! The exporter's private scheme permutes the position components of individual vertices
//! (four permutations observed: identity 47 %, swap x/z 27 %, swap x/y 15 %, swap y/z 11 %, see
//! `docs/src/formats/binary/o3d.md`, Addendum 4). Without the generator, each vertex's
//! permutation is chosen so that the mesh becomes coherent:
//!
//! * **adjacency** — edges of the faces should be short;
//! * **coordinate pool** — coordinates of real models repeat along each axis (walls, floors),
//!   so a vertex prefers values (and value pairs) that many other vertices share;
//! * **face normals** — the sorted absolute components of a face's geometric normal must match
//!   those of the stored vertex normals (invariant under permutation and sign);
//! * **prior** — the observed permutation frequencies.
//!
//! The result is applied only when the quality gate (`GATE_MIN_COHERENCE`) accepts it; texture-coordinate jitter and
//! normals of scrambled files are *not* recoverable here (normals are recomputed by the renderer).

#![allow(clippy::needless_range_loop)]

use std::collections::HashMap;

use glam::Vec3;

use super::O3d;

/// The four permutations that occur: `fixed[k] = stored[P[k]]`.
const PERMS: [[usize; 3]; 4] = [[0, 1, 2], [2, 1, 0], [1, 0, 2], [0, 2, 1]];
/// `ln` of the observed permutation frequencies (0.47, 0.27, 0.15, 0.11).
const PRIOR: [f32; 4] = [-0.755, -1.309, -1.897, -2.207];
/// Coordinate quantisation for the pool (2 mm).
const POOL_SCALE: f32 = 500.0;
const ROUNDS: usize = 16;
const W_ADJ: f32 = 20.0;
const W_PAIR: f32 = 1.0;
const W_NORMAL: f32 = 6.0;
const W_BP: f32 = 20.0;
const ANNEAL_T0: f32 = 4.0;
const ANNEAL_ROUNDS: usize = 100;
/// Largest mesh the confident decoder attempts (vertices).
pub const CONFIDENT_MAX_VERTICES: usize = 40_000;
/// Minimum share of vertices on which two independent runs must agree.
pub const CONFIDENT_MIN_AGREEMENT: f32 = 0.98;
/// Independent runs tried before giving up.
pub const CONFIDENT_MAX_RUNS: u32 = 6;
const BP_ITERS: usize = 25;
const BP_TRUNC: f32 = 0.35;

/// Tunable parameters of the decoder. The defaults are the evaluated configuration; the library
/// never reads the environment (use [`DecodeParams::with_overrides`] from a binary or example).
#[derive(Debug, Clone, PartialEq)]
pub struct DecodeParams {
    /// Weight of the belief-propagation edge cost.
    pub bp_weight: f32,
    /// Truncation of the belief-propagation edge cost (share of the mesh diagonal).
    pub bp_trunc: f32,
    /// Belief-propagation sweeps.
    pub bp_iterations: usize,
    /// Starting temperature of the annealing.
    pub anneal_t0: f32,
    /// Annealing rounds.
    pub anneal_rounds: usize,
    /// Restarts of the plain [`descramble`] solver (best score wins).
    pub restarts: u32,
    /// Share of vertices two runs must agree on ([`decode_confident`]).
    pub min_agreement: f32,
    /// Share of triangles whose normal must fit the stored vertex normals ([`decode_confident`]).
    pub min_coherence: f32,
}

impl Default for DecodeParams {
    fn default() -> Self {
        Self {
            bp_weight: W_BP,
            bp_trunc: BP_TRUNC,
            bp_iterations: BP_ITERS,
            anneal_t0: ANNEAL_T0,
            anneal_rounds: ANNEAL_ROUNDS,
            restarts: 1,
            min_agreement: CONFIDENT_MIN_AGREEMENT,
            min_coherence: GATE_MIN_COHERENCE,
        }
    }
}

impl DecodeParams {
    /// Applies overrides from a key lookup (`BP_W`, `BP_T`, `BP_I`, `ANNEAL_T0`, `ANNEAL_R`,
    /// `OPTS_DESCRAMBLE_RESTARTS`, `OPTS_CONF_AGREE`); unparsable values are ignored. Pass
    /// `|k| std::env::var(k).ok()` to read the environment.
    #[must_use]
    pub fn with_overrides(mut self, get: impl Fn(&str) -> Option<String>) -> Self {
        let f = |k: &str| get(k).and_then(|v| v.parse::<f32>().ok());
        if let Some(v) = f("BP_W") {
            self.bp_weight = v;
        }
        if let Some(v) = f("BP_T") {
            self.bp_trunc = v;
        }
        if let Some(v) = f("BP_I") {
            self.bp_iterations = v as usize;
        }
        if let Some(v) = f("ANNEAL_T0") {
            self.anneal_t0 = v;
        }
        if let Some(v) = f("ANNEAL_R") {
            self.anneal_rounds = v as usize;
        }
        if let Some(v) = f("OPTS_DESCRAMBLE_RESTARTS") {
            self.restarts = v as u32;
        }
        if let Some(v) = f("OPTS_CONF_AGREE") {
            self.min_agreement = v;
        }
        if let Some(v) = f("OPTS_CONF_COH") {
            self.min_coherence = v;
        }
        self
    }

    /// Stable hash of all parameters: part of the on-disk cache version, so changing a parameter
    /// invalidates results decoded under the old one.
    #[must_use]
    pub fn fingerprint(&self) -> u32 {
        let mut h = 0x811C_9DC5_u32;
        for v in [
            self.bp_weight.to_bits(),
            self.bp_trunc.to_bits(),
            self.bp_iterations as u32,
            self.anneal_t0.to_bits(),
            self.anneal_rounds as u32,
            self.restarts,
            self.min_agreement.to_bits(),
            self.min_coherence.to_bits(),
        ] {
            for b in v.to_le_bytes() {
                h = (h ^ u32::from(b)).wrapping_mul(0x0100_0193);
            }
        }
        h
    }
}

/// Outcome of [`descramble`].
#[derive(Debug, Clone, PartialEq)]
pub struct DescrambleReport {
    /// Whether the quality gate accepted the solution (the mesh was modified).
    pub accepted: bool,
    /// Triangles kept after removing the ones the gate judged incoherent.
    pub kept_faces: usize,
    /// Triangles before the gate.
    pub total_faces: usize,
    /// Share of triangles that were coherent before dropping (0..=1).
    pub coherence: f32,
}

fn permuted(p: usize, v: Vec3) -> Vec3 {
    let a = [v.x, v.y, v.z];
    let q = PERMS[p];
    Vec3::new(a[q[0]], a[q[1]], a[q[2]])
}

fn sorted_abs(n: Vec3) -> [f32; 3] {
    let mut a = [n.x.abs(), n.y.abs(), n.z.abs()];
    a.sort_by(|x, y| x.total_cmp(y));
    a
}

fn quant(x: f32) -> i32 {
    (x * POOL_SCALE).round() as i32
}

struct Pool {
    single: [HashMap<i32, u32>; 3],
    pair: [HashMap<(i32, i32), u32>; 3],
}

const PAIRS: [(usize, usize); 3] = [(0, 1), (1, 2), (0, 2)];

impl Pool {
    fn new() -> Self {
        Self {
            single: Default::default(),
            pair: Default::default(),
        }
    }

    fn add(&mut self, p: Vec3, delta: i32) {
        let q = [quant(p.x), quant(p.y), quant(p.z)];
        for k in 0..3 {
            let e = self.single[k].entry(q[k]).or_insert(0);
            *e = e.saturating_add_signed(delta);
        }
        for (t, (a, b)) in PAIRS.iter().enumerate() {
            let e = self.pair[t].entry((q[*a], q[*b])).or_insert(0);
            *e = e.saturating_add_signed(delta);
        }
    }

    fn gain(&self, p: Vec3) -> f32 {
        let q = [quant(p.x), quant(p.y), quant(p.z)];
        let mut g = 0.0;
        for k in 0..3 {
            g += (self.single[k].get(&q[k]).copied().unwrap_or(0) as f32).ln_1p();
        }
        for (t, (a, b)) in PAIRS.iter().enumerate() {
            g += W_PAIR * (self.pair[t].get(&(q[*a], q[*b])).copied().unwrap_or(0) as f32).ln_1p();
        }
        g
    }
}

/// Loop belief propagation (min-sum) over the four labels with truncated-linear edge costs:
/// a good starting labelling for the local search. Messages are normalised each sweep.
fn bp_init(o: &O3d, p: &DecodeParams) -> Vec<u8> {
    let n = o.vertices.len();
    let stored: Vec<Vec3> = o.vertices.iter().map(|v| v.position).collect();
    let mut edges: Vec<(u32, u32)> = Vec::new();
    for f in &o.indices {
        for k in 0..3 {
            let (a, b) = (f[k], f[(k + 1) % 3]);
            if (a as usize) < n && (b as usize) < n && a != b {
                edges.push((a.min(b), a.max(b)));
            }
        }
    }
    edges.sort_unstable();
    edges.dedup();
    let (mut lo, mut hi) = (Vec3::splat(f32::MAX), Vec3::splat(f32::MIN));
    for p in &stored {
        lo = lo.min(*p);
        hi = hi.max(*p);
    }
    let diag = (hi - lo).length().max(1e-4);
    let (w_bp, trunc, iters) = (p.bp_weight, p.bp_trunc, p.bp_iterations);
    // cost[e][la][lb]
    let cost: Vec<[[f32; 4]; 4]> = edges
        .iter()
        .map(|&(a, b)| {
            let mut c = [[0.0; 4]; 4];
            for la in 0..4 {
                for lb in 0..4 {
                    let d = (permuted(la, stored[a as usize]) - permuted(lb, stored[b as usize]))
                        .length()
                        / diag;
                    c[la][lb] = d.min(trunc) * w_bp;
                }
            }
            c
        })
        .collect();
    let unary: [f32; 4] = [-PRIOR[0], -PRIOR[1], -PRIOR[2], -PRIOR[3]];
    // directed messages: 2 per edge, index 2e = a->b, 2e+1 = b->a
    let mut msg = vec![[0.0f32; 4]; edges.len() * 2];
    let mut incoming: Vec<Vec<usize>> = vec![Vec::new(); n]; // message ids arriving at node
    for (e, &(a, b)) in edges.iter().enumerate() {
        incoming[b as usize].push(2 * e);
        incoming[a as usize].push(2 * e + 1);
    }
    for _ in 0..iters {
        let mut next = msg.clone();
        for (e, &(a, b)) in edges.iter().enumerate() {
            for dir in 0..2 {
                let (from, to_mid) = if dir == 0 {
                    (a as usize, 2 * e)
                } else {
                    (b as usize, 2 * e + 1)
                };
                let back = if dir == 0 { 2 * e + 1 } else { 2 * e };
                let mut belief = unary;
                for &m in &incoming[from] {
                    if m != back {
                        for l in 0..4 {
                            belief[l] += msg[m][l];
                        }
                    }
                }
                let mut out = [0.0f32; 4];
                for lt in 0..4 {
                    let mut best = f32::MAX;
                    for lf in 0..4 {
                        let c = if dir == 0 {
                            cost[e][lf][lt]
                        } else {
                            cost[e][lt][lf]
                        };
                        best = best.min(belief[lf] + c);
                    }
                    out[lt] = best;
                }
                let m0 = out.iter().copied().fold(f32::MAX, f32::min);
                for l in 0..4 {
                    out[l] -= m0;
                }
                next[to_mid] = out;
            }
        }
        msg = next;
    }
    (0..n)
        .map(|i| {
            let mut belief = unary;
            for &m in &incoming[i] {
                for l in 0..4 {
                    belief[l] += msg[m][l];
                }
            }
            let mut best = 0;
            for l in 1..4 {
                if belief[l] < belief[best] {
                    best = l;
                }
            }
            best as u8
        })
        .collect()
}

/// Chooses a permutation per vertex; returns the labels (index into the four permutations).
fn solve(o: &O3d, p: &DecodeParams) -> Vec<u8> {
    let restarts = p.restarts;
    let mut best: Option<(f32, Vec<u8>)> = None;
    for r in 0..restarts.max(1) {
        let (lab, e) = solve_once(
            o,
            0x9E37_79B9_u32.wrapping_add(r.wrapping_mul(0x85EB_CA6B)),
            r > 0,
            p,
        );
        if best.as_ref().is_none_or(|b| e > b.0) {
            best = Some((e, lab));
        }
    }
    best.map(|b| b.1).unwrap_or_default()
}

/// One optimisation run; returns the labels and the total score reached (higher is better).
fn solve_once(o: &O3d, seed: u32, random_init: bool, p: &DecodeParams) -> (Vec<u8>, f32) {
    let init = if random_init {
        None
    } else {
        Some(bp_init(o, p))
    };
    run_local_search(o, seed, random_init, init, false, p)
}

/// Total score of a given labelling under the solver's own objective (evaluation tool).
#[must_use]
pub fn labels_score(o: &O3d, labels: &[u8]) -> f32 {
    run_local_search(
        o,
        1,
        false,
        Some(labels.to_vec()),
        true,
        &DecodeParams::default(),
    )
    .1
}

fn run_local_search(
    o: &O3d,
    seed: u32,
    random_init: bool,
    init: Option<Vec<u8>>,
    eval_only: bool,
    params: &DecodeParams,
) -> (Vec<u8>, f32) {
    let n = o.vertices.len();
    let stored: Vec<Vec3> = o.vertices.iter().map(|v| v.position).collect();
    let normal_key: Vec<[f32; 3]> = o.vertices.iter().map(|v| sorted_abs(v.normal)).collect();
    let mut adj: Vec<Vec<u32>> = vec![Vec::new(); n];
    let mut vfaces: Vec<Vec<u32>> = vec![Vec::new(); n];
    for (fi, f) in o.indices.iter().enumerate() {
        for k in 0..3 {
            let (a, b) = (f[k] as usize, f[(k + 1) % 3] as usize);
            if a < n && b < n {
                adj[a].push(b as u32);
                adj[b].push(a as u32);
            }
            if a < n {
                vfaces[a].push(fi as u32);
            }
        }
    }
    let (mut lo, mut hi) = (Vec3::splat(f32::MAX), Vec3::splat(f32::MIN));
    for p in &stored {
        lo = lo.min(*p);
        hi = hi.max(*p);
    }
    let diag2 = (hi - lo).length_squared().max(1e-6);

    let mut lab = init.unwrap_or_else(|| vec![0u8; n]);
    if random_init {
        let mut r = seed | 1;
        for l in &mut lab {
            r ^= r << 13;
            r ^= r >> 17;
            r ^= r << 5;
            *l = (r % 4) as u8;
        }
    }
    let mut pool = Pool::new();
    for (i, p) in stored.iter().enumerate() {
        pool.add(permuted(lab[i] as usize, *p), 1);
    }
    let pos = |lab: &[u8], i: usize| permuted(lab[i] as usize, stored[i]);
    let mut order: Vec<u32> = (0..n as u32).collect();
    let mut rng = seed;
    let mut total = 0.0f32;
    let (t0, anneal_rounds) = if eval_only {
        (0.0, 0)
    } else {
        (params.anneal_t0, params.anneal_rounds)
    };
    for round in 0..(anneal_rounds + ROUNDS) {
        let temp = if round < anneal_rounds {
            t0 * (1.0 - round as f32 / anneal_rounds as f32)
        } else {
            0.0
        };
        // xorshift shuffle keeps the solver deterministic.
        for i in (1..order.len()).rev() {
            rng ^= rng << 13;
            rng ^= rng >> 17;
            rng ^= rng << 5;
            order.swap(i, rng as usize % (i + 1));
        }
        let mut changed = 0;
        total = 0.0;
        for &iu in &order {
            let i = iu as usize;
            pool.add(pos(&lab, i), -1);
            let mut best = (f32::MIN, lab[i]);
            let mut scores = [f32::MIN; 4];
            for p in 0..4 {
                if eval_only && p != lab[i] as usize {
                    continue;
                }
                let cand = permuted(p, stored[i]);
                let mut score = pool.gain(cand) + PRIOR[p];
                if !adj[i].is_empty() {
                    let adjc: f32 = adj[i]
                        .iter()
                        .map(|&j| (cand - pos(&lab, j as usize)).length_squared())
                        .sum::<f32>()
                        / adj[i].len() as f32;
                    score -= W_ADJ * adjc / diag2;
                }
                if !vfaces[i].is_empty() {
                    let mut c = 0.0;
                    for &fi in &vfaces[i] {
                        let f = o.indices[fi as usize];
                        let ps: Vec<Vec3> = f
                            .iter()
                            .map(|&v| {
                                if v as usize == i {
                                    cand
                                } else {
                                    pos(&lab, v as usize)
                                }
                            })
                            .collect();
                        let nrm = (ps[1] - ps[0]).cross(ps[2] - ps[0]);
                        if nrm.length_squared() < 1e-12 {
                            continue;
                        }
                        let sf = sorted_abs(nrm.normalize());
                        for &v in &f {
                            if let Some(sn) = normal_key.get(v as usize) {
                                c += (0..3).map(|k| (sf[k] - sn[k]).powi(2)).sum::<f32>();
                            }
                        }
                    }
                    score -= W_NORMAL * c / vfaces[i].len() as f32;
                }
                scores[p] = score;
                if score > best.0 + 1e-6 {
                    best = (score, p as u8);
                }
            }
            if temp > 1e-4 {
                let mx = scores.iter().copied().fold(f32::MIN, f32::max);
                let w: [f32; 4] = std::array::from_fn(|k| {
                    if scores[k] == f32::MIN {
                        0.0
                    } else {
                        ((scores[k] - mx) / temp).exp()
                    }
                });
                let sum: f32 = w.iter().sum();
                rng ^= rng << 13;
                rng ^= rng >> 17;
                rng ^= rng << 5;
                let mut r = (rng as f32 / u32::MAX as f32) * sum;
                for k in 0..4 {
                    r -= w[k];
                    if r <= 0.0 && w[k] > 0.0 {
                        best = (scores[k], k as u8);
                        break;
                    }
                }
            }
            total += best.0;
            if best.1 != lab[i] {
                lab[i] = best.1;
                changed += 1;
            }
            pool.add(pos(&lab, i), 1);
        }
        if eval_only || (changed == 0 && temp == 0.0) {
            break;
        }
    }
    (lab, total)
}

/// Share of triangles whose edges are coherent (no edge more than `LONG_EDGE` times the
/// median edge) and the per-triangle verdicts.
const LONG_EDGE: f32 = 6.0;

/// Per-triangle coherence verdicts and the coherent share (also used by evaluation tools).
#[must_use]
pub fn triangle_verdicts(o: &O3d) -> (Vec<bool>, f32) {
    let edge = |a: u32, b: u32| match (o.vertices.get(a as usize), o.vertices.get(b as usize)) {
        (Some(p), Some(q)) => (p.position - q.position).length(),
        _ => f32::MAX,
    };
    let mut lens: Vec<f32> = o
        .indices
        .iter()
        .flat_map(|f| [edge(f[0], f[1]), edge(f[1], f[2]), edge(f[2], f[0])])
        .filter(|l| l.is_finite())
        .collect();
    if lens.is_empty() {
        return (vec![false; o.indices.len()], 0.0);
    }
    lens.sort_by(|a, b| a.total_cmp(b));
    let median = lens[lens.len() / 2].max(1e-4);
    let verdicts: Vec<bool> = o
        .indices
        .iter()
        .map(|f| {
            let m = edge(f[0], f[1]).max(edge(f[1], f[2])).max(edge(f[2], f[0]));
            m <= LONG_EDGE * median
        })
        .collect();
    let good = verdicts.iter().filter(|v| **v).count() as f32 / verdicts.len().max(1) as f32;
    (verdicts, good)
}

/// Minimum share of coherent triangles for a solution to be accepted.
pub const GATE_MIN_COHERENCE: f32 = 0.9;

/// Decodes `o` in place when the quality gate accepts the solution; otherwise leaves it untouched.
///
/// Accepted meshes lose the triangles that are still incoherent (holes instead of spikes).
pub fn descramble(o: &mut O3d) -> DescrambleReport {
    descramble_with(o, &DecodeParams::default())
}

/// [`descramble`] with explicit parameters.
pub fn descramble_with(o: &mut O3d, params: &DecodeParams) -> DescrambleReport {
    let total = o.indices.len();
    if o.vertices.len() < 4 || total < 2 {
        return DescrambleReport {
            accepted: false,
            kept_faces: total,
            total_faces: total,
            coherence: 0.0,
        };
    }
    let labels = solve(o, params);
    let mut trial = o.clone();
    for (v, &l) in trial.vertices.iter_mut().zip(&labels) {
        v.position = permuted(l as usize, v.position);
    }
    let (verdicts, coherence) = triangle_verdicts(&trial);
    if coherence < GATE_MIN_COHERENCE {
        return DescrambleReport {
            accepted: false,
            kept_faces: total,
            total_faces: total,
            coherence,
        };
    }
    let (mut idx, mut mats) = (Vec::new(), Vec::new());
    for ((f, m), ok) in trial
        .indices
        .iter()
        .zip(&trial.face_materials)
        .zip(&verdicts)
    {
        if *ok {
            idx.push(*f);
            mats.push(*m);
        }
    }
    let kept = idx.len();
    trial.indices = idx;
    trial.face_materials = mats;
    *o = trial;
    DescrambleReport {
        accepted: true,
        kept_faces: kept,
        total_faces: total,
        coherence,
    }
}

/// Share of triangles whose geometric normal agrees (sorted absolute components, tolerance
/// `tol`) with the stored normals of all three vertices. Invariant under the scrambling of
/// normals (permutation and sign), so it measures whether *positions* fit the file.
#[must_use]
pub fn normal_agreement(o: &O3d, tol: f32) -> f32 {
    let mut ok = 0usize;
    let mut total = 0usize;
    for f in &o.indices {
        let ps: Vec<Vec3> = f
            .iter()
            .filter_map(|&i| o.vertices.get(i as usize).map(|v| v.position))
            .collect();
        if ps.len() != 3 {
            continue;
        }
        let n = (ps[1] - ps[0]).cross(ps[2] - ps[0]);
        if n.length_squared() < 1e-12 {
            continue;
        }
        total += 1;
        let sf = sorted_abs(n.normalize());
        let agrees = f.iter().all(|&i| {
            let sn = sorted_abs(o.vertices[i as usize].normal);
            (0..3).all(|k| (sf[k] - sn[k]).abs() <= tol)
        });
        ok += usize::from(agrees);
    }
    if total == 0 {
        0.0
    } else {
        ok as f32 / total as f32
    }
}

/// Evaluation helper: one independent run (`run` selects the random seed; run 0 starts from
/// belief propagation, later runs from random labels).
#[must_use]
pub fn solved_labels_run(o: &O3d, run: u32) -> (Vec<u8>, f32) {
    solved_labels_run_with(o, run, &DecodeParams::default())
}

/// [`solved_labels_run`] with explicit parameters.
#[must_use]
pub fn solved_labels_run_with(o: &O3d, run: u32, params: &DecodeParams) -> (Vec<u8>, f32) {
    solve_once(
        o,
        0x9E37_79B9_u32.wrapping_add(run.wrapping_mul(0x85EB_CA6B)),
        run > 0,
        params,
    )
}

/// Test/evaluation helper: the chosen label (index into the four permutations) per vertex.
#[must_use]
pub fn solved_labels(o: &O3d) -> Vec<u8> {
    solve(o, &DecodeParams::default())
}

/// Test/evaluation helper: the labels the solver would choose, applied to `o`'s positions.
#[must_use]
pub fn solved_positions(o: &O3d) -> Vec<Vec3> {
    let labels = solve(o, &DecodeParams::default());
    o.vertices
        .iter()
        .zip(labels)
        .map(|(v, l)| permuted(l as usize, v.position))
        .collect()
}

/// Outcome of [`decode_confident`].
#[derive(Debug, Clone, PartialEq)]
pub struct ConfidentReport {
    /// Whether the mesh was decoded and modified.
    pub accepted: bool,
    /// Share of vertices on which the two independent runs agreed.
    pub agreement: f32,
    /// Share of triangles that were coherent after decoding.
    pub coherence: f32,
    /// Triangles dropped because they stayed incoherent.
    pub dropped_faces: usize,
    /// Texture-jitter report of the UV step.
    pub uv: super::uvfix::UvFixReport,
}

/// Decodes a scrambled mesh without a plain twin when the answer is trustworthy.
///
/// Two independent annealing runs (one seeded by belief propagation, one from random labels) must
/// agree on [`CONFIDENT_MIN_AGREEMENT`] of the vertices (on plain/keyed twins, runs that agree
/// to 98 % are exact to 98 % in 92 % of the cases) and the decoded triangles must be coherent.
/// Accepted meshes lose their remaining incoherent triangles and get their texture jitter
/// removed where a planar UV model proves it ([`super::uvfix::unjitter_uv`]).
pub fn decode_confident(o: &mut O3d) -> ConfidentReport {
    decode_confident_with(o, &DecodeParams::default())
}

/// [`decode_confident`] with explicit parameters.
pub fn decode_confident_with(o: &mut O3d, params: &DecodeParams) -> ConfidentReport {
    let rejected = |agreement: f32, coherence: f32| ConfidentReport {
        accepted: false,
        agreement,
        coherence,
        dropped_faces: 0,
        uv: super::uvfix::UvFixReport::default(),
    };
    if o.vertices.len() < 4 || o.indices.len() < 2 || o.vertices.len() > CONFIDENT_MAX_VERTICES {
        return rejected(0.0, 0.0);
    }
    // Runs agree on a vertex when both decodings put it in (practically) the same place; two
    // labels can differ yet give the same position when coordinates coincide.
    let (mut lo, mut hi) = (Vec3::splat(f32::MAX), Vec3::splat(f32::MIN));
    for v in &o.vertices {
        lo = lo.min(v.position);
        hi = hi.max(v.position);
    }
    let tol = ((hi - lo).length() * 0.005).max(1e-5);
    let agreement = |x: &[u8], y: &[u8]| {
        o.vertices
            .iter()
            .zip(x.iter().zip(y))
            .filter(|(v, (p, q))| {
                permuted(**p as usize, v.position).distance(permuted(**q as usize, v.position))
                    <= tol
            })
            .count() as f32
            / x.len() as f32
    };
    let min_agree = params.min_agreement;
    let mut runs: Vec<(Vec<u8>, f32)> = Vec::new();
    let mut chosen: Option<(Vec<u8>, f32)> = None;
    let mut best_pair = 0.0f32;
    'runs: for r in 0..CONFIDENT_MAX_RUNS {
        runs.push(solved_labels_run_with(o, r, params));
        for i in 0..runs.len() - 1 {
            let ag = agreement(&runs[i].0, &runs[runs.len() - 1].0);
            best_pair = best_pair.max(ag);
            if ag >= min_agree {
                let last = runs.len() - 1;
                let pick = if runs[i].1 >= runs[last].1 { i } else { last };
                chosen = Some((runs[pick].0.clone(), ag));
                break 'runs;
            }
        }
    }
    let Some((labels, agree)) = chosen else {
        return rejected(best_pair, 0.0);
    };
    let mut trial = o.clone();
    for (v, &l) in trial.vertices.iter_mut().zip(&labels) {
        v.position = permuted(l as usize, v.position);
    }
    let (verdicts, coherence) = triangle_verdicts(&trial);
    if coherence < params.min_coherence {
        return rejected(agree, coherence);
    }
    let total = trial.indices.len();
    let (mut idx, mut mats) = (Vec::new(), Vec::new());
    for ((f, m), ok) in trial
        .indices
        .iter()
        .zip(&trial.face_materials)
        .zip(&verdicts)
    {
        if *ok {
            idx.push(*f);
            mats.push(*m);
        }
    }
    let dropped = total - idx.len();
    trial.indices = idx;
    trial.face_materials = mats;
    let uv = super::uvfix::unjitter_uv(&mut trial);
    trial.descrambled = true;
    *o = trial;
    ConfidentReport {
        accepted: true,
        agreement: agree,
        coherence,
        dropped_faces: dropped,
        uv,
    }
}

#[cfg(test)]
mod params_tests {
    use super::DecodeParams;

    #[test]
    fn overrides_apply_and_bad_values_are_ignored() {
        let p = DecodeParams::default().with_overrides(|k| match k {
            "BP_W" => Some("12.5".into()),
            "ANNEAL_R" => Some("40".into()),
            "OPTS_CONF_AGREE" => Some("nonsense".into()),
            _ => None,
        });
        assert_eq!(p.bp_weight, 12.5);
        assert_eq!(p.anneal_rounds, 40);
        assert_eq!(p.min_agreement, DecodeParams::default().min_agreement);
    }

    #[test]
    fn fingerprint_changes_with_every_parameter() {
        let base = DecodeParams::default();
        let variants = [
            DecodeParams {
                bp_weight: 1.0,
                ..base.clone()
            },
            DecodeParams {
                bp_trunc: 0.5,
                ..base.clone()
            },
            DecodeParams {
                bp_iterations: 3,
                ..base.clone()
            },
            DecodeParams {
                anneal_t0: 9.0,
                ..base.clone()
            },
            DecodeParams {
                anneal_rounds: 7,
                ..base.clone()
            },
            DecodeParams {
                restarts: 4,
                ..base.clone()
            },
            DecodeParams {
                min_agreement: 0.9,
                ..base.clone()
            },
        ];
        for v in &variants {
            assert_ne!(v.fingerprint(), base.fingerprint());
        }
        assert_eq!(base.fingerprint(), DecodeParams::default().fingerprint());
    }
}
