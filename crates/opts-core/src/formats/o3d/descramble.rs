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
/// Minimum share of vertices on which two independent runs must agree. On twins 0.98 accepts
/// 92 % of the meshes (86 % of them exact), 0.9 accepts 98 % (82 % exact): a mostly right door or
/// panel beats a missing one.
pub const CONFIDENT_MIN_AGREEMENT: f32 = 0.9;
/// Minimum share of coherent triangles for [`decode_confident`]. Off by default: on plain/keyed
/// twins the exact share stays at 85–86 % for every gate between 0.9 and 0.3 (coverage 68 % →
/// 92 %), i.e. the measure does not predict correctness.
pub const CONFIDENT_MIN_COHERENCE: f32 = 0.0;
/// Weight of the fold penalty (adjacent consistently wound triangles must not bend past 90°);
/// on twins 12 raises the share of exact decodes from 85.0 % to 86.3 %.
const FOLD_WEIGHT: f32 = 12.0;
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
    /// Weight of the fold penalty (adjacent consistently wound triangles bending past 90°).
    pub w_fold: f32,
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
            min_coherence: CONFIDENT_MIN_COHERENCE,
            w_fold: FOLD_WEIGHT,
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
        if let Some(v) = f("W_FOLD") {
            self.w_fold = v;
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
            self.w_fold.to_bits(),
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
fn bp_init(o: &O3d, p: &DecodeParams, allowed: Option<&[u8]>) -> Vec<u8> {
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
    let unary_of = |i: usize| -> [f32; 4] {
        std::array::from_fn(|l| {
            let banned = allowed.is_some_and(|a| a[i] & (1 << l) == 0);
            -PRIOR[l] + if banned { 1.0e4 } else { 0.0 }
        })
    };
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
                let mut belief = unary_of(from);
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
            let mut belief = unary_of(i);
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
    solve_masked(o, p, None)
}

fn solve_masked(o: &O3d, p: &DecodeParams, allowed: Option<&[u8]>) -> Vec<u8> {
    let restarts = p.restarts;
    let mut best: Option<(f32, Vec<u8>)> = None;
    for r in 0..restarts.max(1) {
        let (lab, e) = solve_once(
            o,
            0x9E37_79B9_u32.wrapping_add(r.wrapping_mul(0x85EB_CA6B)),
            r > 0,
            p,
            allowed,
        );
        if best.as_ref().is_none_or(|b| e > b.0) {
            best = Some((e, lab));
        }
    }
    best.map(|b| b.1).unwrap_or_default()
}

/// One optimisation run; returns the labels and the total score reached (higher is better).
fn solve_once(
    o: &O3d,
    seed: u32,
    random_init: bool,
    p: &DecodeParams,
    allowed: Option<&[u8]>,
) -> (Vec<u8>, f32) {
    let init = if random_init {
        None
    } else {
        Some(bp_init(o, p, allowed))
    };
    run_local_search(o, seed, random_init, init, false, p, allowed)
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
        None,
    )
    .1
}

/// Neighbourhood data and scoring of one vertex label under the solver's objective.
struct Scorer<'a> {
    o: &'a O3d,
    w_fold: f32,
    /// Faces sharing each undirected edge with the opposite direction (consistent winding).
    mates: HashMap<(u32, u32), Vec<(u32, bool)>>,
    stored: Vec<Vec3>,
    normal_key: Vec<[f32; 3]>,
    adj: Vec<Vec<u32>>,
    vfaces: Vec<Vec<u32>>,
    diag2: f32,
}

impl<'a> Scorer<'a> {
    fn new(o: &'a O3d, w_fold: f32) -> Self {
        let n = o.vertices.len();
        let mut mates: HashMap<(u32, u32), Vec<(u32, bool)>> = HashMap::new();
        for (fi, f) in o.indices.iter().enumerate() {
            for k in 0..3 {
                let (a, b) = (f[k], f[(k + 1) % 3]);
                if a != b {
                    mates
                        .entry((a.min(b), a.max(b)))
                        .or_default()
                        .push((fi as u32, a < b));
                }
            }
        }
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
        Self {
            o,
            w_fold,
            mates,
            stored,
            normal_key,
            adj,
            vfaces,
            diag2,
        }
    }

