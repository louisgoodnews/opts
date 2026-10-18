# ADR-0003: Adopt a Layered, Highly Modular Crate Architecture

| Metadata | Value |
| :--- | :--- |
| **ADR ID** | 0003 |
| **Status** | 🟡 Proposed |
| **Date** | 2026-10-04 |
| **Deciders** | OPTS maintainers |
| **Supersedes** | None (amends the crate-boundary rules in Coding Standards §2.11) |
| **Superseded by** | None |

---

## 📌 Context

The workspace has grown from the five crates described in [Coding Standards §2.11](https://github.com/louisgoodnews/opts/blob/main/docs/src/contributing/standards.md) to thirteen, without a shared model of what goes where:

- Parsers have two homes (`opts-core`, `opts-parsers`), and one of them depends on Bevy.
- Three crates besides `opts-bevy` depend on Bevy, contradicting the existing rule.
- Two crates claim the CLI (`opts-cli`, `opts-tools`), and two binaries share the name `opts-game`.
- The workspace currently fails to load because of undeclared workspace dependencies.

Meanwhile the [Vision](../../src/vision.md) asks for a platform where every subsystem is replaceable, and the roadmap adds GTFS/OSM import, a script VM, an editor and multiplayer. All crates are still empty, so restructuring now is free; later it is expensive. Full findings: [Workspace Audit](../audit.md).

## ✅ Decision

Adopt the architecture described in [Proposed Architecture](../architecture.md):

1. **Seven layers** — Foundation, Formats, Model, Pipeline, Simulation, Engine adapters, Applications — plus dev-only crates, with the allowed-dependency matrix defined there.
2. **Formats and Model are decoupled.** Parsers produce format ASTs; the domain model is format-agnostic; only Pipeline crates translate between them. Consequently, parsers move out of `opts-core`.
3. **Bevy only in the Engine and Application layers**, in crates whose name contains `bevy` (plus applications). This replaces "only `opts-bevy` depends on Bevy".
4. **Siblings communicate via `*-api` / `*-core` crates**; facades (`opts-formats`, `opts-sim`, `opts-bevy`) carry no logic.
5. **Rules are machine-checked** via `[package.metadata.opts] layer` + `cargo xtask check-layers`, and `cargo-deny` for Bevy.
6. **Crates are created on demand** — the [catalog](../catalog.md) is a map, not a backlog.

## 🔀 Alternatives Considered

| Alternative | Why not |
| :--- | :--- |
| **Keep a few large crates** (`core`, `bevy`, `sim`, `tools`, `game`) | Simple, but every parser change rebuilds everything that depends on `opts-core`; GTFS import would have to reach into OMSI-shaped types; nothing is replaceable without forking. |
| **One crate per file format** | ~40 parser crates with heavily shared grammar (`[mesh]`, `[script]` blocks appear in `.sco` *and* `.bus`); grouping by family keeps sharing internal. |
| **Bevy behind optional features in lower crates** | Cargo unifies features workspace-wide; one consumer enabling `bevy` would compile it into tools and the server. |
| **Model types are the parser output (no AST)** | Couples the simulation to OMSI file quirks and makes lossless round-tripping (needed by tools and the editor) impossible. |

## ⚖️ Consequences

**Positive**

- Tools, tests and a future headless server build without Bevy.
- GTFS/OSM/native content targets the same model as OMSI content.
- Third parties can add formats, importers, sim systems and plugins through registries.
- Small rebuilds and better parallel compilation.

**Negative**

- More manifests and an extra AST → model translation step.
- API crates require careful design and review.

**Follow-ups (if accepted)**

- Update Coding Standards §2.11 (crate boundaries), §3.4–3.6 (parser location `opts_core::formats` → `opts_fmt_*`), §3.11 (crate doc path).
- Update the **Parser Crate / Parser Module** rows of every format specification.
- Execute the [migration plan](../audit.md#-migration-plan).
- Move this ADR to `docs/src/adr/` alongside ADR-0001/0002 once accepted, if the project prefers a single ADR index.
