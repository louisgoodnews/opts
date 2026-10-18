# 📐 Crate Conventions

| Metadata | Value |
| :--- | :--- |
| **Status** | 🟡 Proposed — complements [Coding Standards](https://github.com/louisgoodnews/opts/blob/main/docs/src/contributing/standards.md) |
| **Last Updated** | 2026-10-04 |

Conventions that make ~50 crates feel like one codebase.

---

## 🏷️ Naming

| Pattern | Layer | Example |
| :--- | :--- | :--- |
| `opts-<thing>` | Foundation, Pipeline, Applications | `opts-vfs`, `opts-discovery`, `opts-game` |
| `opts-fmt-<family>` | Formats | `opts-fmt-scenery` |
| `opts-model-<context>` | Model | `opts-model-transit` |
| `opts-import-<source>` / `opts-export-<target>` | Pipeline | `opts-import-gtfs` |
| `opts-sim-<system>` | Simulation | `opts-sim-physics` |
| `opts-bevy-<concern>` | Engine | `opts-bevy-render` |
| `*-api` | API crate of a layer/group (traits, events, IDs; no logic) | `opts-sim-api` |
| `*-core` / `*-common` | Shared base of a sibling group | `opts-bevy-core`, `opts-fmt-common` |
| no suffix, group name | Facade | `opts-formats`, `opts-sim`, `opts-bevy` |

The `bevy` infix is reserved: a crate whose name contains it may depend on Bevy, and no other crate may.

---

## 📄 Manifest Template

```toml
[package]
name = "opts-fmt-scenery"
description = "Parsers for OMSI 2 scenery formats (.sco, .sli)."
version.workspace = true
edition.workspace = true
rust-version.workspace = true
authors.workspace = true
license.workspace = true
repository.workspace = true
publish = false            # until Q5 in the architecture is decided

[package.metadata.opts]
layer = "formats"          # foundation | formats | model | pipeline | sim | engine | app | dev
role  = "member"           # member | api | facade

[features]
default = ["sco", "sli"]
sco = []
sli = []

[dependencies]
opts-fmt-api    = { workspace = true }
opts-fmt-common = { workspace = true }

[dev-dependencies]
opts-testkit  = { workspace = true }
opts-fixtures = { workspace = true }

[lints]
workspace = true
```

- Every external dependency is declared once in `[workspace.dependencies]` and consumed with `workspace = true`.
- `[lints] workspace = true` applies shared Clippy/rustc lints (e.g. `unwrap_used = "deny"` for libraries).

---

## 🚩 Features

- **Additive only.** Enabling a feature never removes or changes an item.
- **Per-format / per-system flags** in member crates; facades forward them (`opts-formats/sco = ["opts-fmt-scenery/sco"]`).
- **`full`** in every facade, and `default = ["full"]` — minimal builds opt out with `default-features = false`.
- **Common optional features:** `serde` (derive `Serialize`/`Deserialize`), `tracing` (instrumentation). Never `bevy`.

---

## ⚠️ Errors

- Each library crate defines its own error enum with `thiserror` (`ScoError`, `VfsError`, `ImportError`).
- Errors from lower layers are wrapped with `#[from]`, never flattened into strings.
- Parsers return **`Parsed<T> { value: Option<T>, diagnostics: Diagnostics }`** so a single run reports every problem in a file.
- `anyhow` only in Applications and `xtask`.

---

## 📤 Public API

- Re-export the crate's main types at the root; keep modules for organisation.
- Do not re-export dependencies (see [Coding Standards](https://github.com/louisgoodnews/opts/blob/main/docs/src/contributing/standards.md)); API crates are the exception for types that appear in their trait signatures.
- Mark extensible enums `#[non_exhaustive]`.
- Every public item has rustdoc; every crate has a `//!` header pointing at its page in this book.

---

## 🧪 Tests

| Kind | Location | Fixtures |
| :--- | :--- | :--- |
| Unit | `src/**` `#[cfg(test)]` | inline strings/bytes |
| Integration | `tests/` | `opts-fixtures` (clean-room, self-authored) |
| Corpus | `opts-benchmark`, opt-in via env var pointing at a local OMSI install | user-provided, **never committed** |

---

## 📝 Crate Page Template

Each crate has `docs/crates/<crate>/README.md` following this outline (required content from [PROMPTS.md § Documentation in docs/crates/](https://github.com/louisgoodnews/opts/blob/main/docs/dev/PROMPTS.md)):

```markdown
# `opts-<name>`

| Metadata | Value |
| :--- | :--- |
| **Layer** | Formats |
| **Role** | member / api / facade |
| **Kind** | lib / bin (`<binary name>`) |
| **Status** | ⬜ Skeleton / 🟡 In progress / ✅ Stable |
| **Last Updated** | YYYY-MM-DD |

## 🎯 Purpose
## 📤 Public API (high level)
## 📦 Dependencies — and why
## 🧭 Design Decisions
## 🚧 Current State & Known Issues
## 🔮 Future Direction
## 🔗 Cross-References (specs, ADRs, milestones)
```
