# 🔬 P-1013 — UV Decoder Correctness and Fleet-Wide Validation

You are investigating and fixing a fundamental decoder bug: OMSI's
`.o3d` files encode UV coordinates in a scrambled form, and OPTS's
recovery pass produces incorrect UVs on approximately 23% of
vertices. The visible symptom is texture corruption on the MAN
NL/NG family, but the bug is general — any vehicle whose geometry
samples from a texture atlas can be affected.

The diagnostic work on `fix/man-nlng-materials` (commit `7d13e12`)
has established:

- The textures load correctly.
- The materials bind correctly.
- The UVs are wrong. The correct atlas is being sampled at the
  wrong coordinates.
- The `uvfix` recovery pass is silently wrong on roughly 23% of
  vertices. It reports 0% unresolved, meaning it cannot detect its
  own failures.
- The GN92 folder contains 17 pairs of meshes: one scrambled, one
  plain. Both encode the same geometry. **The plain twins are
  ground truth.**

**This task is a thorough fix.** Do not accept a mitigation. The
goal is a decoder that is correct on all vertices, with detection
for any future case where correctness cannot be guaranteed. A bug
of this class must not appear again.

This is a MULTI-PHASE task with checkpoints. The branch discipline
from P-1011 applies.

---

## 1. Configuration

### Working branch

Create from `bevy-0.20` (or from `fix/man-nlng-materials` if the
user prefers to continue there):

```bash
git checkout bevy-0.20
git pull
git checkout -b fix/uv-decoder-correctness
```

**Branch name:** `fix/uv-decoder-correctness`

Merge target: `bevy-0.20`. Same discipline as P-1011 — no commits
to `bevy-0.20` or `master` until Phase F.

### OMSI installation path
`/home/louisgoodnews/nvme2n1/steamapps/common/OMSI 2`

### Reference vehicles
- `MAN_NG272` — the affected family
- `MAN_SD200` — the working reference
- Any vehicle with a plain/scrambled mesh pair

### OPTS repository root
The current working directory.

### Output targets
- `docs/src/architecture/uv-decoder.md` — the decoder design
- `docs/src/meta/bugs/uv-decoder-YYYY-MM-DD.md` — the bug report
- `crates/opts-core/src/formats/binary/o3d/` — the decoder
- `crates/opts-core/tests/` — regression tests
- `docs/crates/opts-core.md` — updated

### Related context
- `docs/src/formats/binary/o3d.md` — the O3D spec
- `docs/src/formats/binary/o3d-chunks.md`
- `docs/src/meta/bugs/man-nlng-materials-2026-10-12.md` — the
  diagnostic that motivated this task
- `docs/src/architecture/animation-axes.md` — recent decoder work
- The `uvfix` recovery pass source in
  `crates/opts-core/src/formats/binary/o3d/`

---

## 2. Guardrails

### ❌ Forbidden

- Do not disassemble, decompile, or inspect `OMSI.exe` or any DLL.
- Do not modify OMSI vehicle files.
- Do not commit OMSI assets.
- **Do not accept a mitigation as the final fix.** Preferring plain
  twins, flagging unreliable vertices, or falling back to a
  heuristic are acceptable as *interim* measures, but the decoder
  itself must be corrected.
- Do not hardcode per-vehicle or per-mesh exceptions.
- Do not guess at the scramble algorithm. Every change to the
  decoder must be backed by evidence from the plain/scrambled
  mesh pairs.
- Do not skip the fleet-wide validation. A decoder that works on
  one vehicle and fails on another is not fixed.
- Do not commit to `bevy-0.20` or `master` until Phase F.

### ✅ Permitted

- Read every file in the OMSI installation.
- Compare scrambled meshes to their plain twins, byte-for-byte and
  vertex-by-vertex.
- Extend the O3D parser if it discards information needed for
  decoding.
- Add debug overlays and tools showing per-vertex UV values.
- Write decoder test harnesses.
- Add regression tests with real vehicle meshes as fixtures (the
  meshes are referenced, not copied — the tests load them from
  the OMSI installation at runtime, or use synthetic fixtures
  generated from the plain twins).
- Add a fleet-wide validation tool.

