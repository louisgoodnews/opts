# 🎨 Phase 3: `opts-bevy` Integration (The Visuals)

*Goal: Bridge `opts-core` data and Bevy's ECS and rendering pipeline.*

**Status:** ⏳ Not started — depends on Phase 2

The checklist for this phase lives in the [roadmap](../README.md); this page lists its milestones.

## Milestones

_No milestone has been planned for this phase yet. The next milestone is created by prompt P-002 when the previous phase's blocking items are resolved._

## Checklist

*Mirrored from the [roadmap](../README.md), which is canonical; update it there first.*

*Goal: Bridge the gap between `opts-core` data and Bevy's ECS and rendering pipeline.*

- [ ] **📥 Bevy Asset Loaders**
  - [ ] Implement custom `AssetLoader` for `.o3d` files.
  - [ ] Implement custom `AssetLoader` for `.sco` and `.sli` files.
  - [ ] Register the loaders with the `AssetServer`.
- [ ] **🔄 Coordinate Conversion**
  - [ ] Write systems to convert OMSI Z-up coordinates to Bevy Y-up coordinates.
- [ ] **🌟 Rendering OMSI Assets**
  - [ ] Create a Bevy `Material` from parsed OMSI texture files (`.dds`).
  - [ ] Spawn parsed `.sco` objects as Bevy entities with `Mesh` and `Transform` components.
- [ ] **🎓 Tutorials & Examples**
  - [ ] Write `docs/src/tutorials/first-sco.md`: "Loading a single OMSI scenery object in Bevy."
  - [ ] Write `docs/src/tutorials/first-bus.md`: "Loading a full OMSI bus model with textures."
  - [ ] Create runnable `examples/` in the `opts-bevy` crate matching the tutorials.