    fn pos(&self, lab: &[u8], i: usize) -> Vec3 {
        permuted(lab[i] as usize, self.stored[i])
    }

    /// Score of giving vertex `i` label `p` while the others keep `lab`; `pool` must not contain `i`.
    fn score(&self, pool: &Pool, lab: &[u8], i: usize, p: usize) -> f32 {
        let cand = permuted(p, self.stored[i]);
        let mut score = pool.gain(cand) + PRIOR[p];
        if !self.adj[i].is_empty() {
            let adjc: f32 = self.adj[i]
                .iter()
                .map(|&j| (cand - self.pos(lab, j as usize)).length_squared())
                .sum::<f32>()
                / self.adj[i].len() as f32;
            score -= W_ADJ * adjc / self.diag2;
        }
        if !self.vfaces[i].is_empty() {
            let mut c = 0.0;
            for &fi in &self.vfaces[i] {
                let f = self.o.indices[fi as usize];
                let ps: Vec<Vec3> = f
                    .iter()
                    .map(|&v| {
                        if v as usize == i {
                            cand
                        } else {
                            self.pos(lab, v as usize)
                        }
                    })
                    .collect();
                let nrm = (ps[1] - ps[0]).cross(ps[2] - ps[0]);
                if nrm.length_squared() < 1e-12 {
                    continue;
                }
                let sf = sorted_abs(nrm.normalize());
                for &v in &f {
                    if let Some(sn) = self.normal_key.get(v as usize) {
                        c += (0..3).map(|k| (sf[k] - sn[k]).powi(2)).sum::<f32>();
                    }
                }
            }
            score -= W_NORMAL * c / self.vfaces[i].len() as f32;
        }
        if self.w_fold > 0.0 {
            score -= self.w_fold * self.fold_penalty(lab, i, cand);
        }
        score
    }

    /// Mean over the faces at `i` of how far their folds against winding-consistent neighbours
    /// exceed 90° (flat or convex-ish surfaces keep the dot product of adjacent normals >= 0).
    fn fold_penalty(&self, lab: &[u8], i: usize, cand: Vec3) -> f32 {
        let at = |v: u32| {
            if v as usize == i {
                cand
            } else {
                self.pos(lab, v as usize)
            }
        };
        let normal = |fi: u32| {
            let f = self.o.indices[fi as usize];
            let (p0, p1, p2) = (at(f[0]), at(f[1]), at(f[2]));
            (p1 - p0).cross(p2 - p0).normalize_or_zero()
        };
        let (mut sum, mut cnt) = (0.0f32, 0u32);
        for &fi in &self.vfaces[i] {
            let f = self.o.indices[fi as usize];
            let n1 = normal(fi);
            if n1 == Vec3::ZERO {
                continue;
            }
            for k in 0..3 {
                let (a, b) = (f[k], f[(k + 1) % 3]);
                if a as usize != i && b as usize != i {
                    continue;
                }
                let Some(list) = self.mates.get(&(a.min(b), a.max(b))) else {
                    continue;
                };
                let forward = a < b;
                for &(gi, gfwd) in list {
                    if gi == fi || gfwd == forward {
                        continue; // same face, or winding not consistent across this edge
                    }
                    let n2 = normal(gi);
                    if n2 == Vec3::ZERO {
                        continue;
                    }
                    sum += (-n1.dot(n2)).max(0.0);
                    cnt += 1;
                }
            }
        }
        if cnt == 0 {
            0.0
        } else {
            sum / cnt as f32
        }
    }
}