---

## 3. Preconditions

Verify and report:

1. Branch is `fix/uv-decoder-correctness`.
2. `docs/src/meta/bugs/man-nlng-materials-2026-10-12.md` exists.
3. The `uvfix` recovery pass exists in the O3D parser.
4. The 17 plain twins in the GN92 folder are identified by path.
5. The MAN NG272 renders with the reported artifacts (reproducible).
6. A working reference (SD200 or equivalent) renders correctly.
7. The debug overlay infrastructure from P-1011 still works.

If any precondition fails, STOP and report.

---

## 4. Phase A — Diagnostic

The diagnostic work on `fix/man-nlng-materials` already produced
partial results. Complete the picture.

### Stage A1 — Plain Twin Inventory

Enumerate every mesh that has both a scrambled and a plain
version, across the entire OMSI installation — not just the GN92
folder. The 17 in GN92 are the known set; there may be more.

For each pair, record:

- Scrambled mesh path and size.
- Plain mesh path and size.
- Vertex count.
- Whether the material slots match.
- Whether the O3D version matches.
- Whether the O3D chunk structure matches.

**Deliverable:** a table in the bug report.

### Stage A2 — Per-Vertex UV Comparison

For each pair, load both meshes and compare per-vertex UVs:

- For every vertex where the scrambled-decoded UV equals the
  plain UV (within ε), classify as **correct**.
- For every vertex where they differ, classify as **wrong**.
- Compute the error magnitude (Euclidean distance in UV space).

Aggregate across all pairs:

- Total vertices compared.
- Correct count and rate.
- Wrong count and rate.
- Error distribution (min, max, median, p95, p99).

**Deliverable:** the aggregate statistics and the distribution.

### Stage A3 — Error Pattern Analysis

For the wrong vertices, look for patterns:

1. **Value-based.** Do the wrong UVs cluster around specific
   values (0.0, 1.0, negative, out-of-range)?
2. **Position-based.** Do wrong vertices correspond to specific
   positions in the mesh (one region, one triangle type, one
   material slot)?
3. **Normal-based.** Do wrong vertices correlate with the vertex
   normal?
4. **Index-based.** Do wrong vertices appear at specific indices
   in the buffer (first N, last N, every K-th)?
5. **Chunk-based.** Do wrong vertices come from specific chunks
   in the O3D?
6. **Bit-based.** Do the raw scrambled values have distinguishing
   bit patterns (high bits set, low bits zero, specific mantissa
   patterns)?

**Deliverable:** a ranked list of candidate patterns, with the
evidence for each.

### Stage A4 — Decoder Algorithm Reconstruction

Read the existing `uvfix` pass. For each step, determine:

- What it assumes about the scramble.
- Where the assumption breaks for the wrong vertices.
- Whether the assumption is verifiable from the plain twins.

**Deliverable:** the current algorithm with line-by-line notes on
where it fails.

### Stage A5 — Checkpoint

Report the diagnostic:

- Total pairs found.
- Correct/wrong vertex counts and rates.
- The error distribution.
- The ranked candidate patterns.
- The specific failure point in the current `uvfix` pass.

**Wait for user approval before Phase B.**

---

## 5. Phase B — Correct Decoder Design

Produce `docs/src/architecture/uv-decoder.md` before writing code.

### Stage B1 — Correct Algorithm

Design a decoder that produces correct UVs for all vertices.

Requirements:

1. **Provable correctness on the plain twins.** Given a scrambled
   mesh, the decoder must produce UVs identical to the plain twin
   for every vertex, within floating-point tolerance.
2. **No silent failures.** If the decoder cannot guarantee
   correctness for a given vertex, it must flag it.
3. **No magic numbers.** Constants in the algorithm must be
   documented with their derivation.
4. **Symmetric with the encoder.** If OMSI's encoder is
   reversible, the decoder should invert it exactly. If the
   encoder is lossy, the decoder should document the loss.

### Stage B2 — Failure Detection

Design how the decoder reports uncertainty:

- On the decoder's output, mark each vertex as
  `UvQuality::Exact`, `UvQuality::Recovered`, or
  `UvQuality::Unreliable`.
