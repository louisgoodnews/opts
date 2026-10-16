# UV decoder (P-1013) — retired

> **Retired 2026-10-16.** The scrambler's generator was derived ([o3d.md Addendum 14](../formats/binary/o3d.md#-addendum-14--2026-10-16-the-scramblers-generator-exact-decoding)); keyed meshes are decoded exactly, including their texture coordinates, and the heuristic described here was deleted ([Addendum 15](../formats/binary/o3d.md#-addendum-15--2026-10-16-the-heuristic-decoder-retired)). This page is kept as the record of the design; the code it names (`uvfix.rs`, `descramble.rs`, `descramble_cache.rs`, `uv_incident.rs`) is in the git history before that date.


Scope: the texture coordinates of keyed (scrambled) `.o3d` meshes. Positions are decoded by
`descramble` (separate document: `formats/binary/o3d.md`, Addendum 5); this decoder works on the
stored `(u, v)` and the positions. Code: `crates/opts-core/src/formats/o3d/uvfix.rs` (the task text names
`formats/binary/o3d/`; the real path is `formats/o3d/`).

## 1. What the scrambler does (measured on 2,476 plain/keyed pairs, 881,573 vertices)
- Each vertex gets an additive offset `(du, dv) = ((k/20)^2, (m/50)^2)` out of **79** values (index 0: none); 50.1 % of the vertices carry one. Nothing else is added.
- The state behind it is `q = n * s (mod 8000)` (`n` = vertex count, `s` an unknown data-dependent chain). `q mod 100` shows as the u offset when `5 | q`, `2q mod 100` as the v offset when `3 | q`.
  With `5 | n` every `q` is a multiple of 5, so 76.7 % of the vertices of such meshes carry a u offset (17.8 % otherwise): measured P(k = 0) by `(n/5) mod 20` matches the arithmetic.
- Offsets are not uniform (k divisible by 4 or 10 about 2.5 times as likely) and neighbouring vertices agree mostly modulo 4 (consecutive vertices share the pair 30 % of the time). There is no group-of-5 structure by vertex index.
- The chain `s` itself is **not reproduced**; nothing here uses it. Offsets are always `>= 0` and come from a sparse set; that sparsity is what makes joint solving possible.

## 2. Pipeline (stages in order; each vertex keeps the stage that settled it)
1. **Hint** - known from jointly decoded copies of the same model (`descramble::decode_joint`).
2. **Lattice (meshes with `5 | n`)** - plane seeding without clean anchors. A seed triangle's three offsets are *searched* (39 allowed offsets for these meshes, 59,319 combinations); the affine texture map they define predicts every other vertex of the plane; the right combination is the one for which many vertices' residuals all land on allowed offsets (accidental probability about 1e-6 per vertex). Seeds next to settled vertices are cheap (one or two unknowns) and tried exhaustively; seeds without settled corners are sampled (24 per round). Common shifts of all offsets satisfy the equations but move most residuals off the sparse set; ties are broken by the smaller prior energy.
3. **Plane** - classical RANSAC over coplanar patches, needs `min_clean = 6` jitter-free anchors (a vertex joins when its residual is zero or an allowed offset).
4. **Pool** - values that repeat elsewhere in the mesh (box mappings).
5. **Lattice, cheap seeds only (other meshes)** - the same search for faces next to settled vertices.
6. **Hinge** - a vertex is predicted across a shared edge from settled neighbours; accepted when exactly one offset matches.
7. **Smooth** - Gibbs sampling (4 restarts, lowest energy wins) over the offsets of all remaining vertices; energy = robust hinge residuals over welded hinges (corners at equal positions are joined, `weld`) + the measured offset prior (weight 0.3, `PRIOR_NOT5`/`PRIOR_FIVE`).
8. **Second look** - the smooth stage's guesses become unknowns again and the cheap lattice seeds run once more; guesses that cannot be verified are kept.

Parameters (`UvFixParams`, overridable by `UVFIX_*` variables in tools only): `min_clean` 6, `smooth_prior` 0.3, `restarts` 4, `lattice_min` 4, `lattice_trials` 24, `weld`.
Every constant was chosen on the truth set (`uv_step_eval`), none by hand.

## 3. Measured result (UV step with true positions / end to end with decoded positions)
| | exact |
|---|---|
| before (Phase A) | 77.07 % / 76.82 % |
| now | **85.14 %** / **84.82 %** (accepted meshes) |
| meshes with `5 \| n` | 41.4 % -> 69.5 % |
| other meshes | 84.7 % -> 88.5 % |

Precision of the stages (share of all vertices): plane 22.3 % at 96.7 %, lattice 18.7 % at 93.3 %, pool 13.3 % at 93.1 %, hinge 0.7 % at 87 %, smooth 45.0 % at 73.6 %.
The evidence ceiling (every other vertex at truth, hinge + prior model) is 87.2 % for single-vertex decisions; 13 % of the vertices touch no hinge.

## 4. `UvQuality` (per vertex; no vertex leaves the decoder without one)
`UvQuality::Exact`: the value is certain to within the calibration (>= 99 % precision on the truth set), or it comes from a plain twin. `Recovered`: inferred with 90-99 % precision.
`Unreliable`: less than 90 %; the value is the best guess (see the fallback), and the mesh counts toward the fleet report.

