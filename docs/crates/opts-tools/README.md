# `opts-tools`

| Metadata | Value |
| :--- | :--- |
| **Layer** | L3 · Pipeline (proposed) |
| **Role** | member |
| **Kind** | bin (`opts`) → lib |
| **Status** | ⬜ Skeleton |
| **Proposed fate** | 🔁 Becomes the tool-command library |
| **Last Updated** | 2026-10-04 |

---

## 🎯 Purpose

Implementations of the asset tooling suite — inspect, convert, validate, graph — as plain library functions that return structured reports. Front ends (`opts-cli`, later `opts-editor`) render them.

## 📤 Public API (high level)

> [!NOTE]
> Planned surface — nothing is implemented yet.

```rust
pub fn inspect(src: &dyn FileSource, path: &VPath, reg: &FormatRegistry) -> InspectReport;
pub fn validate(src: &dyn FileSource, reg: &FormatRegistry) -> ValidationReport;
pub fn graph(src: &dyn FileSource, root: &VPath) -> AssetGraph;
```

## 📦 Dependencies — and why

| Dependency | Why |
| :--- | :--- |
| `opts-formats`, `opts-discovery`, `opts-import-*` | What the commands operate on |
| `image`, `ddsfile` | Texture conversion (later via `opts-fmt-texture`) |
| `thiserror`, `tracing` | Library errors and diagnostics |

Currently binary-style deps (`anyhow`, `tracing-subscriber`) — these move to `opts-cli`.

## 🧭 Design Decisions

- Features per command group so a slim build is possible.
- Reports are `serde`-serialisable for JSON output.

## 🚧 Current State & Known Issues

- `src/main.rs` (`Hello, world!`) building the binary `opts`.

## 🔮 Future Direction

Phase 2 deliverable: `opts inspect file.sco` prints a parsed scenery object.

## 🔗 Cross-References

- [Vision — A Modding Platform](../../src/vision.md)
- [opts-cli](../opts-cli/)
- [Proposed Architecture](../architecture.md) · [Workspace Audit](../audit.md)