- Downstream systems (rendering, validation tools) can act on the
  quality flag.
- The fleet validation tool counts unreliable vertices per
  vehicle.

### Stage B3 — Fallback Strategy

For any vertex the decoder cannot recover exactly:

1. If a plain twin exists on disk, prefer it.
2. Otherwise, use a documented heuristic (spatial interpolation
   from neighboring correct vertices, or geometric projection).
3. Log the fallback at `WARN` once per mesh.

The fallback is not the fix. It's the safety net for cases the
decoder cannot cover.

### Stage B4 — Verification Design

How the decoder's correctness is verified:

- Unit tests using the plain twins as fixtures.
- Assertion: for every pair, decoded UVs equal plain UVs within
  ε = 1e-6.
- Fleet-wide validation: every mesh in the installation is
  decoded, and the unreliable-vertex count is recorded.

### Stage B5 — Checkpoint

Report the design.

**Wait for user approval before Phase C.**

---

## 6. Phase C — Implementation

### Stage C1 — Decoder Rewrite

Rewrite the `uvfix` pass per the Phase B design. Requirements:

1. Preserve the existing behavior where it was correct.
2. Correct the failure mode identified in Phase A.
3. Emit `UvQuality` per vertex.
4. Log statistics: total vertices, exact, recovered, unreliable.

### Stage C2 — Parser Extension

If the O3D parser discards information needed for decoding
(original scrambled values, chunk offsets, etc.), extend it to
retain that information. Document the extension in `o3d.md`.

### Stage C3 — Fallback Implementation

Implement the fallback strategy from Phase B, Stage B3.

### Stage C4 — Regression Tests

Add tests in `crates/opts-core/tests/uv_decoder.rs`:

1. For every plain twin pair, decode the scrambled mesh and
   assert UV equality with the plain.
2. Assert the quality flags: no `Unreliable` on the plain twins.
3. Assert the failure case: a mesh with known-bad UVs (from the
   pre-fix code) is now flagged as such.

The first test must fail against the pre-fix code. If it passes,
the fixture isn't capturing the bug.

### Stage C5 — Checkpoint

Report the implementation and the test results.

**Wait for user approval before Phase D.**

---

## 7. Phase D — Fleet-Wide Validation

### Stage D1 — Decode the Entire Fleet

Run the decoder against every `.o3d` file in the OMSI
installation. Record per file:

- Total vertices.
- Exact count.
- Recovered count.
- Unreliable count.
- Wall time.

### Stage D2 — Coverage Report

Produce `docs/src/meta/uv-decoder-coverage.md`:

- Total meshes processed.
- Total vertices.
- Overall exact/recovered/unreliable rates.
- Vehicles with >1% unreliable vertices (the failure surface).
- Distribution of unreliable count across the fleet.

### Stage D3 — Visual Spot-Check

For each vehicle with >1% unreliable vertices, load it in OPTS
and screenshot the affected geometry. Confirm that:

