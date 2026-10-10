# 🔬 Stage 2 Deep-Dive — Category Sampling

You have completed the top-level survey (Stage 1) and the two-mode
classification amendment. Stage 1 revealed that the OMSI tree is not
uniform, and the amendment identified four modes rather than two.

This stage performs the deep-dive: sampling each category to
understand what an "entity" actually is, how component files relate
to entities, and how the schema must model each mode.

This is a RESEARCH task. You are not writing Rust code. You are not
drafting the schema yet. You are gathering evidence.

---

## 1. Configuration

### OMSI installation path
/home/louisgoodnews/nvme2n1/steamapps/common/OMSI 2

If this path does not exist or is inaccessible, STOP and report.

### OPTS repository root
The current working directory. Confirm by checking for a `Cargo.toml`
with `[workspace]` and a `docs/src/SUMMARY.md`.

### Output target
Append findings to `docs/src/architecture/catalogue-schema.md`
under a new section titled `## 🔍 Stage 2 — Deep-Dive Findings`.

If that file does not yet exist, STOP and report — the amendment
was supposed to create it.

### Reference documents
- `docs/src/formats/` — existing format specs (for marker extension
  meaning)
- `docs/src/formats/misc-types.md` — for `.ovh` and other minor
  types
- `docs/src/formats/map/timetables.md` — for `global.cfg` and
  chrono variants
- `docs/src/formats/inventory.md` — for the extension catalogue

---

## 2. Guardrails (restated)

### ❌ Forbidden

- Do not disassemble, decompile, or inspect `OMSI.exe` or any DLL.
- Do not sample any file with extension `.exe`, `.dll`, `.so`,
  `.pdb`, `.lib`, or `.obj`.
- Do not modify any file in the OMSI installation.
- Do not copy any OMSI file into the repository.
- Do not quote more than 20 consecutive lines from any OMSI file.
- Do not write Rust code.
- Do not promote any spec from 🟡 Draft to 🟢 Specified.
- Do not modify Phase 1 or Phase 2 milestones.

### ✅ Permitted

- Walk directories, count files, sample directory names.
- Read text files (`.cfg`, `.txt`, `.bus`, `.sco`, `.sli`, `.ovh`,
  `.otp`, `.zug`, `.hum`, `.osn`, `.olf`, `.oop`, `.cti`, `.org`).
- Read the first 200 bytes of binary files (`.o3d`, `.x`) to check
  magic numbers, without analyzing structure.
- Cross-reference existing format specs.
- Append findings to `docs/src/architecture/catalogue-schema.md`.

---

## 3. The Refined Mode Model

Stage 1's two-mode amendment was almost right, but the deep-dive
found three structural modes plus exclusions. Use this model:

| Mode | Definition | Evidence Required |
| :--- | :--- | :--- |
| **Folder-entity** | One folder = one entity. Folder contains a marker file. | Confirm marker extension and folder depth pattern. |
| **File-entity** | One file = one entity. Folder is organizational only. | Confirm marker extension and approximate entity count per folder. |
| **Pool** | Files are resources, not entities. Referenced by entities elsewhere. | Identify what references them and by what path convention. |
| **Excluded** | Not game content. Skipped at walk time. | Already documented in the amendment. |

### Reclassification Required

The Stage 2 Amendment classified the following as **Hierarchical**.
They are actually **Pool** and must be reclassified:

| Directory | Old Mode | New Mode | Reason |
| :--- | :--- | :--- | :--- |
| `Texture` | Hierarchical | **Pool** | Files are environmental assets, not entities. |
| `Sounds` | Hierarchical | **Pool** | Files are audio assets, not entities. |
| `helper` | Flat | **Pool** | Editor gizmos referenced by tooling, not content. |
| `program` | Flat | **Pool** | Core script lists referenced by the engine. |
| `Scripts` | Flat | **Pool** | Shared base scripts referenced by vehicles. |

Document each reclassification with a one-sentence justification.

---

## 4. Amendment Investigations

Before the deep-dive, resolve three specific open questions that
Stage 1 and the amendment surfaced.

### Investigation A — `.ovh` Extension

**Context:** Stage 1 found 1,052 `.ovh` files inside `Vehicles`,
alongside 1,485 `.bus` files. The earlier `misc-types.md` spec may
document `.ovh`, but its role as a **marker file** must be confirmed.

