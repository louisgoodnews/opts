# ⚡ Phase 5: Tooling, Performance & Optimization

*Goal: Profile, benchmark, and optimize engine throughput, and provide developer CLI inspection utilities.*

**Status:** 🟡 In progress (status sync 2026-10-25) — control protocol, hot reload, Bevy 0.19 and 0.20 upgrades, scene trace, settings protocol, instancing and reflections research done; scenery benchmark Phases A, B, D done; CI runs on GitHub (green except Windows `cargo test`), Pages live; benchmarks (M-0028), tile LOD (M-0029) and DLSS open

The checklist for this phase lives in the [roadmap](../README.md); this page lists its milestones.

## Milestones

- M-0028 — Performance Benchmark Suite & Profiling Harness (`opts-benchmark`) — ⏳ Planned (Criterion parser benchmarks, physics step profiling)
- M-0029 — Dynamic LOD, Frustum Culling & Occlusion Systems (`opts-bevy`) — ⏳ Planned (LOD switching, shadow distance culling, mesh instancing)
- M-0030 — Asset Validation & Inspection CLI Tooling (`opts-cli`, `opts-tools`) — 🟡 `opts-tools inspect` delivered 2026-10-10 (`28249ea8`); header dumper and glTF export open
- [M-0043 — CLI Control Protocol & Hot Reload (`opts-cli`, `opts-bevy`)](M-0043.md) — 🟢 Complete (JSON-RPC control server, `opts-cli` client, hot reload; P2 commands deferred)
- [M-0044 — Bevy Engine Upgrade](M-0044.md) — 🟢 Complete (0.14 → 0.19.1)
- [M-0040 — Code-Quality Debt: Size Limits, Panics, Rustdoc, Stubs](M-0040.md) — 🟡 In progress (audit 5037f89a F-005, F-006, F-009–F-012)
- [M-0041 — Documentation Repair & Developer Switch Reference](M-0041.md) — 🟢 Complete (audit 5037f89a F-013, F-018, F-020–F-023)
- [M-0042 — CI & Toolchain Hardening](M-0042.md) — 🟡 Running on GitHub since 2026-10-13; green except the Windows `cargo test` step (audit 5037f89a F-024–F-026)
- [M-0047 — Bevy 0.20 Upgrade](M-0047.md) — 🟢 Complete (`Interaction` follow-up done 2026-10-12, `1d3ad031`)
- [M-0029 — Dynamic LOD, Frustum Culling & Occlusion Systems](M-0029.md) — 🟡 View, shadow, light and detail distance settings live (2026-10-24); tile mesh LOD and instancing open
- [M-0048 — DLSS Upscaling (optional)](M-0048.md) — 📝 Planned
- [M-0052 — Repository Hosting, CI Activation & Commit Gate](M-0052.md) — 🟡 Hosted on GitHub 2026-10-24, CI on every branch, commit gate done, Pages live; a fully green CI run open (critical review 2026-10-11 F-001, F-018–F-020)
- [M-0072 — Settings and Configuration through the Control Protocol](M-0072.md) — 🟢 Complete 2026-10-18 (R-10)
- [M-0081 — `opts-hardware`: Privacy-Respecting Hardware Capability Probe](M-0081.md) — 🟡 Implemented and merged into master (R-22); untested on macOS and Windows hardware
- [M-0082 — Mesh Instancing Research](M-0082.md) — 🔬 Research done 2026-10-13, merged into master (R-23)
- [M-0085 — Reflections Research](M-0085.md) — 🟢 Research complete, merged 2026-10-17; mirrors, glass and wet ground implemented (R-27)
- [M-0087 — Scene Trace: Which Mesh Is Where (Debug Mode)](M-0087.md) — ✅ Implemented 2026-10-14 (R-30)
- [M-0074 — Roadmap Adherence and Expansion](M-0074.md) — 🟡 In progress (audit and checks done), 🔴 high (R-06)
- [M-0054 — Scenery Object Load Verification (`opts-benchmark`)](M-0054.md) — 🟡 Phases A, B, D done; C and E partly (corpus calibration and re-run open), 🔴 high

## Checklist

*Mirrored from the [roadmap](../README.md), which is canonical; update it there first.*

*Goal: Profile, benchmark, and optimize engine throughput, and provide developer CLI inspection utilities.*

- [ ] **⚡ Performance & Optimization** — [M-0028](README.md) & [M-0029](M-0029.md)
  - [ ] Implement Criterion microbenchmark suite in `opts-benchmark` for format parsers and vehicle physics step (M-0028 — traces to F-021).
  - [ ] Document LOD (Level of Detail), frustum culling, and occlusion strategies in `docs/src/architecture/lod-culling.md`.
  - [~] Implement distance-based dynamic LOD mesh switching and shadow culling in `opts-bevy` (M-0029). *Scenery `[LOD]` bands, view, shadow, light and detail distance settings live (2026-10-20/24); tile mesh LOD open.*
  - [~] Implement async background asset streaming to prevent frame drops during tile boundary crossings. (time-budgeted, resumable tile build, [M-0089](../phase-4/M-0089.md) steps 1–2, 2026-10-18).
- [ ] **🧰 Developer CLI Tooling** — [M-0030](README.md)
  - [ ] Document CLI command specifications in `docs/src/reference/opts-tools.md`.
  - [x] Implement `opts inspect <file>` to validate OMSI asset syntax and detect corrupt files (M-0030). — delivered as `opts-tools inspect <file>...` (`28249ea8`, 2026-10-10).
  - [ ] Implement `opts convert <file> --output <gltf>` to export legacy meshes to standard formats.
