# 🔤 Encoding Audit Report — 2026-10-06

This audit report records the results of the repository-wide UTF-8 encoding sweep across all documentation and Markdown files in the OPTS repository, executed in accordance with Phase A of [`docs/prompts/tasks/Encoding-Sweep Repair — Task Prompt.md`](file:///home/louisgoodnews/Dokumente/opts/docs/prompts/tasks/Encoding-Sweep%20Repair%20%E2%80%94%20Task%20Prompt.md).

---

## 1. Executive Summary

- **Audit Date:** 2026-10-06
- **Scope:** All Markdown files (`*.md`) in `docs/src/`, `docs/crates/`, `docs/dev/`, `dev/`, and repository root.
- **Total Files Scanned:** 126
- **Clean Files:** 121 (96.0%)
- **Fixable Files:** 0 (0.0%)
- **Ambiguous Files:** 5 (4.0%) — verified as intentional Unicode characters and illustrative examples.
- **Corrupted Files Requiring Repair:** 0

### Summary Counts

| Category | Count | Description |
| :--- | ---: | :--- |
| **Clean** | 121 | Zero mojibake signatures found; valid UTF-8. |
| **Fixable** | 0 | Automated round-trip recovers corrupted text. |
| **Ambiguous** | 5 | Signatures detected, but manual review confirms intentional usage. |
| **Manually Reviewed** | 5 | All 5 ambiguous files reviewed and confirmed clean. |
| **Total in Scope** | 126 | Entire repository Markdown documentation suite. |

---

## 2. Key Finding: `docs/src/formats/text/money.md`

The task prompt highlighted `docs/src/formats/text/money.md` as an example of known mojibake corruption (`тАФ` $\to$ `—`, `ЁЯЯв` $\to$ `🟢`, `тЭУ` $\to$ `❓`, `ЁЯУЦ` $\to$ `📖`, `тЬЕ` $\to$ `✅`).

Upon comprehensive byte-level and character-level inspection:
- `docs/src/formats/text/money.md` is **already clean UTF-8**.
- All em dashes (`—`), emojis (`🟢`, `❓`, `📖`, `✅`), and symbols are correctly encoded as multi-byte UTF-8 sequences.
- Zero CP866/CP1252 mojibake artifacts (`тА`, `ЁЯ`, `тЭ`, `â€`, etc.) exist in the file.
- Git commit history confirms that `money.md` was committed in clean UTF-8 during the Documentation Freeze (`commit 9d8771a`).

---

## 3. Detailed Review of Ambiguous Files

Five files matched single high-byte or sub-signature search terms (`├`, `Ã¶`, `ÃŸ`). Each has been manually inspected in its exact context:

### 1. `docs/crates/architecture.md`
- **Matched Signature:** `├` (7 occurrences)
- **Line Numbers:** Lines 377–383
- **Context:** Crate layer directory tree diagrams:
  ```text
  ├── foundation/   opts-utils, opts-diagnostics, opts-text, opts-vfs, opts-geom
  ├── formats/      opts-fmt-api, opts-fmt-common, opts-fmt-*, opts-formats
  ├── model/        opts-core, opts-model-*, opts-mod-api
  ├── pipeline/     opts-import-*, opts-discovery, opts-assets, opts-cache, opts-tools
  ├── sim/          opts-sim-api, opts-sim, opts-sim-*, opts-script, opts-save, opts-replay
  ├── engine/       opts-bevy-core, opts-bevy-*, opts-bevy
  ├── apps/         opts-game, opts-cli, opts-editor, opts-server
  ```
- **Finding:** Legitimate Unicode box-drawing character `U+251C` (`BOX DRAWINGS LIGHT VERTICAL AND RIGHT`). Not mojibake.
- **Classification:** **Clean / Intentional**.

### 2. `docs/dev/PROMPTS.md`
- **Matched Signature:** `├` (6 occurrences)
- **Line Numbers:** Lines 19, 22, 25, 28, 31, 34
- **Context:** Unicode border frames in table layout:
  ```text
  ├─────────────────────────────────────────────────────────────┤
  ```
- **Finding:** Legitimate Unicode box-drawing character `U+251C`. Not mojibake.
- **Classification:** **Clean / Intentional**.

### 3. `docs/src/contributing/standards.md`
- **Matched Signature:** `├` (15 occurrences)
- **Line Numbers:** Lines 441, 443, 600, 601, 614, 616, 617, 618, 619, 620, 622, 623, 624, 625
- **Context:** Crate and source module hierarchy trees:
  ```text
  ├── opts-bevy
  ├── opts-sim
  ...
  ├── lexer.rs    // bytes → chunks
  ├── parser.rs   // chunks → RawO3d
  ├── o3d/
  │   ├── mod.rs
  │   ├── header.rs
  ```
- **Finding:** Legitimate Unicode box-drawing character `U+251C`. Not mojibake.
- **Classification:** **Clean / Intentional**.

### 4. `docs/src/formats/binary/o3d.md`
- **Matched Signature:** `├` (1 occurrence)
- **Line Numbers:** Line 42
- **Context:** Table framing border:
  ```text
  ├──────────────────────────────────────────────┤
  ```
- **Finding:** Legitimate Unicode box-drawing character `U+251C`. Not mojibake.
- **Classification:** **Clean / Intentional**.

### 5. `docs/src/formats/text/sco.md`
- **Matched Signature:** `Ã¶`, `ÃŸ` (1 occurrence each)
- **Line Number:** Line 90
- **Context:** Explanatory documentation of real-world OMSI encoding defects:
  ```markdown
  CP1252 in the vast majority of files. A few add-ons are saved in UTF-8 (their comments show up as `GrÃ¶ÃŸer` when decoded as CP1252). All semantic data (file names, keywords, numbers) is ASCII. Recommended: try UTF-8 first, fall back to CP1252, and never fail on bytes.
  ```
- **Finding:** Intentional illustrative example showing the visual effect of decoding UTF-8 German text ("Größer") under CP1252. Not corrupted documentation text.
- **Classification:** **Clean / Intentional Example**.

---

## 4. Full Audit Table

| File | Signatures Found | Round-Trip Valid | Category |
| :--- | :--- | :--- | :--- |
| `CODE_OF_CONDUCT.md` | — | N/A | Clean |
| `CONTRIBUTING.md` | — | N/A | Clean |
| `ISSUE_TEMPLATE.md` | — | N/A | Clean |
| `README.md` | — | N/A | Clean |
| `ROADMAP.md` | — | N/A | Clean |
| `docs/crates/README.md` | — | N/A | Clean |
| `docs/crates/SUMMARY.md` | — | N/A | Clean |
| `docs/crates/adr/0003-layered-crate-architecture.md` | — | N/A | Clean |
| `docs/crates/architecture.md` | `├` | ⚠️ | Ambiguous (Box drawing) |
| `docs/crates/audit.md` | — | N/A | Clean |
| `docs/crates/catalog.md` | — | N/A | Clean |
| `docs/crates/conventions.md` | — | N/A | Clean |
| `docs/crates/opts-assets/README.md` | — | N/A | Clean |
| `docs/crates/opts-benchmark/README.md` | — | N/A | Clean |
| `docs/crates/opts-bevy/README.md` | — | N/A | Clean |
| `docs/crates/opts-cli/README.md` | — | N/A | Clean |
| `docs/crates/opts-core/README.md` | — | N/A | Clean |
| `docs/crates/opts-discovery/README.md` | — | N/A | Clean |
| `docs/crates/opts-game/README.md` | — | N/A | Clean |
| `docs/crates/opts-graphics/README.md` | — | N/A | Clean |
| `docs/crates/opts-gui/README.md` | — | N/A | Clean |
| `docs/crates/opts-parsers/README.md` | — | N/A | Clean |
| `docs/crates/opts-sim/README.md` | — | N/A | Clean |
| `docs/crates/opts-tools/README.md` | — | N/A | Clean |
| `docs/crates/opts-utils/README.md` | — | N/A | Clean |
| `docs/dev/BOARD.md` | — | N/A | Clean |
| `docs/dev/Documentation Freeze — Task Prompt.md` | — | N/A | Clean |
| `docs/dev/PROMPTS.md` | `├` | ⚠️ | Ambiguous (Box drawing) |
| `docs/src/SUMMARY.md` | — | N/A | Clean |
| `docs/src/adr/0001-why-bevy-and-rust.md` | — | N/A | Clean |
| `docs/src/adr/0002-clean-room-policy.md` | — | N/A | Clean |
| `docs/src/adr/template.md` | — | N/A | Clean |
| `docs/src/architecture/asset-pipeline.md` | — | N/A | Clean |
| `docs/src/architecture/coordinates.md` | — | N/A | Clean |
| `docs/src/architecture/crate-structure.md` | — | N/A | Clean |
| `docs/src/architecture/determinism-replays.md` | — | N/A | Clean |
| `docs/src/architecture/error-handling.md` | — | N/A | Clean |
| `docs/src/architecture/overview.md` | — | N/A | Clean |
| `docs/src/contributing/documentation-standards.md` | — | N/A | Clean |
| `docs/src/contributing/legal-licensing.md` | — | N/A | Clean |
| `docs/src/contributing/release-process.md` | — | N/A | Clean |
| `docs/src/contributing/setup.md` | — | N/A | Clean |
| `docs/src/contributing/standards.md` | `├` | ⚠️ | Ambiguous (Tree diagram) |
| `docs/src/formats/README.md` | — | N/A | Clean |
| `docs/src/formats/binary/README.md` | — | N/A | Clean |
| `docs/src/formats/binary/o3d-chunks.md` | — | N/A | Clean |
| `docs/src/formats/binary/o3d.md` | `├` | ⚠️ | Ambiguous (Box drawing) |
| `docs/src/formats/binary/x.md` | — | N/A | Clean |
| `docs/src/formats/community-crosscheck.md` | — | N/A | Clean |
| `docs/src/formats/dat.md` | — | N/A | Clean |
| `docs/src/formats/gap-analysis.md` | — | N/A | Clean |
| `docs/src/formats/html.md` | — | N/A | Clean |
| `docs/src/formats/inventory.md` | — | N/A | Clean |
| `docs/src/formats/ipr.md` | — | N/A | Clean |
| `docs/src/formats/itx.md` | — | N/A | Clean |
| `docs/src/formats/kennz.md` | — | N/A | Clean |
| `docs/src/formats/kyb.md` | — | N/A | Clean |
| `docs/src/formats/map/README.md` | — | N/A | Clean |
| `docs/src/formats/map/chrono.md` | — | N/A | Clean |
| `docs/src/formats/map/global-cfg.md` | — | N/A | Clean |
| `docs/src/formats/map/lists.md` | — | N/A | Clean |
| `docs/src/formats/map/tiles.md` | — | N/A | Clean |
| `docs/src/formats/map/timetables.md` | — | N/A | Clean |
| `docs/src/formats/off.md` | — | N/A | Clean |
| `docs/src/formats/overview.md` | — | N/A | Clean |
| `docs/src/formats/sichtbar.md` | — | N/A | Clean |
| `docs/src/formats/text/README.md` | — | N/A | Clean |
| `docs/src/formats/text/cfg.md` | — | N/A | Clean |
| `docs/src/formats/text/engine-config.md` | — | N/A | Clean |
| `docs/src/formats/text/environment.md` | — | N/A | Clean |
| `docs/src/formats/text/fonts.md` | — | N/A | Clean |
| `docs/src/formats/text/misc-types.md` | — | N/A | Clean |
| `docs/src/formats/text/money.md` | — | N/A | Clean |
| `docs/src/formats/text/opl.md` | — | N/A | Clean |
| `docs/src/formats/text/org.md` | — | N/A | Clean |
| `docs/src/formats/text/repaints.md` | — | N/A | Clean |
| `docs/src/formats/text/scenery-scripts.md` | — | N/A | Clean |
| `docs/src/formats/text/sco-sli-audit.md` | — | N/A | Clean |
| `docs/src/formats/text/sco.md` | `Ã¶, ÃŸ` | ⚠️ | Ambiguous (Intentional example) |
| `docs/src/formats/text/sli.md` | — | N/A | Clean |
| `docs/src/formats/text/texture-cfg.md` | — | N/A | Clean |
| `docs/src/formats/text/ticketpacks.md` | — | N/A | Clean |
| `docs/src/formats/text/vehicles/bus.md` | — | N/A | Clean |
| `docs/src/formats/text/vehicles/const.md` | — | N/A | Clean |
| `docs/src/formats/text/vehicles/hof.md` | — | N/A | Clean |
| `docs/src/formats/text/vehicles/model_cfg.md` | — | N/A | Clean |
| `docs/src/formats/text/vehicles/osc.md` | — | N/A | Clean |
| `docs/src/formats/text/vehicles/passengercabin_cfg.md` | — | N/A | Clean |
| `docs/src/formats/text/vehicles/paths_cfg.md` | — | N/A | Clean |
| `docs/src/formats/text/vehicles/predefined-variables.md` | — | N/A | Clean |
| `docs/src/formats/text/vehicles/script-sets.md` | — | N/A | Clean |
| `docs/src/formats/text/vehicles/sound_cfg.md` | — | N/A | Clean |
| `docs/src/formats/text/vehicles/varnamelist.md` | — | N/A | Clean |
| `docs/src/formats/text/vehicles/vehicle-corpus-audit.md` | — | N/A | Clean |
| `docs/src/formats/textures.md` | — | N/A | Clean |
| `docs/src/formats/wav.md` | — | N/A | Clean |
| `docs/src/formats/xsli.md` | — | N/A | Clean |
| `docs/src/glossary.md` | — | N/A | Clean |
| `docs/src/introduction.md` | — | N/A | Clean |
| `docs/src/meta/cross-references.md` | — | N/A | Clean |
| `docs/src/meta/documentation-freeze-2026-10-06.md` | — | N/A | Clean |
| `docs/src/meta/link-integrity.md` | — | N/A | Clean |
| `docs/src/meta/provenance-audit.md` | — | N/A | Clean |
| `docs/src/meta/roadmap-reconciliation.md` | — | N/A | Clean |
| `docs/src/meta/status-audit.md` | — | N/A | Clean |
| `docs/src/roadmap/README.md` | — | N/A | Clean |
| `docs/src/roadmap/phase-0/M-0000.md` | — | N/A | Clean |
| `docs/src/roadmap/phase-0/README.md` | — | N/A | Clean |
| `docs/src/roadmap/phase-1/M-0001.md` | — | N/A | Clean |
| `docs/src/roadmap/phase-1/M-0002.md` | — | N/A | Clean |
| `docs/src/roadmap/phase-1/M-0003.md` | — | N/A | Clean |
| `docs/src/roadmap/phase-1/M-0004.md` | — | N/A | Clean |
| `docs/src/roadmap/phase-1/M-0005.md` | — | N/A | Clean |
| `docs/src/roadmap/phase-1/M-0006.md` | — | N/A | Clean |
| `docs/src/roadmap/phase-1/M-0007.md` | — | N/A | Clean |
| `docs/src/roadmap/phase-1/M-0008.md` | — | N/A | Clean |
| `docs/src/roadmap/phase-1/M-0009.md` | — | N/A | Clean |
| `docs/src/roadmap/phase-1/M-0010.md` | — | N/A | Clean |
| `docs/src/roadmap/phase-1/M-0011.md` | — | N/A | Clean |
| `docs/src/roadmap/phase-1/M-0012.md` | — | N/A | Clean |
| `docs/src/roadmap/phase-2/README.md` | — | N/A | Clean |
| `docs/src/roadmap/phase-3/README.md` | — | N/A | Clean |
| `docs/src/roadmap/phase-4/README.md` | — | N/A | Clean |
| `docs/src/roadmap/phase-5/README.md` | — | N/A | Clean |
| `docs/src/vision.md` | — | N/A | Clean |

---

## 5. Next Steps

In accordance with Phase A Stage A3, automated modification is paused pending user confirmation of these detection findings. Because all 126 files are clean or contain only intentional Unicode/examples, Phase B will confirm baseline integrity, update `CONTRIBUTING.md` with UTF-8 encoding requirements and pre-commit checks, and produce the final repair/audit report `docs/src/meta/encoding-repair-2026-10-06.md`.