/// Per-vertex margin of the labelling `labels`: the objective's score for the chosen label minus
/// the best alternative (negative: another label would fit better). Small margins mark vertices
/// the data does not pin down.
#[must_use]
pub fn label_margins(o: &O3d, labels: &[u8]) -> Vec<f32> {
    let sc = Scorer::new(o, DecodeParams::default().w_fold);
    let mut pool = Pool::new();
    for (i, p) in sc.stored.iter().enumerate() {
        pool.add(permuted(labels[i] as usize, *p), 1);
    }
    (0..o.vertices.len())
        .map(|i| {
            pool.add(sc.pos(labels, i), -1);
            let own = sc.score(&pool, labels, i, labels[i] as usize);
            let alt = (0..4)
                .filter(|&p| p != labels[i] as usize)
                .map(|p| sc.score(&pool, labels, i, p))
                .fold(f32::MIN, f32::max);
            pool.add(sc.pos(labels, i), 1);
            own - alt
        })
        .collect()
}

fn run_local_search(
    o: &O3d,
    seed: u32,
    random_init: bool,
    init: Option<Vec<u8>>,
    eval_only: bool,
    params: &DecodeParams,
    allowed: Option<&[u8]>,
) -> (Vec<u8>, f32) {
    let n = o.vertices.len();
    let ok = |i: usize, l: usize| allowed.is_none_or(|a| a[i] & (1 << l) != 0);
    let sc = Scorer::new(o, params.w_fold);
    let stored = &sc.stored;

    let mut lab = init.unwrap_or_else(|| vec![0u8; n]);
    if random_init {
        let mut r = seed | 1;
        for l in &mut lab {
            r ^= r << 13;
            r ^= r >> 17;
            r ^= r << 5;
            *l = (r % 4) as u8;
        }
        for (i, l) in lab.iter_mut().enumerate() {
            while !ok(i, *l as usize) {
                *l = (*l + 1) % 4;
            }
        }
    }
    let mut pool = Pool::new();
    for (i, p) in stored.iter().enumerate() {
        pool.add(permuted(lab[i] as usize, *p), 1);
    }
    let pos = |lab: &[u8], i: usize| sc.pos(lab, i);
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
                if (eval_only && p != lab[i] as usize) || !ok(i, p) {
                    continue;
                }
                let score = sc.score(&pool, &lab, i, p);
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
        None,
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

/// Two independent runs (belief-propagation seeded and random) must agree; returns the labels and
/// the agreement, or the best agreement reached when none sufficed. `allowed` restricts the label
/// set of each vertex (bit `l` set: label `l` possible).
fn confident_labels(
    o: &O3d,
    params: &DecodeParams,
    allowed: Option<&[u8]>,
) -> Result<(Vec<u8>, f32), f32> {
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
    let mut runs: Vec<(Vec<u8>, f32)> = Vec::new();
    let mut best_pair = 0.0f32;
    for r in 0..CONFIDENT_MAX_RUNS {
        runs.push(solve_once(
            o,
            0x9E37_79B9_u32.wrapping_add(r.wrapping_mul(0x85EB_CA6B)),
            r > 0,
            params,
            allowed,
        ));
        for i in 0..runs.len() - 1 {
            let ag = agreement(&runs[i].0, &runs[runs.len() - 1].0);
            best_pair = best_pair.max(ag);
            if ag >= params.min_agreement {
                let last = runs.len() - 1;
                let pick = if runs[i].1 >= runs[last].1 { i } else { last };
                return Ok((runs[pick].0.clone(), ag));
            }
        }
    }
    Err(best_pair)
}

/// Whether two meshes are scramblings of the same model: same face lists and the same sorted
/// coordinate triple at every vertex.
#[must_use]
pub fn same_model(a: &O3d, b: &O3d) -> bool {
    a.vertices.len() == b.vertices.len()
        && a.indices == b.indices
        && a.vertices.iter().zip(&b.vertices).all(|(p, q)| {
            let (mut x, mut y) = (
                [p.position.x, p.position.y, p.position.z],
                [q.position.x, q.position.y, q.position.z],
            );
            x.sort_by(f32::total_cmp);
            y.sort_by(f32::total_cmp);
            x == y
        })
}

/// Decodes several scramblings of the *same* model together (see [`same_model`]). The files carry
/// different keys, so their per-vertex permutations are independent while the true positions are
/// equal: the first copy is solved with every label no other copy contradicts, the other copies
/// take the labels that give the same positions. Texture jitter is resolved jointly too (a
/// vertex's true coordinates must reproduce every copy's stored value with an allowed offset).
///
/// Returns one report per copy (all rejected when the copies do not match or the runs disagree);
/// accepted copies are modified in place.
pub fn decode_joint(copies: &mut [O3d], params: &DecodeParams) -> Vec<ConfidentReport> {
    decode_joint_bounded(copies, params, None)
}

/// [`decode_joint`] with an expected box for the vertices.
pub fn decode_joint_bounded(
    copies: &mut [O3d],
    params: &DecodeParams,
    bounds: Option<ExpectedBounds>,
) -> Vec<ConfidentReport> {
    let rejected = |agreement: f32| ConfidentReport {
        accepted: false,
        agreement,
        coherence: 0.0,
        dropped_faces: 0,
        uv: super::uvfix::UvFixReport::default(),
    };
    let none = |n: usize| vec![rejected(0.0); n];
    if copies.len() < 2 {
        return none(copies.len());
    }
    let n = copies[0].vertices.len();
    if !(4..=CONFIDENT_MAX_VERTICES).contains(&n)
        || copies[0].indices.len() < 2
        || copies[1..].iter().any(|c| !same_model(&copies[0], c))
    {
        return none(copies.len());
    }
    const EPS: f32 = 1e-5;
    let close = |a: Vec3, b: Vec3| (a - b).abs().max_element() <= EPS;
    // Labels of copy 0 that every other copy can reproduce.
    let mut allowed = vec![0u8; n];
    for i in 0..n {
        let a = copies[0].vertices[i].position;
        for l in 0..4 {
            let t = permuted(l, a);
            if copies[1..].iter().all(|c| {
                let b = c.vertices[i].position;
                (0..4).any(|q| close(permuted(q, b), t))
            }) {
                allowed[i] |= 1 << l;
            }
        }
        if allowed[i] == 0 {
            return none(copies.len()); // not the same scrambled model after all
        }
    }
    if let Some(mask) = bounds.and_then(|b| bounds_mask(&copies[0], b)) {
        for (a, m) in allowed.iter_mut().zip(mask) {
            if *a & m != 0 {
                *a &= m;
            }
        }
    }
    let (labels, agree) = match confident_labels(&copies[0], params, Some(&allowed)) {
        Ok(v) => v,
        Err(best) => return vec![rejected(best); copies.len()],
    };
    let truth: Vec<Vec3> = (0..n)
        .map(|i| permuted(labels[i] as usize, copies[0].vertices[i].position))
        .collect();
    let uv_hints = super::uvfix::joint_uv(&copies.iter().collect::<Vec<&O3d>>());
    let mut out = Vec::with_capacity(copies.len());
    for (ci, c) in copies.iter_mut().enumerate() {
        for (v, t) in c.vertices.iter_mut().zip(&truth) {
            let stored = v.position;
            v.position = (0..4)
                .map(|q| permuted(q, stored))
                .find(|p| close(*p, *t))
                .unwrap_or(*t);
        }
        let dropped = drop_spike_faces(c);
        let uv = super::uvfix::unjitter_uv_with_hints(c, &uv_hints[ci]);
        c.descrambled = true;
        out.push(ConfidentReport {
            accepted: true,
            agreement: agree,
            coherence: 1.0,
            dropped_faces: dropped,
            uv,
        });
    }
    out
}

/// A decoded face whose longest edge is more than this many times the median edge (and longer than
/// [`SPIKE_MIN_EDGE`]) is a spike made by a wrongly labelled vertex.
pub const SPIKE_EDGE_FACTOR: f32 = 40.0;
/// Edges up to this length (metres) are never spikes.
pub const SPIKE_MIN_EDGE: f32 = 1.0;

/// Removes the faces of a decoded mesh that reach across the model (longest edge more than
/// [`SPIKE_EDGE_FACTOR`] times the median edge and longer than [`SPIKE_MIN_EDGE`]); returns how many
/// were dropped. Measured on 1,826 accepted plain/keyed twins (`mesh_detector_eval`): faces beyond
/// 40x the median edge are wrong (touch a mislabelled vertex) in 69 % of the cases, whereas the
/// 6x rule that was used before dropped 2 correct faces for every wrong one. A spike across the
/// whole model is far more conspicuous than a missing thin triangle, hence the trade.
pub fn drop_spike_faces(o: &mut O3d) -> usize {
    let edge = |o: &O3d, t: &[u32; 3]| {
        let p = t.map(|k| {
            o.vertices
                .get(k as usize)
                .map_or(Vec3::ZERO, |v| v.position)
        });
        (p[1] - p[0])
            .length()
            .max((p[2] - p[0]).length())
            .max((p[2] - p[1]).length())
    };
    if o.indices.len() < 8 {
        return 0;
    }
    let lens: Vec<f32> = o.indices.iter().map(|t| edge(o, t)).collect();
    let mut sorted = lens.clone();
    sorted.sort_by(f32::total_cmp);
    let limit = (sorted[sorted.len() / 2] * SPIKE_EDGE_FACTOR).max(SPIKE_MIN_EDGE);
    let keep: Vec<bool> = lens.iter().map(|&l| l.is_finite() && l <= limit).collect();
    let dropped = keep.iter().filter(|k| !**k).count();
    if dropped == 0 {
        return 0;
    }
    let mut it = keep.iter();
    o.indices.retain(|_| *it.next().unwrap_or(&true));
    if o.face_materials.len() == keep.len() {
        let mut it = keep.iter();
        o.face_materials.retain(|_| *it.next().unwrap_or(&true));
    }
    dropped
}

/// Moves vertices that the labelling put far outside the bulk of the mesh back inside, when one
/// of their four permutations fits: the robust box is the 2 %–98 % range of every axis widened by
/// 30 % of its span. Real stray vertices are rare, wrong labels at symmetric parts produce them.
fn refine_outliers(o: &O3d, labels: &mut [u8]) {
    let n = o.vertices.len();
    if n < 50 {
        return;
    }
    let pos = |i: usize, l: u8| permuted(l as usize, o.vertices[i].position);
    let mut axis: [Vec<f32>; 3] = Default::default();
    for i in 0..n {
        let p = pos(i, labels[i]);
        for k in 0..3 {
            axis[k].push(p[k]);
        }
    }
    let (mut lo, mut hi) = ([0.0f32; 3], [0.0f32; 3]);
    for k in 0..3 {
        axis[k].sort_by(f32::total_cmp);
        let (a, b) = (axis[k][n / 50], axis[k][n - 1 - n / 50]);
        let pad = 0.3 * (b - a) + 0.05;
        lo[k] = a - pad;
        hi[k] = b + pad;
    }
    let inside = |p: Vec3| (0..3).all(|k| p[k] >= lo[k] && p[k] <= hi[k]);
    for i in 0..n {
        if inside(pos(i, labels[i])) {
            continue;
        }
        if let Some(l) = (0..4u8).find(|&l| inside(pos(i, l))) {
            labels[i] = l;
        }
    }
}

/// Axis-aligned box every decoded vertex is expected to lie in (mesh frame).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ExpectedBounds {
    /// Smallest corner.
    pub min: Vec3,
    /// Largest corner.
    pub max: Vec3,
}

