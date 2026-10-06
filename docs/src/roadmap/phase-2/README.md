# ⚙️ Phase 2: `opts-core` Implementation (The Parsers)

*Goal: Implement the Rust code that turns the Phase 1 specifications into data structures.*

**Status:** ⏳ Not started — blocked on promotion of the `.o3d`, `.sco`, `.sli`, `.hof` specs from Draft to Specified

The checklist for this phase lives in the [roadmap](../README.md); this page lists its milestones.

## Milestones

_No milestone has been planned for this phase yet. The next milestone is created by prompt P-002 when the previous phase's blocking items are resolved._

## Checklist

*Mirrored from the [roadmap](../README.md), which is canonical; update it there first.*

*Goal: Implement the Rust code that translates the specifications from Phase 1 into usable data structures.*

- [ ] **🧱 Setup `opts-core`**
  - [ ] Convert `src/main.rs` to `src/lib.rs`.
  - [ ] Define the custom `OptsError` enum using `thiserror`.
- [ ] **📝 Implement Text Parsers**
  - [ ] Write `nom` combinators for `.sco`, `.sli`, and `.hof` based strictly on the specs.
  - [ ] Write unit tests for every parser using self-authored (non-copyrighted) test fixtures.
- [ ] **💾 Implement Binary Parsers**
  - [ ] Write `binrw` structs for the `.o3d` format.
  - [ ] Implement tests that verify the byte alignment and chunk parsing.
- [ ] **🗂️ Data Model**
  - [ ] Define the core Rust structs (`ScoObject`, `SliSpline`, `O3dMesh`, `BusConfig`).
  - [ ] Implement `serde` traits for save/load functionality.