- [x] **🎛️ Remote Control & Live Editing** — [M-0043](M-0043.md)
  - [x] Design documents and ADRs: [Control Protocol](../../architecture/control-protocol.md), [Hot Reload](../../architecture/hot-reload.md), ADR-0006, ADR-0007.
  - [x] `opts-cli` client, `opts-bevy::control` server and `opts-bevy::hot_reload` ([final report](../../meta/control-protocol-2026-10-10.md)).
- [x] **⬆️ Engine Upgrade** — [M-0044](M-0044.md)
  - [x] Migrate Bevy 0.14 → 0.19.1 (done in one step; see [M-0044](M-0044.md)).
- [x] **⬆️ Bevy 0.20** — [M-0047](M-0047.md) (plan: [strategy](../../architecture/bevy-0-20-dlss.md))
- [ ] **🎞️ DLSS upscaling (optional feature)** — [M-0048](M-0048.md)
- [ ] **🧹 Audit-Driven Hardening (audit 5037f89a, 2026-10-10)**
  - [~] Code-quality debt (rustdoc, panics, fixtures, stubs, Bevy bundle links and file splits done — 0 files over 500 lines of code; function splits and `SystemParam` grouping open): size limits, panics in library code, stubs, rustdoc — [M-0040](M-0040.md) (F-005, F-006, F-009–F-012).
  - [x] Documentation repair and developer switch reference — [M-0041](M-0041.md) (F-013, F-018, F-020–F-023).
  - [~] CI and toolchain hardening (running on GitHub since 2026-10-13; green except Windows `cargo test`; `binrw` no longer built; broken-link proof open): link check, mdBook build, toolchain pin, Windows job — [M-0042](M-0042.md) (F-024–F-026).
- [ ] **🧾 Critical-Review Work (critical review 2026-10-11, [report](../../meta/critical-review-2026-10-11.md))**
  - [ ] 🟡 GitHub hosts the repository (2026-10-24), CI runs on every branch, commits can be gated on the staged tree (`scripts/install-hooks.sh`); Pages deploys and the site is live; open: a fully green `ci.yml` run (Windows `cargo test` fails) and the remote broken-push proof — [M-0052](M-0052.md) (F-001, F-018, F-019, F-020).
  - [~] Stop the size and lint-allow growth: fail CI on new size violations (file ratchet in `local-ci.sh` since 2026-10-19, M-0125; not in hosted CI), group load-job parameters (open; lint allows 37 → 144), fix stale Bevy rustdoc links (done, `3c760f73`) (appended to [M-0040](M-0040.md); F-005–F-007).
  - [x] Decide and execute the `bevy-0.20` → `master` merge (appended to [M-0047](M-0047.md); F-019) — fast-forwarded 2026-10-11.
  - [x] Roadmap copy generator with CI check, crate-graph table, crate guides, README and status reconciliation (done in the review pass; F-011, F-013–F-015, F-017).
- [ ] **🔬 Scenery Object Load Verification** — [M-0054](M-0054.md) (spec: [scenery benchmark](../../architecture/scenery-benchmark.md))
  - [x] Specify the `opts-benchmark` scenery harness: one-tile generated map, one object at a time, off-screen capture, automatic placement/geometry/texture checks, script-effect checks, report format (Phase A).
  - [x] Implement the harness with static checks and calibrate thresholds (Phase B). (done 2026-10-12)
  - [~] Script-effect checks; audio recorded only (Phase C: runtime, scenarios, judgement and tests done 2026-10-12; corpus calibration open).
  - [~] Full corpus run and triage report (Phase D, done 2026-10-12, [triage](../../meta/scenery-benchmark-triage-2026-10-12.md)); fixes by failure category (Phase E: E1 done, E2/E5 fixed without corpus re-run).
- [x] **🛠️ Settings and Configuration through the Control Protocol** — [M-0072](M-0072.md)
  - [x] Reproduce the reported gap; confirm/revert and restart-required handling, batch set, export/import, registry-wide coverage test (R-10).
- [ ] **🧭 Roadmap Adherence and Expansion** — [M-0074](M-0074.md)
  - [~] Drift audit, intake rule and automated consistency checks; gap analysis proposing the next milestones (R-06). (audit and checks done 2026-10-18; regular review moved to M-0119; intake rule open)
- [~] **🖥️ `opts-hardware`** — [M-0081](M-0081.md)
  - [x] Privacy-respecting hardware capability probe crate (no identifiers, no network, auditable field list) and recommendation functions for tile radius, filtering, look, UI scale (R-22). Linux verified; macOS and Windows compile-checked in CI, untested on real hardware.
  - [ ] Report dialog preview with checkbox; GPU benchmark.
- [x] **🧬 Mesh Instancing Research** — [M-0082](M-0082.md)
  - [x] Research branch `research/mesh-instancing`: inventory, Bevy mechanisms, prototype, measurements, recommendation (R-23). (merged 2026-10-17; follow-up milestone of report §9 not yet added)
- [x] **🪞 Reflections Research** — [M-0085](M-0085.md)
  - [x] Research branch `research/reflections`: mirrors, glass, later wet surfaces; prototype, measurements, recommendation (R-27). (merged 2026-10-17; mirrors, glass and wet ground implemented)