impl ExpectedBounds {
    /// Box of a vehicle given in OMSI's frame (x lateral, y longitudinal, z up) as *full* extents
    /// and centre offset, expressed in the mesh frame (x lateral, y up, z longitudinal) and widened
    /// by `0.5 m + 10 %` of the longest side. (The `.bus` box holds full extents although the
    /// format description calls them half extents: a 2.55 m wide bus has `2.55`; it covers the
    /// body without roof equipment and undercarriage, hence the margin.)
    #[must_use]
    pub fn from_omsi_box(size: Vec3, centre: Vec3) -> Self {
        let pad = 0.5 + 0.1 * size.max_element();
        let h = Vec3::new(size.x, size.z, size.y) * 0.5;
        let c = Vec3::new(centre.x, centre.z, centre.y);
        Self {
            min: c - h - Vec3::splat(pad),
            max: c + h + Vec3::splat(pad),
        }
    }

    /// Stable tag of the box for cache keys (centimetre resolution).
    #[must_use]
    pub fn tag(&self) -> u64 {
        let mut h = 0xcbf2_9ce4_8422_2325_u64;
        for v in [self.min, self.max] {
            for c in [v.x, v.y, v.z] {
                h ^= (c * 100.0).round() as i64 as u64;
                h = h.wrapping_mul(0x0100_0000_01b3);
            }
        }
        h
    }
}

