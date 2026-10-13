# ⚡ Phase 5: Tooling, Performance & Optimization

*Goal: Profile, benchmark, and optimize engine throughput, and provide developer CLI inspection utilities.*

**Status:** 🟡 Started — control protocol, hot reload, Bevy 0.19 upgrade and audit hardening done; Bevy 0.20 on `master` (merged 2026-10-11); CI written but never run (no hosted repository); benchmarks, LOD and DLSS planned

The checklist for this phase lives in the [roadmap](../README.md); this page lists its milestones.

## Milestones

- M-0028 — Performance Benchmark Suite & Profiling Harness (`opts-benchmark`) — ⏳ Planned (Criterion parser benchmarks, physics step profiling)
- M-0029 — Dynamic LOD, Frustum Culling & Occlusion Systems (`opts-bevy`) — ⏳ Planned (LOD switching, shadow distance culling, mesh instancing)
- M-0030 — Asset Validation & Inspection CLI Tooling (`opts-cli`, `opts-tools`) — ⏳ Planned (asset validator, header dumper, glTF export)
- [M-0043 — CLI Control Protocol & Hot Reload (`opts-cli`, `opts-bevy`)](M-0043.md) — 🟢 Complete (JSON-RPC control server, `opts-cli` client, hot reload; P2 commands deferred)
- [M-0044 — Bevy Engine Upgrade](M-0044.md) — 🟢 Complete (0.14 → 0.19.1)
- [M-0040 — Code-Quality Debt: Size Limits, Panics, Rustdoc, Stubs](M-0040.md) — 🟡 In progress (audit 5037f89a F-005, F-006, F-009–F-012)
- [M-0041 — Documentation Repair & Developer Switch Reference](M-0041.md) — 🟢 Complete (audit 5037f89a F-013, F-018, F-020–F-023)
- [M-0042 — CI & Toolchain Hardening](M-0042.md) — 🟡 Written, never run — blocked on [M-0052](M-0052.md) (audit 5037f89a F-024–F-026)
- [M-0047 — Bevy 0.20 Upgrade](M-0047.md) — 🟡 Merged to `master` 2026-10-11; `Interaction` follow-up open
- [M-0048 — DLSS Upscaling (optional)](M-0048.md) — 📝 Planned
- [M-0052 — Repository Hosting, CI Activation & Commit Gate](M-0052.md) — ⏳ Planned, 🔴 high (critical review 2026-10-11 F-001, F-018–F-020)
- [M-0072 — Settings and Configuration through the Control Protocol](M-0072.md) — 📝 Planned (R-10)
- [M-0081 — `opts-hardware`: Privacy-Respecting Hardware Capability Probe](M-0081.md) — 📝 Planned (R-22)
- [M-0082 — Mesh Instancing Research](M-0082.md) — 🔬 Research branch started (R-23)
- [M-0085 — Reflections Research](M-0085.md) — 🔬 Research branch planned (R-27)
- [M-0074 — Roadmap Adherence and Expansion](M-0074.md) — 📝 Planned, 🔴 high (R-06)
- [M-0054 — Scenery Object Load Verification (`opts-benchmark`)](M-0054.md) — 🟡 Spec done (Phase A), harness planned, 🔴 high

## Checklist

*Mirrored from the [roadmap](../README.md), which is canonical; update it there first.*

*Goal: Profile, benchmark, and optimize engine throughput, and provide developer CLI inspection utilities.*

- [ ] **⚡ Performance & Optimization** — [M-0028](README.md) & [M-0029](README.md)
  - [ ] Implement Criterion microbenchmark suite in `opts-benchmark` for format parsers and vehicle physics step (M-0028 — traces to F-021).
  - [ ] Document LOD (Level of Detail), frustum culling, and occlusion strategies in `docs/src/architecture/lod-culling.md`.
  - [ ] Implement distance-based dynamic LOD mesh switching and shadow culling in `opts-bevy` (M-0029).
  - [ ] Implement async background asset streaming to prevent frame drops during tile boundary crossings.
- [ ] **🧰 Developer CLI Tooling** — [M-0030](README.md)
  - [ ] Document CLI command specifications in `docs/src/reference/opts-tools.md`.
  - [ ] Implement `opts inspect <file>` to validate OMSI asset syntax and detect corrupt files (M-0030).
  - [ ] Implement `opts convert <file> --output <gltf>` to export legacy meshes to standard formats.
