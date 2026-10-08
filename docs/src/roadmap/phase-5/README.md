# ⚡ Phase 5: Tooling, Performance & Optimization

*Goal: Profile, benchmark, and optimize engine throughput, and provide developer CLI inspection utilities.*

**Status:** ⏳ Planned — depends on Phase 4

The checklist for this phase lives in the [roadmap](../README.md); this page lists its milestones.

## Milestones

- [M-0028 — Performance Benchmark Suite & Profiling Harness (`opts-benchmark`)](M-0028.md) — ⏳ Planned (Criterion parser benchmarks, physics step profiling)
- [M-0029 — Dynamic LOD, Frustum Culling & Occlusion Systems (`opts-bevy`)](M-0029.md) — ⏳ Planned (LOD switching, shadow distance culling, mesh instancing)
- [M-0030 — Asset Validation & Inspection CLI Tooling (`opts-cli`, `opts-tools`)](M-0030.md) — ⏳ Planned (asset validator, header dumper, glTF export)

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