/// Label masks that keep only the permutations placing each vertex inside `bounds`; a vertex no
/// label fits keeps all four. `None` when more than 5 % of the vertices fit no label (the box
/// does not describe this mesh).
fn bounds_mask(o: &O3d, bounds: ExpectedBounds) -> Option<Vec<u8>> {
    let mut misfits = 0usize;
    let mask: Vec<u8> = o
        .vertices
        .iter()
        .map(|v| {
            let mut m = 0u8;
            for l in 0..4 {
                let p = permuted(l, v.position);
                if p.cmpge(bounds.min).all() && p.cmple(bounds.max).all() {
                    m |= 1 << l;
                }
            }
            if m == 0 {
                misfits += 1;
                0b1111
            } else {
                m
            }
        })
        .collect();
    (misfits * 20 <= o.vertices.len()).then_some(mask)
}

/// [`decode_confident_with`] restricted to labels that keep every vertex inside `bounds`
/// (for instance the box of the vehicle the mesh belongs to).
pub fn decode_confident_bounded(
    o: &mut O3d,
    params: &DecodeParams,
    bounds: ExpectedBounds,
) -> ConfidentReport {
    decode_confident_masked(o, params, bounds_mask(o, bounds))
}

/// Decodes a scrambled mesh without a plain twin when the answer is trustworthy.
///
/// Two independent annealing runs (one seeded by belief propagation, one from random labels) must
/// agree on [`CONFIDENT_MIN_AGREEMENT`] of the vertices (on plain/keyed twins, runs that agree
/// to 98 % are exact to 98 % in 86 % of the cases). Accepted meshes keep all triangles and get
/// their texture jitter removed where a planar UV model proves it ([`super::uvfix::unjitter_uv`]).
pub fn decode_confident(o: &mut O3d) -> ConfidentReport {
    decode_confident_with(o, &DecodeParams::default())
}

