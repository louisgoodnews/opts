# ADR-0014: Separate Legacy OMSI Data from the Native Model

| Metadata | Value |
| :--- | :--- |
| **ADR ID** | 0014 |
| **Status** | ✅ Accepted 2026-10-19 (the project lead accepted the recommended answers of the five decisions "for now" and will adjust after testing; implemented in [M-0132](../roadmap/phase-13/M-0132.md) to [M-0134](../roadmap/phase-13/M-0134.md)) |
| **Date** | 2026-10-19 |
| **Deciders** | Project lead |
| **Supersedes** | None |
| **Superseded by** | None |
| **Milestone** | [M-0131](../roadmap/phase-13/M-0131.md) (Phase 13); the crate-level part of [M-0129](../roadmap/phase-999/M-0129.md) moves here |

---

## 📌 Context

`opts-core` ("Core data structures and parsers for OMSI 2 assets") holds **everything that reads a file**: the OMSI format parsers, the data structures they fill, the localisation machinery and the vehicle-option discovery. Every other crate that touches content depends on it. There is **no native data model**: the renderer, the simulation, the traffic and the game consume the *OMSI-shaped* structures directly (`BusManifest`, `SceneryObject`, `MapTile`, `MapManifest`). A native TOML format would therefore have to be bolted onto structures that carry OMSI's quirks (keyword names, encodings, optional-by-omission fields).

The task of 2026-10-19 asks for a boundary: `opts-omsi` (legacy readers, a leaf adapter), `opts-model` (native TOML), a shared base crate so the two never depend on each other, and engine crates (`opts-bevy`, `opts-sim`, `opts-game`) that do not see OMSI. This ADR is the audit and the proposal (Phase A); nothing has moved.

### What `opts-core` contains (measured 2026-10-19, `crates/opts-core/src`, 24,082 lines in 82 files)

| Bucket | Content | Lines | Notes |
| :--- | :--- | ---: | :--- |
| **OMSI parsers *and* their data, in one file** | `formats/{vehicles,scenery,map,o3d,x,trains,chrono,timetable,weather,humans,money,org,...}`: `bus.rs` 912, `sco.rs` 1,414, `sli.rs` 796, `tile.rs` 1,073, `hof.rs` 704, `entry.rs` (chrono) 836, `model` 1,293, `o3d/*` ≈ 2,400, `x/*` ≈ 1,400, ... | ≈ 20,600 (with the in-file tests) | The structs (`BusManifest`, `ScoObject`, `MapTile`, ...) are defined next to the parser that fills them. They are the OMSI-specific data (bucket 3) and the parsers (bucket 2) at once. |
| **Generic OMSI text machinery** | `formats/text.rs` (keyword-block scanner, encoding detection) | 337 | Shared by almost every text parser; OMSI's keyword grammar, not a general lexer. |
| **Shared geometry and math** | `formats/map/grid.rs` (`TileGrid`: tile coordinates, mercator row width, absolute/local transforms) 537; `formats/scenery/crossing.rs` (`BaseSurface`, a height field) 302; the vertex/triangle structs of `o3d` and `x` | ≈ 1,300 | `glam` types are used; `TileGrid` is used by game, traffic and bevy. These are the real candidates for a base crate, but they are mixed into format modules. |
| **Native, not OMSI at all** | `l10n/*` (fluent subset, language packs, checks, interchange, built-in tables) | ≈ 2,800 | No dependency on `formats`. Used by `opts-gui` (8 uses), `opts-game`, `opts-tools`. It is in `opts-core` for convenience. |
| **Vehicle options** | `setvars.rs` + `setvars/` (option descriptions discovered from `.cti` and scripts) | ≈ 600 | Depends on `formats::cti`; consumed by `opts-game`. |
| **Abstract domain model** | none | 0 | `VehicleDefinition`, `MeshAsset`, `PhysicsProperties` do not exist. |

### How the other crates depend on it (uses of `opts_core::...` in source, 2026-10-19)

