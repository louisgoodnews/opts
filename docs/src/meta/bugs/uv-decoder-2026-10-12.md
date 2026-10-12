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