**Task:**
1. Read `docs/src/formats/misc-types.md`. Quote the relevant
   passage describing `.ovh`.
2. Sample 5 `.ovh` files. For each, record:
   - Full relative path
   - First 10 lines of content
   - Whether it resembles a `.bus` file or has different keywords
3. Sample 5 folder pairs where both `.ovh` and `.bus` exist side by
   side. Are they alternates for the same vehicle, or separate
   entities?
4. Sample 5 folders that contain **only** `.ovh` (no `.bus`). Are
   these AI-only vehicles?

**Deliverable:** A subsection in the deep-dive findings titled
"Investigation A — `.ovh`" with:
- The spec's current description (if any)
- Sample content (≤ 10 lines per sample)
- A conclusion: is `.ovh` a marker for a distinct entity type, or
  a companion to `.bus`?
- Schema implication: does this need its own `category` value, a
  `entity_tags` entry, or both?

### Investigation B — Maps with Multiple `global.cfg`

**Context:** Stage 1 found **46 `global.cfg` files across 43 map
folders**. Three map folders contain more than one `global.cfg`.
This may be chrono variants (seasonal or time-of-day map versions),
or a genuine multi-entity map.

**Task:**
1. Identify the three map folders with multiple `global.cfg` files.
2. For each, list all `global.cfg` paths and the immediate
   subfolders they live in.
3. Read the first 30 lines of each `global.cfg` in **one** of those
   folders.
4. Cross-reference `docs/src/formats/map/timetables.md` and
   `docs/src/formats/map/chrono.md` (if it exists) for how chrono
   variants are structured.

**Deliverable:** A subsection titled "Investigation B — Maps with
Multiple `global.cfg`" with:
- The three folders identified
- Whether the extra `global.cfg` files are chrono variants or
  distinct maps
- Schema implication: does `entities.kind = 'folder'` still hold for
  maps, or does `Maps` need `kind = 'file'` with variants?

### Investigation C — Vehicle Category Depth

**Context:** Stage 1 found `.bus` files at depths 1–2 relative to
`Vehicles/`. But `.ovh` files were found at depth 1–3. The exact
pattern determines whether the entity folder is `Vehicles/<Name>/`
or `Vehicles/<Category>/<Name>/`.

**Task:**
1. Enumerate all `.bus` files. Record the depth distribution:
   - Depth 1: `Vehicles/Foo.bus`
   - Depth 2: `Vehicles/A/Bus.bus`
   - Depth 3: `Vehicles/A/B/Bus.bus`
2. Same for `.ovh`.
3. Sample 3 folders at each depth. What is the parent folder's role
   at depth 1 vs. depth 2 vs. depth 3?

**Deliverable:** A subsection titled "Investigation C — Vehicle Depth
Pattern" with:
- Depth distribution tables for `.bus` and `.ovh`
- Sample parent folder names at each depth
- A conclusion: what is the canonical entity folder pattern?
- Schema implication: does the scanner need a depth-agnostic rule
  ("find the nearest ancestor containing the marker")?

---

## 5. Deep-Dive by Mode

After the three investigations, sample each mode systematically.

### 5.1 Folder-Entity Categories

For each of: `Vehicles`, `Maps`, `TicketPacks`, `Money`, `Humans`.

1. **Sample 3 entities.** For each, record:
   - The entity folder path (relative)
   - The marker file(s) it contains
   - The immediate files it contains (list, ≤ 20)
   - The subfolders it contains (list names only)
2. **Identify component roles.** Which files serve which purpose?
   Example for a vehicle:
   - `.bus` — definition
   - `model.cfg` — model configuration
   - `paths.cfg`, `passengercabin.cfg`, `sound.cfg` — referenced
     configs
   - `*.o3d` — meshes
   - `*.dds`, `*.bmp`, `*.tga` — textures
3. **Check consistency.** Do all 3 samples follow the same pattern,
   or do some entities have extra components?

**Deliverable:** One subsection per category titled "Deep-Dive —
<Category>" with:
- The marker file pattern
- The typical component layout
- Any exceptions or variants
- Schema implication: what roles should appear in
  `entity_references`?

### 5.2 File-Entity Categories

For each of: `Sceneryobjects`, `Splines`, `Fonts`, `Trains`,
`Situations`, `Weather`, `option_presets`, `Languages`, `Inputs`,
`GUI`, `Drivers`.

1. **Sample 5 marker files.** For each:
   - Full relative path
   - Parent folder path
   - File size
