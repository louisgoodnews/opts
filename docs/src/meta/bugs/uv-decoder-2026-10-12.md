# O3D UV decoder correctness (P-1013)

Branch `fix/uv-decoder-correctness` (from `fix/man-nlng-materials`, which carries the P-1012 diagnostic).
Tools: `crates/opts-core/examples/uv_truth.rs` (twin enumeration and per-vertex truth), `scripts/uv_truth_analysis.py`.
Run: `uv_truth "<OMSI 2>" <out>` then `python3 scripts/uv_truth_analysis.py <out>`.
Status: **Phase A (diagnostic) complete.**

## Preconditions
1. Branch `fix/uv-decoder-correctness`: yes. 2. `man-nlng-materials-2026-10-12.md`: yes (7d13e12).
3. `uvfix` exists at `crates/opts-core/src/formats/o3d/uvfix.rs` (not `binary/o3d/` as the task says).
4. **The premise "17 twin pairs in the GN92 folder" does not hold.** The `plain twins 17` in the game log counts meshes of
   the loaded vehicle whose plain file name exists elsewhere. Only 13 GN92 file names occur as plain files in
   other vehicles (`GN_Dum_Gelenk_A-D`, `Rad_HL/HR/VL/VR`, `schildFrnt*`, `seitenschild`, `seitenschklemme`) and only
   5 of those are the same model; **none of the artifact meshes (steering wheel, panel, interior) has a twin**. There is
   no pair inside `MAN_NL_NG`. The ground truth is the installation-wide set below (2,476 pairs, 881,573 vertices).
5-7. The NG272 artifacts reproduce, SD77/NLC are the working references, the overlay works (P-1012/P-1011 runs).

## A1 - twin inventory (whole installation, 172,434 `.o3d` files)
Pairs = same file name (case-insensitive), vertex and triangle counts, one plain and one keyed file (first by sorted
path when several exist): **2,595 candidate pairs, 2,476 with identical geometry (`same_model`)**; the other 119 share
name and counts only. Of the 2,476: material slots/texture names/face slots equal in 2,158 (318 differ), version equal
in 2,320, header flags equal in 1,790, chunk structure (tags of unknown chunks, matrix chunk present) equal in all 2,476.
The full table (paths, sizes in bytes, counts) is `pairs.tsv` of `uv_truth` (not committed, 554 kB; regenerate).

## A2 - per-vertex comparison (tolerance 1e-4 in u and v)
| | vertices | exact |
|---|---|---|
| UV step alone (`unjitter_uv` with the true positions) | 881,573 | **77.07 %** |
| end to end (`decode_confident`, 2,439 accepted meshes) | 853,855 | **76.82 %** |

