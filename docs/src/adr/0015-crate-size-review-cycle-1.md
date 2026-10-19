# ADR-0015: Keep the Crate Layout After the Cycle 1 Size Review

| Metadata | Value |
| :--- | :--- |
| **ADR ID** | 0015 |
| **Status** | ✅ Accepted 2026-10-19 (the project lead left the cycle's decisions to the executing session and signs off on the result; every decision here is "keep", so nothing changes in `Cargo.toml`) |
| **Date** | 2026-10-19 |
| **Deciders** | Project lead |
| **Supersedes** | None |
| **Superseded by** | None |
| **Milestone** | [M-0129](../roadmap/phase-999/M-0129.md) (Phase 999, refactoring cycle 1, batch 5) |

---

## 📌 Context

The structure audit of cycle 1 ([structure-audit-2026-10-19](../meta/structure-audit-2026-10-19.md), section 6) flags crates by two size rules and two naming rules: under 1,000 lines of code, over 10,000 lines of code, a name that does not match the content, a top-level module that shadows the crate. The 17 crates at the end of the cycle (lines of code in `src`, from `python3 scripts/structure_audit.py report`):

| Crate | Lines of code | Flag |
| :--- | ---: | :--- |
| `opts-game` | 60,227 | over 10,000 |
| `opts-bevy` | 31,650 | over 10,000 |
| `opts-gui` | 17,832 | over 10,000 |
| `opts-omsi` | 16,016 | over 10,000 |
| `opts-types` | 454 | under 1,000 |
| `opts-model` | 107 | under 1,000 |
| `opts-importer` | 60 | under 1,000 |
| `opts-core` | 4 | under 1,000 |
| the other nine (`opts-benchmark`, `opts-catalogue`, `opts-cli`, `opts-hardware`, `opts-l10n`, `opts-logging`, `opts-sim`, `opts-tools`, `opts-traffic`) | 1,159 to 7,198 | none |

A scan of module names against crate names finds no top-level module that shadows its crate (for example `opts-catalogue/src/catalogue*` does not exist). `python3 scripts/crate_graph.py` reports 18 nodes and no difference to `docs/src/architecture/crate-graph.md`.

The default of the protocol ([P-1015](../../dev/prompts/process/P-1015-refactoring-cycle.md)) is *keep*: a crate split fragments the API, multiplies manifests and re-exports, and a crate is also a compile unit, so a split can speed up incremental builds (smaller units) or slow clean builds (more link and metadata work). A benefit has to be stated and measured.

---

## 🎯 Decision

**We keep all 17 crates as they are. No crate is split, merged or renamed in cycle 1.** The four large crates stay whole because the file-size rule is met by modules (cycle 1 took the workspace from 64 files over 500 lines of code to none) and none of them has a dependency or an API boundary that the compiler could enforce better in two crates. The four small crates stay because [ADR-0014](0014-data-model-separation.md) created them on purpose and their size is the state of a migration under way. For each crate the section below states what would change the decision.

### The four large crates

| Crate | Biggest parts (lines of code, `src`) | Candidate seam | Why it is not taken now | What would make it worth doing |
| :--- | :--- | :--- | :--- | :--- |
| `opts-game` (60,227) | `in_game` 22.7k, `hud` 6.3k, `new_game_screen` 6.2k, `settings_registry` 4.3k, `settings_screen` 4.0k, `save` 3.7k | `settings_registry` + `settings_screen` (8.3k) into an `opts-settings` crate | The registry and the page refer to `app_state`, the control protocol and the shell (10 `crate::` references in the two directories) and to `opts-gui`. Extracting it needs the game-wide resources moved down first, which is a design change, not a move | A second consumer of the registry (a tool or a launcher), or the settings types freed from `crate::app_state` |
| `opts-bevy` (31,650) | `control` 3.8k, `capture` 3.3k, `scenery_runtime` 2.2k, `camera` 1.7k, `scene_trace` 1.6k | `capture` (3.3k, depends on `rav1e`, `image`, `ddsfile`) into `opts-capture` | The heavy dependency `rav1e` is a real argument, but `capture` and `control` use each other (`capture` uses `control::{params, state}`; `control` registers `capture::screenshot` and `capture::register`), so a crate would need a third crate below both first; a measured clean-build gain would have to be shown before that work | A clean-build profile (`cargo build --timings`) in which `rav1e` sits on the critical path of unrelated work |
| `opts-gui` (17,832) | `components` 7.5k, `theme` 4.0k, `patterns` 2.6k, `interaction` 2.1k, `l10n` 1.6k | `theme` + `tokens` (4.5k) into a design-token crate | `components` use the theme in every widget; the boundary would be crossed by every call | A second renderer or a non-Bevy consumer of the tokens |
| `opts-omsi` (16,016) | `formats` 20.2k incl. tests (`vehicles`, `scenery`, `map`, `o3d`, ...) | one crate per format family | By ADR-0014 it is the **one** leaf adapter for legacy readers; the invariant "no engine, no IO" is checked by `crate_graph.py` and the ratchet `omsi_uses.py`. Splitting it by family would multiply the boundary the ADR tightened | Phase 13 consumer migration finished (the engine crates read `opts-model`, not `opts-omsi`), then the importer-only parts could move into `opts-importer` |

### The four small crates

| Crate | Lines | Role | Decision |
| :--- | ---: | :--- | :--- |
| `opts-types` | 454 | Shared geometry (`TileGrid`, `BorderSpline`) that both legacy readers and the native model need; the base of both | Keep: its job is to be the layer both sides depend on without depending on each other |
| `opts-model` | 107 | The native TOML model (`ContentSource`, vehicle stubs) | Keep: it is the target of the Phase 13 migration and grows as the engine crates move to it ([M-0131](../roadmap/phase-13/M-0131.md) and its successors) |
| `opts-importer` | 60 | The conversion from legacy content to the native model | Keep: a placeholder with a documented purpose; it fills when the first importer is written |
| `opts-core` | 4 | Facade that re-exports `opts-omsi`, `opts-l10n`, `opts-types`, `opts-model` | Keep **for now**; remove when no crate imports `opts_core::` (the ratchet `scripts/omsi_uses.py` counts the remaining uses per engine crate). Removing it earlier would be a mass edit with no benefit |

---

## 🧠 Rationale

| Reason | Explanation |
| :--- | :--- |
| **The size problem is solved at module level** | Cycle 1 split every file over the limit by concern; the largest file is now 497 lines of code. A crate-size rule exists to keep the boundary between concerns visible; the module tree does that here. |
| **No dependency benefit measured** | The only split with a heavy-dependency argument (`capture` and `rav1e`) has no timing evidence; the protocol asks for measurements before a split, not after. |
| **Boundaries that exist are enforced** | The OMSI boundary is enforced by [ADR-0014](0014-data-model-separation.md), `crate_graph.py` and `omsi_uses.py`; a new boundary needs the same machinery to be worth the manifests. |
| **The small crates are a migration, not an anomaly** | Their size is the stage of a planned move; merging them back would undo ADR-0014 and re-create the coupling it removed. |

---

## ⚖️ Alternatives Considered

| Alternative | Reason for Rejection |
| :--- | :--- |
| **Split `opts-game` by feature (`in_game`, `hud`, settings)** | Almost everything in it reads the same Bevy resources and `AppState`; the split would need a "game state" crate below it first, and every feature crate would depend on that crate: the same coupling in more manifests. |
| **Extract `capture` now** | See the table: `capture` and `control` use each other, and nothing is measured. Revisit with a `--timings` profile. |
| **Merge `opts-importer` and `opts-model`** | They are the two sides of the migration (read legacy, write native); keeping them apart keeps the importer out of the engine's dependency tree. |
| **Remove the `opts-core` facade now** | A mass edit of about 360 lines that name `opts_core::` with no behaviour benefit; the facade is the compatibility layer ADR-0014 promised. |

---

## 🌊 Consequences

### ✅ Positive
- No churn: no manifest, re-export or CI change; the crate graph stays as documented.
- Each flagged crate has a recorded reason and a recorded trigger, so the next cycle does not repeat the debate.

### ⚠️ Negative
- `opts-game` stays large (60k lines of code); a full build and test of the crate remains the slowest step of the workspace.
- The crate-size flag of the audit stays on for the four large crates; the audit notes column should point here instead of re-listing them.

### 🔄 Neutral
- Reopen at the next refactoring cycle or when a trigger of the tables above happens; a split then needs its own ADR with the measured clean and incremental build times before and after.

---

## 🔗 References

- [ADR-0014: Separate Legacy OMSI Data from the Native Model](0014-data-model-separation.md)
- [Structure audit 2026-10-19](../meta/structure-audit-2026-10-19.md), [refactoring plan](../architecture/refactoring-plan.md), [cycle report](../meta/structure-refactoring-2026-10-19.md)
- [Crate graph](../architecture/crate-graph.md), `scripts/crate_graph.py`, `scripts/omsi_uses.py`