2. **Determine the folder's role.** Is the parent folder an author
   name, a pack name, a category, or something else?
3. **Estimate entity count.** How many marker files per parent
   folder on average? (Stage 1 already gave some of these.)
4. **Identify pool files.** Which non-marker files in the same
   folder serve as resources for the marker entities?

**Deliverable:** One subsection per category titled "Deep-Dive —
<Category>" with:
- The marker extension
- Typical folder depth
- Entity count (approximate)
- Pool files present (if any)
- Schema implication: is `entities.kind = 'file'` the right model?

### 5.3 Pool Categories

For each of: `Texture`, `Sounds`, `helper`, `program`, `Scripts`.

1. **Identify the reference path convention.** How do other files
   refer to these? Look for:
   - `[matl]` lines in `.cfg` files referencing `Texture\...`
   - `[sound]` lines in `.cfg` files referencing `Sounds\...`
   - `[script]` lines referencing `Scripts\...`
2. **Sample the pool structure.** Are files organized by category,
   season, or something else?
3. **Confirm nothing in the pool is itself an entity.** If a
   subfolder contains marker files, it belongs in a different mode.

**Deliverable:** One subsection per category titled "Deep-Dive —
<Category>" with:
- Reference path convention
- Internal structure
- Confirmation that no entities exist within
- Schema implication: pool files are just rows in `files`, never
  `entities`

### 5.4 Excluded Categories

Already documented in the amendment. No additional sampling needed.
Restate the excluded list in the deep-dive for completeness.

---

## 6. Cross-Mode Relationships

Identify relationships between modes. Examples to look for:

1. **Vehicle → Money.** Does a vehicle reference a money system?
   (Check `.bus` for `[moneysystem]`.)
2. **Map → Vehicles.** Does a map reference AI vehicle lists?
   (Check `global.cfg` for `[aigroup]` or `[ailist]`.)
3. **Vehicle → Sounds.** How do vehicles reference audio pools?
4. **Sceneryobject → Texture.** How do `.sco` files reference
   shared textures?
5. **Map → Splines.** How do `.map` tiles reference `.sli` files?

**Deliverable:** A section titled "Cross-Mode Relationships" with a
table:

| Source Mode | Target Mode | Reference Mechanism | Example |
| :--- | :--- | :--- | :--- |
| Folder-entity | Pool | Path string in `.cfg` | `[sound] sound\foo.cfg` |
| File-entity | Pool | Path string | `.sco` `[texture]` |
| Folder-entity | Folder-entity | Filename | `.bus` `[couple_back]` |

This is what Stage 3 (Relationship Mapping) will expand, but the
shape should be visible now.

---

## 7. Reporting Cadence

Report after each of these checkpoints:

1. Investigation A (`.ovh`) — approval before proceeding
2. Investigation B (Maps) — approval before proceeding
3. Investigation C (Vehicle depth) — approval before proceeding
4. Deep-dive folder-entity categories — approval before file-entity
5. Deep-dive file-entity categories — approval before pool
6. Deep-dive pool categories — approval before cross-mode
7. Cross-mode relationships — approval before exit

Do not proceed between checkpoints without confirmation.

Format:

```
## Stage 2 Deep-Dive — Checkpoint N

**Checkpoint:** <name>
**Findings:** <summary>
**Schema implications:** <bulleted list>
**Blockers:** <if any>

Ready to proceed? (yes / adjust)
```

---

## 8. Exit Criteria

- ✅ All three investigations (A, B, C) complete with conclusions.
- ✅ Every folder-entity category sampled with 3 entities.
- ✅ Every file-entity category sampled with 5 marker files.
- ✅ Every pool category sampled with reference convention
  identified.
- ✅ Cross-mode relationships table populated.
- ✅ The four-mode model confirmed, with the reclassifications
  documented.
- ✅ Findings appended to `docs/src/architecture/catalogue-schema.md`.
- ✅ No OMSI binary was inspected.
- ✅ No OMSI asset was committed.
- ✅ No Rust code was written.

When finished, respond with a summary containing:

- The three investigation conclusions
- The confirmed mode for each of the 35 top-level directories
- The cross-mode relationship count
- Any spec gaps discovered (like `.ovh` if it turns out to be
  undocumented)
- Recommended next step

Then respond with the exact string:
"STAGE 2 DEEP-DIVE COMPLETE — ready for Stage 3"