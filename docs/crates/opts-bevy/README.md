# `opts-bevy`

| Metadata | Value |
| :--- | :--- |
| **Layer** | L5 · Engine |
| **Role** | facade |
| **Kind** | lib |
| **Status** | ⬜ Skeleton |
| **Proposed fate** | ✅ Keep — becomes the `OptsPlugins` group |
| **Last Updated** | 2026-10-04 |

---

## 🎯 Purpose

The single entry point for Bevy applications: a `PluginGroup` that bundles all OPTS engine-adapter plugins, each behind a feature.

## 📤 Public API (high level)

> [!NOTE]
> Planned surface — nothing is implemented yet.

```rust
App::new()
    .add_plugins(DefaultPlugins)
    .add_plugins(opts_bevy::OptsPlugins)   // or .build().disable::<…>()
    .run();
```

## 📦 Dependencies — and why

| Dependency | Why |
| :--- | :--- |
| `bevy` | Engine |
| `opts-bevy-*` *(planned)* | The member plugins |
| `opts-core` | Shared types |

Currently also declares `image` and `ddsfile`; texture decoding should live in `opts-fmt-texture`, with `opts-bevy-assets` converting to Bevy `Image`.

## 🧭 Design Decisions

- Facade only (rule R4). Logic lives in focused `opts-bevy-*` crates that depend on `opts-bevy-core`, never on each other.
- Model types are wrapped in newtypes to derive Bevy traits (orphan rule).

## 🚧 Current State & Known Issues

- Empty `lib.rs`.

## 🔮 Future Direction

Members: `opts-bevy-core`, `-assets`, `-render`, `-camera`, `-input`, `-ui`, `-sim`, `-audio`, `-streaming`, `-devtools`.

## 🔗 Cross-References

- [ADR-0001: Why Bevy and Rust?](../../src/adr/0001-why-bevy-and-rust.md)
- [Phase 3 roadmap](../../src/roadmap/phase-3/README.md)
- [Proposed Architecture](../architecture.md) · [Workspace Audit](../audit.md)
