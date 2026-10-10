## 4a. Stage 2 Amendment — Two-Mode Category Model

Stage 1 revealed that the OMSI installation is not uniform. Some
top-level categories are **hierarchical** (deeply nested, entity
folders), and others are **flat** (single-level file pools). The
schema must model both without forcing a false symmetry.

Before proceeding to the original Stage 2, classify every top-level
directory found in Stage 1 into one of three modes:

| Mode | Description | Examples from Stage 1 |
| :--- | :--- | :--- |
| **Hierarchical** | Content is organized into entity folders at some depth. Individual items are directories. | `Vehicles`, `Sceneryobjects`, `maps`, `Splines`, `Humans`, `Money`, `TicketPacks`, `Sounds`, `Texture` |
| **Flat** | Content is a single-level pool of individual files. There are no entity folders. | `Fonts`, `Trains`, `Situations`, `Weather`, `option_presets`, `Languages`, `Inputs`, `GUI`, `Drivers`, `program`, `Scripts` |
| **Excluded** | Directory is not user content — it is tooling, staging, backup, or user artifacts. | `SDK`, `OMSI-Tools`, `DEMImport`, `plugins`, `OmniNavigation`, `_Straßenbahn`, `Temp_*`, `Screenshots`, `docs`, `Tutorials`, `Addons`, `RegAddons` |

### Deliverable for this Amendment

Produce a table with one row per top-level directory:

| Directory | Mode | Recursive Files | Rationale |
| :--- | :--- | ---: | :--- |
| `Vehicles` | Hierarchical | 246,465 | Entity folder depth 2–3 (`Vehicles/<Category>/<Name>/`); marker file `.bus` |
| `Fonts` | Flat | 2,095 | All files at depth 1; no subfolder structure |
| `SDK` | Excluded | 202 | Development tooling, not runtime content |

For **Hierarchical** directories, identify the entity folder pattern
in the same table:

| Directory | Entity Pattern | Marker File | Approx. Entity Count |
| :--- | :--- | :--- | ---: |
| `Vehicles` | `Vehicles/<Category>/<Name>/` | `*.bus` | 2,000+ |
| `Maps` | `Maps/<Name>/` | `global.cfg` | ~200 |

For **Flat** directories, no entity pattern is needed — the whole
directory is one pool, and each file is a first-class object.

For **Excluded** directories, record the reason. Do not sample their
contents beyond the file-counting already done in Stage 1.

### Rationale for the Three Modes

1. **Hierarchical categories** produce `entities` rows (a vehicle, a
   map, a scenery object) with `entity_files` rows linking component
   files to their roles.
2. **Flat categories** produce only `files` rows. There is no entity
   to link them to — they are the entity.
3. **Excluded categories** produce neither. The scanner skips them at
   walk time.

A schema that treats `Fonts` and `Vehicles` uniformly would either
(a) invent entity folders where none exist, or (b) fragment vehicles
into loose files. Both are wrong. The two-mode model (plus exclusion)
matches reality.

### Safety Reminder

The direct root files include executables (`Omsi.exe`,
`Launcher.exe`) and DLLs (`d3dx9.dll`, `qtintf.dll`, `steam_api.dll`).
During Stage 2 and Stage 3, **do not sample any file** with extension
`.exe`, `.dll`, `.so`, `.pdb`, `.lib`, or `.obj`. If a directory
contains primarily such files (e.g., `OMSI-Tools`), classify it as
Excluded and move on.

### Edge Cases to Document During This Stage

The Stage 1 survey surfaced several categories that don't fit the
two-mode model cleanly. Address each explicitly:

1. **Leading-underscore folders** (`_Straßenbahn`) — mod staging or
   real content? Look for a `.bus`, `.sco`, or `global.cfg` inside.
   If none, exclude.

2. **`Temp_*` folders** — staging artifacts. Exclude unless they
   contain entity markers.

3. **Root-level shell scripts** (`hof_organizer.sh`,
   `bash_script.sh`) — user artifacts. The scanner should have a
   rule for skipping files that are neither OMSI content nor
   referenced by content. Document the rule.

4. **Empty directories** (`Screenshots`) — insert the directory row
   so the schema remains complete, but skip the file walk. Do not
   treat as an error.

5. **Mutable root files** (`logfile.txt`, `laststn.osn`,
   `options.cfg`, `envir.cfg`) — these change frequently. Document
   which should be:
   - **Catalogued but not tracked for staleness** (most)
   - **Tracked normally** (rare)
   - **Excluded entirely** (logfile.txt likely)

6. **Tool directories** (`SDK`, `OMSI-Tools`, `DEMImport`,
   `plugins`, `OmniNavigation`) — these are not content. The schema
   should not catalogue them. If the application later needs to
   query tool inventory, that's a separate concern.

### When to Proceed to Original Stage 2

Once this amendment's deliverable is complete (classification table
+ entity pattern table + edge cases), proceed to the original Stage 2
with the modes already known. The original Stage 2 (Category
Identification) now becomes a deeper dive into each Hierarchical
category, rather than a first-pass classification.

### Reporting Cadence

Report the amendment's deliverable separately from the original
Stage 2. The user should approve the mode classification before the
entity pattern investigation begins, since the mode determines what
the investigation looks for.