Assignment (calibrated on the truth set, evidence strength -> precision, share of all vertices):
| Stage and evidence | Precision | Share | Quality |
|---|---|---|---|
| plain twin, or the mesh is not keyed | 100 % | - | Exact |
| plane, >= 10 anchors | 99.0 % | 11.1 % | Exact |
| lattice, >= 12 verified vertices | 99.1 % | 3.4 % | Exact |
| smooth, margin >= 6 | 99.2 % | 2.4 % | Exact |
| plane, 6-9 anchors | about 95 % | 11.3 % | Recovered |
| lattice, 3-11 verified | 93-99 % | 15.4 % | Recovered |
| pool | 93.1 % | 13.3 % | Recovered |
| smooth, margin 2-6 | 95-99 % | 3.3 % | Recovered |
| hinge, smooth with margin < 2, anything unverified | below 90 % (smooth: about 70 %) | about 40 % | Unreliable |

So on a mesh without a twin about 17 % of the vertices can be certified, about 43 % are recovered with 90-99 %, and about 40 % are unreliable. The tiers are looked up from the stage and the confidence value that `UvFixOutcome` carries;
the table is code (`uv_quality_of`), regenerated from `uv_step_eval` whenever the pipeline changes.

## 5. No silent corrections
- A vertex whose evidence admits several offsets is never settled by a stage that cannot rank them: the pool and hinge stages require a unique match; the lattice stage takes the hypothesis with the most verified vertices and, on a tie, the smaller prior - ties on both are reported as ambiguous and the vertices stay unsettled for the smooth stage; the smooth stage reports the energy margin to the runner-up.
- Settled-by-smooth vertices are never labelled better than their margin allows; an unresolved vertex is `Unreliable`.
- The decoder's old counter `unresolved` stays 0 for completed runs; it is no longer a quality statement. `UvQuality` is.

## 6. Twin and copy preference (cost)
1. **Plain twin** (same file name, vertex and triangle counts, `same_model` geometry) - the plain file's texture coordinates replace the decoded ones and every vertex is `Exact`. Lookup: `OmsiInstall::files_named` (catalogue backed), then `same_model` on the candidate; one parse of the twin per mesh, cached with the descramble cache.
2. **Joint copies** (keyed copies of the same model with other keys) - `decode_joint`; hints are applied first, each copy's `Hint` stage vertices are `Exact` only when two copies agree with margin (97.5 % measured: `Recovered` otherwise).
3. **Inference** - the pipeline above.
Coverage in the installation: 2,476 meshes of the truth set have a twin; for `MAN_NL_NG` 5 of 325 meshes (all keyed) have a plain copy elsewhere and 31 more only keyed copies.

## 6a. Incident files
When more than 10 % of a scrambled mesh's vertices are `Unreliable` the loader writes, once per mesh and session, a text file `<log directory>/incidents/uv-<unix time>-<mesh>-<hash>.log` (the log directory is the one of `--log-file` or the default; the `WARN` line carries the path, at most 500 files are kept, the oldest go first). It records:
- the mesh (path, model index, file size, header version/flags/key, vertex and triangle counts, `vertex count mod 5`), the context that was loading it (the vehicle's `.bus` file) and where the decoding came from (decoded now, decode cache or background decode), the decoder parameter fingerprint, and how many other files with the same name exist (the chances of a twin or copy);
- the counts per quality and per stage (how many vertices each stage settled and how many of them are unreliable);
- the reasons, grouped (for example *several offsets explained the neighbours about equally well, energy margin below 2*), with notes for the known hard cases (5 | n, no twin or copy);
- the first 400 unreliable vertices: index, decoded position, stored (scrambled) texture coordinate, best-guess coordinate, the offset removed, the stage and its evidence strength.
The decode cache (version 8) stores the stage and evidence per vertex so that cached meshes produce the same file. Code: `crates/opts-bevy/src/uv_incident.rs`.

## 7. Fallback for `Unreliable` vertices
The value stays the decoder's best guess (smooth stage maximum). A hinge-interpolated value (prediction from `Exact`/`Recovered` neighbours without snapping to the offset set) will be evaluated against the truth set in Phase C and used only if it beats the guess;
evaluated 2026-10-13 on the twin truth set: worse than the best guess (51 % against 81 % within 0.01 on the 21 % of unreliable vertices where a prediction exists), so it is not used. One `WARN` and one incident file per mesh with the unreliable share; no per-vertex special handling.

## 8. Verification
- `crates/opts-core/tests/uv_decoder.rs` (Phase C): for every twin pair available at run time, decode the keyed mesh with true positions and assert the quality contract: no `Exact` vertex is wrong beyond the calibrated rate (at most 1 % wrong among `Exact`, 10 % among `Recovered`), every vertex has a quality, no `Unreliable` on plain twins; and a regression case from the pre-fix code (the GN92 steering wheel is a keyed mesh without twin: its share of `Unreliable` is asserted > 0, i.e. the flag exists where the old code reported 0 %).
- `opts-cli validate-uvs` (Phase E): the fleet table (vehicle, mesh, vertices, exact %, unreliable %), sorted by unreliable share.