Wrong vertices (step alone): 202,184. Error |decoded - plain|: min 0.0001, **median 0.250**, p95 0.846, p99 1.032, max 1.551
(the error of a vertex is the scrambler's jitter or a wrong correction of it, both multiples of the jitter set).
Per mesh: median 83.9 % exact, p25 60.0 %, p5 22.2 %; only 310 of 2,476 meshes are 100 % exact.
The decoder reports 0 % unresolved on these meshes, so it cannot see these errors.

## A3 - error patterns (ranked by explanatory power)
Truth: 50.1 % of all vertices carry jitter (441,964). Of those the decoder **leaves 38.2 % jittered** and **fixes 3.9 %
with a wrong offset**; of the 439,609 clean vertices it **wrongly shifts 3.7 %**.
1. **Index/count-based: meshes with `vertex count mod 5 = 0`.** Their jittered vertices are corrected right only 26.2 %
   of the time (118,694 vertices) against 69.5 % for the others; the whole class is 41 % exact against 84-85 % (n mod 5 =
   1..4). This single class holds 43 % of all errors. The documented generator property (78 % of vertices u-jittered when
   5 | n instead of 20 %) leaves too few clean vertices for the planar fits.
2. **Jitter kind:** u-only 48.0 % exact, both 51.9 %, v-only 69.1 %, none 96.3 %.
3. **Format version:** v4 74.0 %, v5 77.9 %, v7 82.0 %; flags 0 (primary seed) 75.3 %, flags 2 (alternative seed) 81.4 %.
4. **Material slot:** slot 0 78.9 %, slots 2-6 71-74 % (mild).
5. **Mesh size:** < 50 vertices 76.6 %, >= 5000 vertices 73.9 % (mild, no trend).
6. Position-permutation class (0 / 1 / 2 / 3): 77.0 / 77.1 / 77.3 / 76.9 %: **no effect**. First/last 8 vertices, `i mod 2`,
   `i mod 5`: no effect (75-77 %). Plain UV inside or outside the unit square: 80.4 / 76.8 %. Share of distinct UVs of the
   mesh (constant-UV meshes): < 5 % distinct 73.0 %, otherwise 76-80 %: weak. Jitter magnitude: 53-64 %, flat.
7. **Chunk-based:** the chunk structure is identical in all pairs: no information. **Bit-based** (raw float patterns) and
   **normal-based** were not measured: the scrambled coordinate is the plain value plus the offset in float arithmetic, so
   the low bits carry no extra signal; the scrambled normals are not recovered at all (open).

## A4 - the `uvfix` pass (`unjitter_uv_with_hints`)
Stages: (1) plane-affine RANSAC over coplanar vertex patches; a vertex is accepted as jittered only when its residual
is a member of the allowed set (`jitter_pairs`, 79 offsets: u `(5k/100)^2`, v `(2m/100)^2`, joint with `m_v = 2 m_u mod 100`);
(2) value-pool pass (leftovers made equal to values that occur >= 3 times); (3) hinge pass (unfold across a shared edge,
accept when exactly one offset matches within 3e-3); (4) smooth pass (Gibbs sampling over offset indices with the
measured priors `PRIOR_NOT5`/`PRIOR_FIVE`). Failing assumptions (evidence A3): the planar-affine model needs
`MIN_CLEAN = 3` clean vertices and `MIN_CLEAN_PERCENT = 40` per patch, which fails when most vertices are jittered
(5 | n); non-planar or curved UVs (steering wheel, needles, gauges) have no affine patches; a vertex whose offset
is one of several consistent ones is silently accepted (`unresolved` counts only vertices that matched no model).
Verifiable from the twins: every rule above can be scored per vertex with `uv_truth`.

## A5 - answer
- Pairs: 2,476 verified (881,573 vertices), none in the NL/NG family.
- Exact: 77.07 % (UV step), 76.82 % end to end; wrong 202,184 vertices, median error 0.25, p99 1.03.
- Dominant pattern: 5 | n meshes (43 % of the errors); then u-jitter vertices.
- Failure point: planar-patch model needs clean vertices that these meshes do not have; failures are silent.
- **Feasibility:** the scrambler's chain `s` (data-dependent state) is still unknown (Addendum 5). Without it no decoder can
  be *provably* exact on a mesh that has no twin; what is possible is a better inference with a quality flag per vertex
  and twin/joint-copy preference. See the checkpoint proposal.

## Phase B1 - inference with the 881k truth set
All numbers: 2,476 pairs, 881,573 vertices, UV step with the true positions (`uv_step_eval`), tolerance 1e-4.

| Change | Exact |
|---|---|
| baseline (Phase A) | 77.07 % |
| smooth prior 0.4 -> 0.3 (measured optimum; 0.1: 71.96, 0.6: 76.35) | 78.42 % |
| + welded hinges (corners that share a position join the hinge graph; 13 % of vertices still have no hinge) | 78.05 % alone, 78.42 % with the prior |
| + `min_clean` 3 -> 6 (plane patches need 6 jitter-free anchors) | 78.97 % |
| + 4 annealing restarts, lowest energy wins | **79.55 %** (non-5\|n meshes 87.7 %, 5\|n 41.4 %) |
| vertex-order chain as post-correction (forward-backward, trained on even pairs, held-out odd) | +0.35 points only (75.57 -> 75.92 on the odd pairs) |
| vertex-order chain as an energy term inside the smooth stage | **no gain** (79.51 -> 79.57 at best, worse at weights >= 0.3) |
| smooth stage alone (no plane/pool/hinge stages), 80 / 300 / 1000 sweeps | 64.6 / 66.4 / 67.2 % |

Target 85 %: **not reached; 79.55 % is where this evidence stalls.**

**Stage accuracy** (new defaults): plane 24.4 % of vertices at 94.4 % exact; pool 13.4 % at 92.7 %; hinge 0.6 % at 88 %; smooth 61.7 % at 70.8 %.
The smooth stage is the weak link and it settles most vertices.

**Ceiling of the evidence (the "top-K" question).** With every other vertex at its true offset, the best of the 79 candidates
under the smooth model (hinge + prior) is the true one for **87.2 %** of the vertices (5|n meshes 86.5 %); the true offset is
within the best 3 for 92.6 %, 5: 93.5 %, 10: 95.3 %. 13 % of the vertices touch no hinge at all and can only use the prior.
The search is what falls short of that ceiling: on 1,007 of 2,468 meshes the *true* assignment scores a lower energy than the decoded one
(search failure), on 1,135 the decoded one is lower (model failure), 326 equal.

**Offset prior (measured).** 79 offsets occur and nothing else (`(u, v) = ((k/20)^2, (m/50)^2)`); 50.1 % of the vertices are jittered.
Most frequent (share of all jittered vertices): (k=10) 5.0 %, 12: 4.2 %, 8: 4.2 %, 16: 4.2 %, 4: 4.1 %, then (k,m) = (4,20) 2.1 %, (16,30) 2.1 %, (12,10) 2.1 %, (8,40) 2.0 %,
18/14/6/2 (u only) 1.8-1.7 %, (0,36), (0,46), (0,12), (0,22), (0,26), (0,18), (0,42) (v only) 1.7 % each: u offsets with k divisible by 4 and 10 are
about 2.5x as frequent as odd k.
**Neighbour structure.** Consecutive vertices share the same offset pair 30.0 % of the time (u-only: k equal 22.7 %). P(next has u-jitter) is 53 % after a vertex
with u-jitter and 19 % after one without. The difference `(k_i - k_(i-1)) mod 20` is 19.6 % at 0, 8.5 % at each of 4/8/12/16, 6.4 % at 10, about 2 % at odd values:
the offsets of neighbours agree mostly modulo 4. Using this as a prior did not help (above).

**Negative result - a single hinge cannot be solved exactly** (`uv_hinge_solve`, 204 hinges that are affine across the shared edge, 46 % of all): the true offsets of its four vertices are always among the solutions of
`uv(x) = w . [uv(b), uv(c), uv(d)]` but only 2 hinges have a unique solution, 186 have more than 20 (a common shift of all four offsets is always a solution,
plus coincidences). Exactness needs region-level joint solving, not local solving.

## Phase B2 - the 5 | n lead
- **No group-of-5 structure.** Exactness by `vertex index mod 5` inside 5|n meshes is flat (41.4 % for each residue); errors do not cluster at positions 0, 5, 10... and no chunk-level difference exists (chunk tags are identical in all pairs).
- **It is arithmetic, and a bootstrap problem.** The scrambler's state is `q = n * s (mod 8000)` (Addendum 5). With `5 | n` every `q` is divisible by 5, so the u-jitter `(q mod 100 / 100)^2`
  is visible on every vertex: u-jitter on 76.7 % of the vertices against 17.8 % otherwise (v-jitter 22.3 % / 32.3 %, independent of `n mod 3`). For these meshes `k = (n/5 * s) mod 20`: P(k = 0) is 1.0 for
  `n/5 = 0 mod 20`, 0.56 for `n/5 = 10 mod 20`, 0.25 for 5, 0.05 for `n/5` coprime to 20 - the observed values match the arithmetic, with `s` close to uniform.
- **Why the decoder fails there.** With 77 % of the vertices jittered there are almost no jitter-free anchors, so the plane stage (which needs them) finds nothing and the smooth stage has to bootstrap from nothing:
  41 % exact. The *information* is there: the single-vertex ceiling of 5|n meshes is 86.5 %, the same as for other meshes. This is a search/anchor problem, not an information problem.
- **Conclusion:** structural finding is the arithmetic above; the group-of-5 hypothesis is a negative result. A decoder for these meshes needs region-level joint solving over the lattice of allowed offsets
  (all offsets are non-negative and drawn from a sparse set, so the common-shift ambiguity disappears over a few vertices). Not implemented.

## Calibration of the evidence (basis for `UvQuality`)
Precision of a vertex given its stage and evidence strength, measured on the 881,573 truth vertices (previous defaults, `min_clean` 3):

| Evidence | Share of all vertices | Precision |
|---|---|---|
| plane, >= 6 anchors | 22.3 % | 95.0 % |
| plane, >= 10 | 11.9 % | 97.4 % |
| plane, >= 20 | 5.5 % | 98.5 % |
| plane, >= 40 | 2.4 % | 99.7 % |
| pool | 10.0 % | 92.9 % |
| smooth, margin >= 2 | 4.8 % | 93.0 % |
| smooth, margin >= 4 | 2.9 % | 98.2 % |
| smooth, margin >= 8 | 1.1 % | 98.9 % |

So at >= 99 % precision only about 2-5 % of the vertices of a mesh without a twin can be certified; at >= 90 % about a third.
For the MAN NL/NG folder (325 meshes, all keyed) only 5 have a plain copy elsewhere and 31 more have keyed copies elsewhere
(`uv_copies`): about 89 % of its meshes depend on inference alone.