/// [`decode_confident`] with explicit parameters.
pub fn decode_confident_with(o: &mut O3d, params: &DecodeParams) -> ConfidentReport {
    decode_confident_masked(o, params, None)
}

fn decode_confident_masked(
    o: &mut O3d,
    params: &DecodeParams,
    allowed: Option<Vec<u8>>,
) -> ConfidentReport {
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
    let (labels, agree) = match confident_labels(o, params, allowed.as_deref()).map(|(mut l, a)| {
        refine_outliers(o, &mut l);
        (l, a)
    }) {
        Ok(v) => v,
        Err(best_pair) => return rejected(best_pair, 0.0),
    };
    let mut trial = o.clone();
    for (v, &l) in trial.vertices.iter_mut().zip(&labels) {
        v.position = permuted(l as usize, v.position);
    }
    let (verdicts, coherence) = triangle_verdicts(&trial);
    if coherence < params.min_coherence {
        return rejected(agree, coherence);
    }
    // A long-edge filter at 6x the median edge dropped 91 % correct faces on plain/keyed twins
    // (big flat panels), so only extreme spikes go: see `drop_spike_faces`.
    let _ = verdicts;
    let dropped = drop_spike_faces(&mut trial);
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
                min_agreement: 0.7,
                ..base.clone()
            },
            DecodeParams {
                min_coherence: 0.5,
                ..base.clone()
            },
            DecodeParams {
                w_fold: 1.0,
                ..base.clone()
            },
        ];
        for v in &variants {
            assert_ne!(v.fingerprint(), base.fingerprint());
        }
        assert_eq!(base.fingerprint(), DecodeParams::default().fingerprint());
    }
}