| Crate | Uses | Most used modules |
| :--- | ---: | :--- |
| `opts-game` | 170 | `formats::map` 44, `vehicles` 34, `trains` 17, `scenery` 17, `weather` 10, `script_texture` 6, `setvars` 5, `l10n` 9 |
| `opts-bevy` | 130 | `scenery` 26, `vehicles` 26, `o3d` 20, `map` 14, `x` 10, `hof` 9, `weather` 9, `script_texture` 5 |
| `opts-sim` | 31 | `vehicles` 15, `scenery` 6, `constfile` 3, `text` 3, `map` 2 |
| `opts-traffic` | 14 | `map` 9, `scenery` 4, `vehicles` 1 |
| `opts-benchmark` | 11 | `scenery` 5, `map` 3, `o3d` 2 |
| `opts-gui` | 8 | `l10n` only |
| `opts-tools` | 10 | `l10n` 5, formats 5 |
| `opts-catalogue` | 5 | `text`, `map`, `scenery`, `vehicles` |

Files that import `opts_core::formats`: `opts-bevy` 55, `opts-game` 49, `opts-sim` 20, `opts-traffic` 7, `opts-benchmark` 6. **Two facts follow.** (1) The game *reads OMSI files at run time*, through these structures; there is no importer step today. (2) The OMSI **script language** (`.osc`, the varlists, `constfile`) is executed by the engine (`opts-bevy`, `opts-sim`): it is legacy data that the running game needs, not only importer input.

---

## 🎯 Decision

