# ⚡ Phase 5: Tooling, Performance & Optimization

*Goal: Profile, benchmark, and optimize engine throughput, and provide developer CLI inspection utilities.*

**Status:** ⏳ Planned — depends on Phase 4

The checklist for this phase lives in the [roadmap](../README.md); this page lists its milestones.

## Milestones

- M-0028 — Performance Benchmark Suite & Profiling Harness (`opts-benchmark`) — ⏳ Planned (Criterion parser benchmarks, physics step profiling)
- M-0029 — Dynamic LOD, Frustum Culling & Occlusion Systems (`opts-bevy`) — ⏳ Planned (LOD switching, shadow distance culling, mesh instancing)
- M-0030 — Asset Validation & Inspection CLI Tooling (`opts-cli`, `opts-tools`) — ⏳ Planned (asset validator, header dumper, glTF export)
- [M-0043 — CLI Control Protocol & Hot Reload (`opts-cli`, `opts-bevy`)](M-0043.md) — 🟢 Complete (JSON-RPC control server, `opts-cli` client, hot reload; P2 commands deferred)
- [M-0044 — Bevy Engine Upgrade](M-0044.md) — 🟢 Complete (0.14 → 0.19.1)
- [M-0040 — Code-Quality Debt: Size Limits, Panics, Rustdoc, Stubs](M-0040.md) — 🟡 In progress (audit 5037f89a F-005, F-006, F-009–F-012)
- [M-0041 — Documentation Repair & Developer Switch Reference](M-0041.md) — 🟢 Complete (audit 5037f89a F-013, F-018, F-020–F-023)
- [M-0042 — CI & Toolchain Hardening](M-0042.md) — 🟡 Written, unverified (audit 5037f89a F-024–F-026)

## Checklist

*Mirrored from the [roadmap](../README.md), which is canonical; update it there first.*

- [ ] **⚡ Performance & Optimization**
  - [ ] Implement Criterion microbenchmarks for all format parsers and physics step in `crates/opts-benchmark`.
  - [ ] Document LOD (Level of Detail), frustum culling, and occlusion strategies in `docs/src/architecture/lod-culling.md`.
  - [ ] Implement distance-based LOD mesh switching in `opts-bevy`.
  - [ ] Implement async background asset streaming to prevent frame drops during tile boundary crossings.
- [ ] **🧰 Developer CLI Tooling**
  - [ ] Implement `opts inspect <file>` to validate OMSI asset syntax and report corruption.
  - [ ] Implement `opts convert <file> --output <gltf>` to export legacy meshes to standard formats.