#[cfg(test)]
mod joint_tests {
    use super::*;
    use crate::formats::o3d::Vertex;
    use glam::{Vec2, Vec3};

    /// An asymmetric box surface, 3x3 quads per face, flat-shaded (vertices not shared across faces).
    fn box_mesh() -> O3d {
        let dims = Vec3::new(1.0, 2.3, 3.7);
        let mut vertices = Vec::new();
        let mut indices = Vec::new();
        for axis in 0..3usize {
            for side in [0.0f32, 1.0] {
                let (a, b) = ((axis + 1) % 3, (axis + 2) % 3);
                let base = vertices.len() as u32;
                for j in 0..4 {
                    for i in 0..4 {
                        let mut p = [0.0f32; 3];
                        p[axis] = side * dims[axis];
                        p[a] = i as f32 / 3.0 * dims[a];
                        p[b] = j as f32 / 3.0 * dims[b];
                        let mut n = [0.0f32; 3];
                        n[axis] = if side == 0.0 { -1.0 } else { 1.0 };
                        vertices.push(Vertex {
                            position: Vec3::from(p),
                            normal: Vec3::from(n),
                            uv: Vec2::new(i as f32 / 3.0, j as f32 / 3.0),
                            color: None,
                        });
                    }
                }
                for j in 0..3u32 {
                    for i in 0..3u32 {
                        let q = base + j * 4 + i;
                        indices.push([q, q + 1, q + 4]);
                        indices.push([q + 1, q + 5, q + 4]);
                    }
                }
            }
        }
        let faces = indices.len();
        O3d {
            version: 7,
            header_flags: 0,
            vertices,
            indices,
            face_materials: vec![0; faces],
            materials: Vec::new(),
            transform: None,
            encryption_key: Some(7),
            descrambled: false,
            unknown_chunks: Vec::new(),
        }
    }

    /// Applies a pseudo-random permutation class to every vertex.
    fn scramble(truth: &O3d, mut seed: u32, key: u32) -> O3d {
        let mut o = truth.clone();
        o.encryption_key = Some(key);
        for v in &mut o.vertices {
            seed = seed.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
            let l = (seed >> 24) as usize % 4;
            // `permuted` is its own inverse for these four (transpositions and identity).
            v.position = permuted(l, v.position);
        }
        o
    }

    #[test]
    fn two_scramblings_decode_to_the_true_positions() {
        let truth = box_mesh();
        let mut copies = vec![scramble(&truth, 11, 100), scramble(&truth, 99, 200)];
        assert!(same_model(&copies[0], &copies[1]));
        let reports = decode_joint(&mut copies, &DecodeParams::default());
        assert!(reports.iter().all(|r| r.accepted), "{reports:?}");
        for c in &copies {
            for (v, t) in c.vertices.iter().zip(&truth.vertices) {
                assert!((v.position - t.position).length() < 1e-5);
            }
        }
    }

