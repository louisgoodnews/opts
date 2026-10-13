# P-1014 — Scrambled Mesh Audit and Fix

Branch `agent/mesh-audit` (worktree `opts-wt-mesh-audit`, based on `agent/integration`). Progress is tracked in [`docs/dev/progress/mesh-audit.md`](../progress/mesh-audit.md); every irregularity found is catalogued in the [Mesh Issue Registry](../../src/quality/mesh-issue-registry.md).

## Goal

Find every mesh that renders scrambled (exploded vertices, shards, stretched or chaotic texturing, wrong winding, missing parts) across the installed corpus (maps: tiles, scenery objects, splines and road meshes; vehicles: bodies, interiors, wheels, bellows, animated parts) and classify each one as

* an **OPTS bug** (parser/loader/decoder) — fixed at the root with a regression test;
* a **content defect** (the source asset itself is broken) — registered, with a minimal and deterministic tolerance where one is sensible (for example skip degenerate triangles), otherwise left registered;
* **scrambled by the exporter** (keyed file) — covered by the decoder ([o3d.md Addendum 5](../../src/formats/binary/o3d.md)); the audit measures how many remain skipped or doubtful.

## Method

1. **Offline statistical audit** over the whole installed corpus (`crates/opts-core/examples/mesh_audit.rs`, seconds for 172k files without decoding, minutes with `--decode`): parse failures, non-finite values, index and material-index range, degenerate and huge triangles, exploded vertices and bounding-box outliers, winding disagreement with the stored normals, texture-coordinate chaos, unresolved texture names, scrambling state and the outcome of the confident decoder. Existing audits (`abs_audit`, `crossing_audit`, `diag_audit`, `hp_audit`, `spline_audit`, `uv_unjitter_eval`, `confident_eval`) stay the specialised tools.
2. **Live visual checks** through the application and its control protocol (`opts-cli`, [control protocol](../../src/reference/control-protocol.md)) on a representative sample of maps and vehicles, in resumable chunks; screenshots are kept outside the repository and deleted afterwards. Candidates are chosen from the offline audit (flagged assets first, then a stratified sample).
3. **Registry**: every irregularity gets an id, asset reference, kind, detector, evidence, root cause and status.
4. **Fix** root causes that are OPTS bugs with regression tests; document tolerances in the format docs.
5. **Known open app bug**: answering the first-run dialog before the first OMSI scan has finished can leave wizard lists empty, and leaving the wizard may panic with bevy_ecs "Entity despawned" in a deferred command — reproduce and fix if reproducible.

## Acceptance

* Task, progress and registry documents exist and are linked from `SUMMARY.md` and the board.
* Coverage is reported honestly: files checked offline, maps/vehicles/objects checked visually, issues found / fixed / remaining.
* Each fix has a regression test; format doc addenda describe every tolerance.
* Gates green: `cargo fmt --all -- --check`, `cargo clippy --workspace --all-targets --all-features -- -D warnings`, `cargo test --workspace --all-features`, `RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps`, `python3 scripts/check_links.py`, `python3 scripts/gen_roadmap_copy.py --check`.
* No proprietary data committed (ADR-0002): only catalogue ids/paths as defect references, no images.
