# `opts-game`

| Metadata | Value |
| :--- | :--- |
| **Layer** | L6 · Application |
| **Role** | composition root |
| **Kind** | bin (`opts-game`) |
| **Status** | ⬜ Skeleton |
| **Proposed fate** | ✅ Keep |
| **Last Updated** | 2026-10-04 |

---

## 🎯 Purpose

The simulator executable. Parses arguments, sets up logging, selects plugins and runs the Bevy app.

## 📤 Public API (high level)

> [!NOTE]
> Planned surface — nothing is implemented yet.

Command line (planned): `opts-game --omsi-root <path> --map <name> [--config <file>]`

## 📦 Dependencies — and why

| Dependency | Why |
| :--- | :--- |
| `bevy`, `opts-bevy` | Engine + OPTS plugins |
| `opts-sim` | Simulation (via `opts-bevy-sim` later) |
| `opts-core`, `opts-utils` | Shared types |
| `anyhow` | Top-level error handling (allowed in binaries) |
| `tracing-subscriber` | Logging setup |

## 🧭 Design Decisions

- Composition only (rule R7): no game logic in this crate.
- CLI parser choice should match `opts-cli` (Audit F2).

## 🚧 Current State & Known Issues

- `src/main.rs` with `Hello, world!`.

## 🔮 Future Direction

Phase 3: load and display a map; Phase 4: drive a route.

## 🔗 Cross-References

- [Vision — Stage 3 "Driver"](../../src/vision.md)
- [Proposed Architecture](../architecture.md) · [Workspace Audit](../audit.md)