**Recommended: Option 2 (three crates plus an importer) reached in stages, with `opts-core` kept as a temporary facade (Option 3's mechanism) and a ratchet on the remaining OMSI uses.**

```
opts-types      shared, no OMSI, no format code: TileGrid and tile/coordinate maths, ids, error kinds,
                the geometry structs both sides need; depends on glam, thiserror, serde
opts-l10n       the localisation machinery (today opts-core::l10n); depends on serde only
opts-omsi       the legacy readers: formats/* with the OMSI-shaped structs, the keyword lexer, the
                script-variable and const readers; depends on nom, binrw, image, opts-types
opts-model      the native schema (VehicleDefinition, MeshAsset, ..., TOML read/write);
                depends on serde, toml, opts-types
opts-importer   opts-omsi -> opts-model (reads OMSI, writes TOML); depends on opts-omsi, opts-model
opts-core       temporary facade: re-exports opts-omsi / opts-types / opts-l10n at the old paths and
                disappears when the last consumer is migrated
```

### Dependency lists (proposed `Cargo.toml` edges)

| Crate | Depends on | Must **not** depend on |
| :--- | :--- | :--- |
| `opts-types` | `glam`, `serde`, `thiserror` | everything below |
| `opts-l10n` | `serde` (and the existing fluent subset, no new crate) | `opts-omsi`, `opts-model` |
| `opts-omsi` | `nom`, `binrw`, `image`, `thiserror`, `tracing`, `opts-types` | `opts-model`, `opts-bevy`, `opts-sim`, `opts-game` |
| `opts-model` | `serde`, `toml`, `opts-types` | `opts-omsi` |
| `opts-importer` | `opts-omsi`, `opts-model`, `opts-types` | engine crates |
| `opts-sim`, `opts-traffic` | `opts-model`, `opts-types` | `opts-omsi` |
| `opts-bevy` | `bevy`, `opts-model`, `opts-types`, `opts-sim`, `opts-traffic`, `opts-gui` | `opts-omsi` |
| `opts-gui` | `opts-l10n` | `opts-omsi` |
| `opts-game` (composition root) | `opts-bevy`, `opts-model`, `opts-importer`, `opts-l10n`, ... | — (the only engine-side crate allowed to wire the OMSI adapter) |

`opts-model` is deliberately built **before** it is used: M-0134 delivers stubs that compile with `serde` derives and one vehicle round trip; filling the schema belongs to [Phase 14](../roadmap/phase-14/README.md) (vehicle profiles) and to the migration below.

### The three open points of M-0131, answered

1. **`opts-game` and the importer.** The task says `opts-game` depends on `opts-omsi` "for the importer" and also that it must not. Resolution: **`opts-game` is the composition root**. It may depend on `opts-importer` (and through it on `opts-omsi`) to wire the OMSI source; `opts-bevy`, `opts-sim`, `opts-traffic`, `opts-gui`, `opts-catalogue` must not. The importer is its own crate so the CLI (`opts-tools import`) and the game share it.
2. **The run-time loading path.** Until content exists in the native format, the game reads OMSI directly. The loader sits behind a **source trait** in `opts-model` (`ContentSource`: "give me the vehicle, the map tile, the mesh"); the OMSI-backed source (`opts-importer`'s `OmsiSource`) converts on demand and the native TOML source reads files. Engine crates see only the trait and the model types. This is the dated exception: **the facade `opts-core` keeps the old paths working during the migration and is removed when the ratchet below reaches zero.**
3. **Overlap with M-0129.** This ADR takes the `opts-core` split; M-0129 keeps the remaining crate-level decisions (the other crates' merges, splits, renames).

---

## 🧠 Rationale

* **No cycles.** Both parsers need the same small shared types (`TileGrid`, ids, geometry): they live in `opts-types`, below both. `opts-omsi` and `opts-model` never import each other; only the importer knows both.
* **The boundary is a migration, not a move.** 24,082 lines can be moved with `git mv`, but 130 + 170 + 31 + 14 uses of OMSI-shaped types cannot be made native in one step: the *model does not exist yet*. Splitting the crates first and migrating consumers module by module keeps every step green.
* **`l10n` is the free win.** 2,800 lines with no OMSI content and no dependency on the formats; moving it out removes `opts-gui`'s only reason to depend on the OMSI crate (8 uses).
* **Scripts stay OMSI-shaped for now.** The `.osc` VM, the variable lists and the constants are executed by the engine; they get their own seam later (a `opts-script` crate or a model-side script description), tracked as a follow-up, not forced into this split.

## ⚖️ Alternatives Considered

| Option | What | Why not (or when) |
| :--- | :--- | :--- |
| **1. Two crates**: `opts-omsi` + `opts-model`, shared types in `opts-model` | `opts-omsi` depends on `opts-model` | The parser would import the native crate for `TileGrid` and ids: the legacy reader depends on the new format, and the first native type that wants an OMSI helper creates a cycle. |
| **2. Three crates**: `opts-types`, `opts-omsi`, `opts-model` (+ importer, `opts-l10n`) | as above | **Recommended.** |
| **3. Facade only**: `opts-core` stays and re-exports `opts-omsi` + `opts-model` | the implementation moves, the paths do not | Used as the **migration mechanism** (so no `use` statement changes in the first steps), not as the end state: it hides the boundary the task wants visible and every consumer still reaches OMSI through it. |

## 🌊 Consequences

* **Staged delivery** (Phase 13): M-0132 creates `opts-types`, `opts-l10n`, `opts-omsi`, `opts-model`, `opts-importer`, moves `l10n` and the shared geometry (`TileGrid` first); M-0133 `git mv`s the format modules into `opts-omsi` with the facade in `opts-core`; M-0134 adds the `opts-model` stubs, the importer skeleton, CONTRIBUTING rules and the ratchet.
* **A ratchet file** (`docs/dev/omsi-uses-known.txt`, checked by `local-ci.sh`) counts, per engine crate, the files that import OMSI-shaped types. It starts at today's numbers (bevy 55, game 49 → composition root exempt, sim 20, traffic 7) and may only fall. A new import of `opts_omsi` (or the facade) into `opts-bevy`, `opts-sim`, `opts-traffic`, `opts-gui` fails CI.
* **The end state is a follow-up programme**, not this phase: migrating the consumers module by module to model types (suggested order by coupling: `weather`/`humans` (small), `map` (grid first), `scenery`, `vehicles`, then `o3d`/`x`, and the script seam). Each is a milestone of its own once the model has the type; Phase 13 delivers the boundary, the tooling and the ratchet.
* **Behaviour does not change:** the parsers move unchanged; the corpus tests must give identical numbers before and after (M-0133 acceptance).
* **Cost of not doing it:** every new feature on the native format (Phase 14 profiles, the map profiles' static counts) grows more code that mixes OMSI quirks into the engine.

## ❓ Decisions for the project lead

1. **Option 2 with the facade as migration mechanism** (recommended) — or a different option?
2. **`opts-game` as composition root** that may wire the OMSI source (recommended), or must even `opts-game` stay free of `opts-omsi`, with the OMSI source in a separate binary or plugin crate?
3. **`opts-l10n` as its own crate** in this phase (recommended) or leave `l10n` in a slimmed `opts-core`?
4. **The ratchet** as the enforcement of "engine crates do not depend on `opts-omsi`" until the migration is done (recommended) — or a hard dependency rule from day one (impossible without the model)?
5. **Scope of the follow-up migration:** plan it as milestones after M-0134 (recommended), or fold the first module (`weather`, 9 + 10 uses) into this phase as proof?

---

## 🛠️ Implemented (2026-10-19, branch `refactor/crate-separation`)

The workspace now has 18 crates; the crate graph is in [crate-graph](../architecture/crate-graph.md) and the rules in `CONTRIBUTING.md`.

| Crate | What it holds now | Depends on (internal) |
| :--- | :--- | :--- |
| `opts-types` | `TileGrid`, `GridFit`, `mercator_row_width` and the `BorderSpline` trait (the grid is measured from any spline record that implements it) | none |
| `opts-l10n` | the former `opts-core::l10n` (2,764 lines, 23 tests), moved with `git mv` | none |
| `opts-omsi` | the former `opts-core::formats` and `setvars` with their data, 17 corpus tests and the examples, moved with `git mv` (`git log --follow` follows them); 236 unit tests | `opts-types` |
| `opts-model` | `VehicleDefinition`, `AxleDefinition` with a TOML round trip, the `ContentSource` trait (stubs) | `opts-types` |
| `opts-importer` | `import_vehicle` / `vehicle_from_manifest`: a `.bus` becomes a native vehicle that round-trips through TOML | `opts-omsi`, `opts-model` |
| `opts-core` | the facade: `pub use opts_omsi::{formats, setvars}; pub use opts_l10n as l10n;`, no code of its own | `opts-omsi`, `opts-l10n` |

**Deviations from the proposal, and what was learned:**

* **`TileGrid` could not simply move.** Its `measure` function read the OMSI spline record. It is generic now over `opts_types::BorderSpline`, implemented by `opts-omsi` for its `MapSplineInstance`; the eight call sites (`TileGrid::measure(&sample, ...)`) did not change.
* **The planned dependency lists were longer than needed:** `nom`, `binrw`, `petgraph` and `directories` (outside `l10n`) were declared by the old `opts-core` but used nowhere in its source; the new manifests list what is used (`opts-omsi`: `opts-types`, `serde`, `serde_json`, `glam`, `image`, `thiserror`, `tracing`).
* **`opts-gui` no longer depends on the OMSI side at all** (it used `opts-core` for `l10n` only); `opts-game` and `opts-tools` name `opts-l10n` directly.
* **Behaviour is unchanged:** the corpus tests of the whole installation (`OMSI_PATH`) give the same results in `opts-omsi` as they did in `opts-core`; no parser file changed except the import paths and the one trait implementation.
* **The ratchet** (`scripts/omsi_uses.py`, step of `local-ci.sh` and `local-ci.ps1`) starts at the numbers of the audit, counted as **uses** (occurrences; a first version counted files and a refactoring that moved tests into their own files raised it without any new reach into OMSI): `opts-bevy` 130, `opts-sim` 31, `opts-traffic` 14, `opts-catalogue` 5, `opts-gui` 0. The 55, 20, 7 and 1 files of the audit are the files that hold these uses. Only `opts-game` (the composition root), `opts-benchmark` and `opts-tools` are not counted.
* **Not done, by design:** no engine crate uses `opts-model` yet, and `opts-game` does not use `opts-importer`; migrating the consumers (suggested order: `weather`, `humans`, `map`, `scenery`, `vehicles`, `o3d`/`x`, then the script seam) is the follow-up of this phase.