    #[test]
    fn unrelated_meshes_are_rejected() {
        let a = box_mesh();
        let mut b = box_mesh();
        b.vertices[3].position += Vec3::splat(0.5);
        let mut copies = vec![a, b];
        let reports = decode_joint(&mut copies, &DecodeParams::default());
        assert!(reports.iter().all(|r| !r.accepted));
    }
}

#[cfg(test)]
mod bounds_tests {
    use super::*;

    #[test]
    fn omsi_box_becomes_a_padded_mesh_frame_box() {
        // 2.5 wide, 10 long, 2.5 high, centred 1 m ahead and 1.5 m up (OMSI: x, y long, z up).
        let b = ExpectedBounds::from_omsi_box(Vec3::new(2.5, 10.0, 2.5), Vec3::new(0.0, 1.0, 1.5));
        let pad = 0.5 + 0.1 * 10.0;
        // Mesh frame: x lateral, y up, z longitudinal.
        assert!((b.min.x + 1.25 + pad).abs() < 1e-5 && (b.max.x - 1.25 - pad).abs() < 1e-5);
        assert!((b.min.y - (0.25 - pad)).abs() < 1e-5 && (b.max.y - (2.75 + pad)).abs() < 1e-5);
        assert!((b.min.z - (-4.0 - pad)).abs() < 1e-5 && (b.max.z - (6.0 + pad)).abs() < 1e-5);
    }

    #[test]
    fn the_tag_depends_on_the_box() {
        let a = ExpectedBounds {
            min: Vec3::ZERO,
            max: Vec3::ONE,
        };
        let b = ExpectedBounds {
            min: Vec3::ZERO,
            max: Vec3::splat(1.5),
        };
        assert_ne!(a.tag(), b.tag());
        assert_eq!(a.tag(), a.tag());
    }
}

#[cfg(test)]
mod spike_tests {
    use super::*;
    use crate::formats::o3d::Vertex;
    use glam::{Vec2, Vec3};

    fn grid(spike: bool) -> O3d {
        let mut vertices = Vec::new();
        let mut indices = Vec::new();
        for j in 0..6u32 {
            for i in 0..6u32 {
                vertices.push(Vertex {
                    position: Vec3::new(i as f32 * 0.5, 0.0, j as f32 * 0.5),
                    normal: Vec3::Y,
                    uv: Vec2::ZERO,
                    color: None,
                });
            }
        }
        for j in 0..5u32 {
            for i in 0..5u32 {
                let q = j * 6 + i;
                indices.push([q, q + 1, q + 6]);
                indices.push([q + 1, q + 7, q + 6]);
            }
        }
        if spike {
            // one wrongly labelled vertex flung 80 m away
            vertices[14].position = Vec3::new(80.0, 40.0, -60.0);
        }
        let n = indices.len();
        O3d {
            version: 7,
            header_flags: 0,
            vertices,
            indices,
            face_materials: (0..n as u16).collect(),
            materials: Vec::new(),
            transform: None,
            encryption_key: Some(1),
            descrambled: false,
            unknown_chunks: Vec::new(),
        }
    }

    #[test]
    fn spike_faces_of_a_flung_vertex_are_dropped_with_their_materials() {
        let mut o = grid(true);
        let before = o.indices.len();
        let dropped = drop_spike_faces(&mut o);
        assert!(dropped >= 3, "faces around the flung vertex go: {dropped}");
        assert_eq!(o.indices.len(), before - dropped);
        assert_eq!(o.face_materials.len(), o.indices.len());
        assert!(o.indices.iter().all(|t| !t.contains(&14)));
    }

    #[test]
    fn a_clean_mesh_loses_nothing() {
        let mut o = grid(false);
        assert_eq!(drop_spike_faces(&mut o), 0);
        assert_eq!(o.indices.len(), 50);
    }
}