- Exact vertices render correctly.
- Unreliable vertices have visible artifacts (expected — this is
  the fallback's job).
- The fallback produces acceptable results (or flags the mesh for
  review).

### Stage D4 — NL/NG Confirmation

Load the MAN NG272 with the full fix. Confirm:

- The cockpit steering wheel renders correctly.
- The ceiling renders correctly.
- The external view remains correct.
- No new artifacts.

Screenshot all three.

### Stage D5 — Checkpoint

Report the validation.

**Wait for user approval before Phase E.**

---

## 8. Phase E — Documentation and Tooling

### Stage E1 — Bug Report

Complete `docs/src/meta/bugs/uv-decoder-YYYY-MM-DD.md`:

- Symptom (with screenshots).
- Root cause (the specific failure in the pre-fix decoder).
- Fix (the corrected algorithm).
- Verification (fleet-wide coverage report).
- Regression tests.
- Lessons.

### Stage E2 — Decoder Documentation

Complete `docs/src/architecture/uv-decoder.md`:

- The scramble algorithm (as reconstructed from the plain twins).
- The decode algorithm.
- The failure modes the algorithm protects against.
- The quality flags and their meaning.
- The fallback strategy.

### Stage E3 — Fleet Validation Tool

Add a CLI subcommand (in `opts-tools` or `opts-cli`):

```
opts-cli validate-uvs --input "/path/to/OMSI 2"
```

It runs the decoder across a folder tree and produces the
coverage report. This tool becomes part of the routine validation
suite.

### Stage E4 — Roadmap Note

Add a roadmap item: "Integrate UV validation into CI" — a nightly
or on-demand job that runs the decoder against a reference OMSI
installation and fails if the unreliable rate exceeds a threshold.

### Stage E5 — Checkpoint

Report the documentation.

**Wait for user approval before Phase F.**

---

## 9. Phase F — Merge

Merge `fix/uv-decoder-correctness` into `bevy-0.20` via `--no-ff`.

Before merge:

1. `cargo fmt --check`, `cargo clippy -- -D warnings`,
   `cargo test --workspace` all pass.
2. The MAN NG272 renders correctly (screenshot evidence).
3. The fleet coverage report shows the decoder meets its quality
   targets.

Merge commit:

```
fix(opts-core): correct O3D UV decoder

Replaces the heuristic uvfix pass with a decoder verified against
the plain/scrambled mesh pairs found in the installation. UVs are
now exact for all vertices on validated meshes, and unreliable
vertices are flagged instead of silently corrupted.

See docs/src/meta/bugs/uv-decoder-YYYY-MM-DD.md.
```

Post-merge verification: `cargo check`, `cargo test`, and a
visual confirmation on the MAN NG272.

Do not delete the branch unless the user approves.

---

## 10. The Blank-Parameter Parse Bug

The diagnostic flagged a secondary bug: a keyword with an empty
parameter line takes the next non-blank line as its value. Handle
it as a separate scoped item:

1. Create a short task or commit on the same branch.
2. Fix the parser so an empty parameter line consumes zero lines
   (or consumes the empty line as the empty value, per the spec).
3. Add a regression test.
4. Document the fix in the relevant format spec.

This is small and independent of the UV work. Do it before Phase E
so the branch is clean before the docs are written.

---

## 11. Reporting Cadence

1. Preconditions — user confirmation
2. Phase A (diagnostic) — approval before Phase B
3. Phase B (design) — approval before Phase C
4. Phase C (implementation) — approval before Phase D
5. Phase D (validation) — approval before Phase E
6. Phase E (documentation) — approval before Phase F
7. Phase F (merge) — final report

Format:

```
## P-1013 — Checkpoint N

**Phase:** <name>
**Branch:** fix/uv-decoder-correctness
**Deliverables:** <summary>
**Key finding:** <summary>
**Blockers:** <if any>

Ready to proceed? (yes / adjust)
```

---

## 12. Exit Criteria

- ✅ All work on `fix/uv-decoder-correctness`.
- ✅ `bevy-0.20` untouched until Phase F.
- ✅ Plain twin inventory complete across the installation.
- ✅ Per-vertex error distribution documented.
- ✅ Decoder produces exact UVs on every plain twin.
- ✅ Unreliable vertices are flagged, not silently corrupted.
- ✅ Fallback strategy implemented and documented.
- ✅ Fleet-wide validation run and report produced.
- ✅ MAN NG272 renders correctly (screenshot evidence).
- ✅ Regression tests in `crates/opts-core/tests/uv_decoder.rs`.
- ✅ Blank-parameter parse bug fixed with its own test.
- ✅ `docs/src/architecture/uv-decoder.md` exists.
- ✅ `docs/src/meta/bugs/uv-decoder-YYYY-MM-DD.md` exists.
- ✅ `opts-cli validate-uvs` exists.
- ✅ Roadmap item for CI integration added.
- ✅ `cargo fmt`, `cargo clippy -- -D warnings`,
      `cargo test --workspace` pass.
- ✅ Merged into `bevy-0.20` via `--no-ff`.
- ✅ No OMSI asset committed.
- ✅ No OMSI binary inspected.
- ✅ No per-vehicle hardcoded exceptions.

When finished, respond with the summary from Phase F.

Then respond with the exact string:
"UV DECODER CORRECTNESS COMPLETE — ready for user review"