- [x] **🎛️ Remote Control & Live Editing** — [M-0043](M-0043.md)
  - [x] Design documents and ADRs: [Control Protocol](../../architecture/control-protocol.md), [Hot Reload](../../architecture/hot-reload.md), ADR-0006, ADR-0007.
  - [x] `opts-cli` client, `opts-bevy::control` server and `opts-bevy::hot_reload` ([final report](../../meta/control-protocol-2026-10-10.md)).
- [ ] **⬆️ Engine Upgrade** — [M-0044](M-0044.md)
  - [x] Migrate Bevy 0.14 → 0.19.1 (done in one step; see [M-0044](M-0044.md)).
- [ ] **⬆️ Bevy 0.20** — [M-0047](M-0047.md) (plan: [strategy](../../architecture/bevy-0-20-dlss.md))
- [ ] **🎞️ DLSS upscaling (optional feature)** — [M-0048](M-0048.md)
- [ ] **🧹 Audit-Driven Hardening (audit 5037f89a, 2026-10-10)**
  - [~] Code-quality debt (rustdoc, panics, fixtures, tile loader done; size splits open): size limits, panics in library code, stubs, rustdoc — [M-0040](M-0040.md) (F-005, F-006, F-009–F-012).
  - [x] Documentation repair and developer switch reference — [M-0041](M-0041.md) (F-013, F-018, F-020–F-023).
  - [~] CI and toolchain hardening (written, unverified until first CI run; binrw open): link check, mdBook build, toolchain pin, Windows job — [M-0042](M-0042.md) (F-024–F-026).
- [ ] **🧾 Critical-Review Work (critical review 2026-10-11, [report](../../meta/critical-review-2026-10-11.md))**
  - [ ] Host the repository, activate CI, fix the three gate failures at HEAD (fmt, clippy, rustdoc), isolate agent sessions in worktrees and gate commits on a staged-tree build — [M-0052](M-0052.md) (F-001, F-018, F-019, F-020).
  - [ ] Stop the size and lint-allow growth: fail CI on new size violations, group load-job parameters, fix stale Bevy rustdoc links (appended to [M-0040](M-0040.md); F-005–F-007).
  - [x] Decide and execute the `bevy-0.20` → `master` merge (appended to [M-0047](M-0047.md); F-019) — fast-forwarded 2026-10-11.
  - [x] Roadmap copy generator with CI check, crate-graph table, crate guides, README and status reconciliation (done in the review pass; F-011, F-013–F-015, F-017).
- [ ] **🔬 Scenery Object Load Verification** — [M-0054](M-0054.md) (spec: [scenery benchmark](../../architecture/scenery-benchmark.md))
  - [x] Specify the `opts-benchmark` scenery harness: one-tile generated map, one object at a time, off-screen capture, automatic placement/geometry/texture checks, script-effect checks, report format (Phase A).
  - [ ] Implement the harness with static checks and calibrate thresholds (Phase B).
  - [ ] Script-effect checks; audio recorded only (Phase C, needs a scenery-script runtime).
  - [ ] Full corpus run and triage report (Phase D); fixes by failure category (Phase E).
- [ ] **🛠️ Settings and Configuration through the Control Protocol** — [M-0072](M-0072.md)
  - [ ] Reproduce the reported gap; confirm/revert and restart-required handling, batch set, export/import, registry-wide coverage test (R-10).
- [ ] **🧭 Roadmap Adherence and Expansion** — [M-0074](M-0074.md)
  - [ ] Drift audit, intake rule and automated consistency checks; gap analysis proposing the next milestones (R-06).
- [~] **🖥️ `opts-hardware`** — [M-0081](M-0081.md)
  - [x] Privacy-respecting hardware capability probe crate (no identifiers, no network, auditable field list) and recommendation functions for tile radius, filtering, look, UI scale (R-22). Linux verified; macOS and Windows compile-checked in CI, untested on real hardware.
  - [ ] Report dialog preview with checkbox; GPU benchmark.
- [ ] **🧬 Mesh Instancing Research** — [M-0082](M-0082.md)
  - [ ] Research branch `research/mesh-instancing`: inventory, Bevy mechanisms, prototype, measurements, recommendation (R-23).
- [ ] **🪞 Reflections Research** — [M-0085](M-0085.md)
  - [ ] Research branch `research/reflections`: mirrors, glass, later wet surfaces; prototype, measurements, recommendation (R-27).
