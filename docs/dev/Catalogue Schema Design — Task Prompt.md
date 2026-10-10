# 🗄️ Catalogue Schema Design

You are designing the SQLite schema for the OPTS path catalogue. The
catalogue caches a discovered inventory of an OMSI 2 installation so
that the application can query "what's installed?" without walking
600k+ files on every launch.

This is a RESEARCH AND DESIGN task. You are not writing Rust code.
You are not creating the database. You are not writing the scanner.
You are producing a **schema proposal** that a future implementer will
build against.

Your proposal must be grounded in actual evidence from the OMSI
installation — not in assumptions about how OMSI is organized.

---

## 1. Configuration

### OMSI installation path
/home/louisgoodnews/nvme2n1/steamapps/common/OMSI 2

If this path does not exist or is inaccessible, STOP and report.

### OPTS repository root
The current working directory. Confirm by checking for a `Cargo.toml`
with `[workspace]` and a `docs/src/SUMMARY.md`.

### Output file
docs/src/architecture/catalogue-schema.md (new file)

If this file already exists, STOP and report. Do not overwrite.

### Related context (read before starting)
- docs/src/adr/0002-clean-room-policy.md
- docs/src/formats/inventory.md (the P-100 format inventory)
- docs/src/contributing/standards.md

---

## 2. Legal and Safety Guardrails

### ❌ Forbidden

- Do not disassemble, decompile, or inspect `OMSI.exe` or any DLL.
- Do not run any binary analysis tool.
- Do not modify any file in the OMSI installation. Read-only.
- Do not copy any file from the OMSI installation into the OPTS
  repository, not even as a test fixture.
- Do not quote more than 20 consecutive lines from any OMSI file.
- Do not create or commit a `.db` file.
- Do not write Rust code.
- Do not modify any existing spec, ADR, or milestone.

### ✅ Permitted

- Walk the OMSI directory tree recursively (read-only).
- Enumerate, count, size, and categorize files and directories.
- Sample file contents (headers, keyword lines, filenames) to
  understand what each entity type looks like on disk.
- Read the existing specs in `docs/src/formats/` to cross-reference
  what OPTS already knows about these files.
- Produce a schema proposal document.

---

## 3. Design Constraints

These constraints were agreed in prior design discussions. Your
schema must satisfy them. If your investigation reveals a constraint
cannot be satisfied, STOP and report.

### 3.1 Stable Schema

The set of tables must be **fixed**. It must not grow as mods are
installed. Adding a new vehicle pack must add *rows*, not *tables*.

### 3.2 Views for Ergonomics

Queries like "all drivable vehicles" or "vehicles with their
passengercabin" should be expressible as simple `SELECT`s against
views, without the caller needing to write joins.

### 3.3 Staleness Detection

The schema must support detecting when the catalogue is out of date
relative to the OMSI installation, at the granularity of individual
directories and files.

### 3.4 Schema Versioning

The schema must record its own version. A future migration path must
be possible without discarding user catalogues.

### 3.5 Multi-Root Forward Compatibility

The v1 application will support a single OMSI root. The schema must
not require alteration to support multiple roots in v2.

### 3.6 No Content Parsing

Discovery catalogues paths, sizes, timestamps, and types. It does not
parse `.bus` files or extract friendly names. Entity *naming* comes
from folder structure, not from file contents.

---

## 4. Methodology

Follow these stages in order. Report progress after each one. Do not
skip stages.

### Stage 1 — Top-Level Survey

1. Enumerate every direct child of the OMSI installation root.
2. Classify each as: directory / file / symlink / other.
3. For directories, record:
   - Name
   - Immediate child count
   - Recursive file count (estimate if > 100k)
   - Recursive total size
4. Produce a table sorted by recursive file count, descending.

This is the foundation. Every subsequent stage builds on it.

### Stage 2 — Category Identification

For each top-level directory, determine what kind of content it
contains by sampling:

1. List its immediate subdirectories.
2. Sample 2–3 representative subdirectories.
3. Identify the "entity folder" pattern — the depth at which an
   individually-meaningful item begins.
   - For `Vehicles/`, an entity is likely `Vehicles/<Category>/<Name>/`.
   - For `Maps/`, an entity is likely `Maps/<Name>/`.
   - For `Sceneryobjects/`, an entity is likely `Sceneryobjects/<Author>/<Name>/`.
   - Some may be flatter or deeper. **Record what you actually find.**

4. For each top-level category, document:
   - The entity-folder pattern (as a glob or regex)
   - The approximate entity count
   - The files that mark a folder as an entity (e.g., a `.bus` file
     for vehicles, a `global.cfg` for maps)

This stage determines what "entity" means for each category.

### Stage 3 — Relationship Mapping

For each category, identify the **component files** that belong to
an entity.

Example for a vehicle entity:

- `.bus` file (the definition)
- `model.cfg` (the model configuration)
- `paths.cfg`, `passengercabin.cfg`, `sound.cfg` (referenced from `.bus`)
- Multiple `.o3d` files (the meshes)
- Multiple `.dds`/`.bmp`/`.tga` files (the textures)

Document the relationship as:

| Category | Entity Marker | Component Roles | Component File Kinds |
| :--- | :--- | :--- | :--- |
| Vehicle | `*.bus` | definition, model, paths, cabin, sound, mesh, texture | `.bus`, `.cfg`, `.o3d`, `.dds`, ... |
| Map | `global.cfg` | definition, tiles, lists, timetables | `.cfg`, `.map`, `.ttl`, ... |
| Sound pack | ? | ? | ? |

Where you cannot determine the relationship from filenames alone,
sample file contents to confirm.

### Stage 4 — Edge Cases

Enumerate structures that don't fit the simple entity pattern:

- **Backup folders** — `_Backup`, `_backup`, `*.bak`, timestamped
  copies. Should they be catalogued? Likely not.
- **Duplicate author folders** — multiple top-level folders that
  represent the same logical author.
- **Deeply nested mods** — some mods nest their content inside
  intermediate folders.
- **Non-entity folders** — folders that exist for organization but
  don't represent an entity, e.g., `Vehicles/AI/` might just be a
  grouping folder.
- **Non-standard extensions** — files with extensions not in the
  P-100 inventory.
- **Empty entity folders** — folders that look like entities but
  lack their marker file.
- **Symlinks** — if any exist, how should discovery handle them?

Document each edge case with a concrete example from the actual
installation.

### Stage 5 — Metadata Selection

Determine what metadata each row should store. Consider:

- **For directories:** name, relative path, mtime, child count
- **For files:** name, relative path, size, mtime, extension
- **For entities:** category (from Stage 2), name (from folder),
  entity folder path
- **For the catalogue itself:** scan start/end time, file count,
  schema version, OMSI root path

Justify each field. Do not add fields speculatively — the schema
must serve the queries we've described, and nothing more.

### Stage 6 — Query Survey

Enumerate the queries the catalogue must answer. These come from
the application's needs. Start with these and add any you identify:

1. "List all drivable vehicles."
2. "List all AI-only vehicles."
3. "Given a vehicle, list its component files by role."
4. "Find a file by relative path."
5. "Find all files with a given extension under a given category."
6. "Determine if the catalogue is stale relative to the
   installation."
7. "List all entities in a given category."

For each query, sketch how it would be expressed in SQL. If any
query requires complex joins or subqueries, that's a signal the
schema needs a view.

### Stage 7 — Schema Proposal

Now write the actual schema. Follow this structure:

1. **Table definitions** — `CREATE TABLE` statements with types,
   constraints, and foreign keys. Include comments explaining each
   column.
2. **Index definitions** — `CREATE INDEX` statements for each query
   in Stage 6 that benefits from one. Justify each index.
3. **View definitions** — `CREATE VIEW` statements providing the
   ergonomics of Stage 3.2. One view per "entity category" the
   application needs to query.
4. **Staleness algorithm** — pseudocode describing how the
   application detects a stale catalogue, using the columns your
   schema defines.
5. **Versioning strategy** — how schema migrations will work.
6. **Multi-root plan** — how a v2 release could add a second root
   without altering existing tables.

### Stage 8 — Sample Queries

Write 5–10 illustrative `SELECT` statements that exercise the
schema. Each should:

- Be a realistic query the application would run.
- Include a comment explaining its purpose.
- Return a well-formed result.

These are what a reviewer will read to understand whether the schema
is usable in practice.

### Stage 9 — Open Questions

List every question the investigation could not resolve. For each:

- A one-line statement of the question.
- Why it matters.
- What evidence would resolve it.

Do not guess at answers.

---

## 5. Output Format

The output file `docs/src/architecture/catalogue-schema.md` must
contain these sections:

```markdown
# 🗄️ Path Catalogue Schema

| Metadata | Value |
| :--- | :--- |
| **Status** | 🟡 Proposal — awaiting review |
| **Schema Version** | 1 (proposed) |
| **Storage** | SQLite (rusqlite, WAL mode) |
| **Last Updated** | YYYY-MM-DD |

## 📖 Overview
<What the catalogue is for, in one paragraph.>

## 🔍 Investigation Findings

### Top-Level Structure
<Table from Stage 1.>

### Entity Patterns by Category
<Table from Stage 2.>

### Component Relationships
<Table from Stage 3.>

### Edge Cases
<Bulleted list from Stage 4.>

## 🗄️ Schema

### Tables
<CREATE TABLE statements from Stage 7.1.>

### Indexes
<CREATE INDEX statements from Stage 7.2.>

### Views
<CREATE VIEW statements from Stage 7.3.>

## 🔄 Staleness Algorithm
<Pseudocode from Stage 7.4.>

## 📈 Versioning Strategy
<Prose from Stage 7.5.>

## 🌐 Multi-Root Plan
<Prose from Stage 7.6.>

## 🧪 Sample Queries
<SELECT statements from Stage 8.>

## ❓ Open Questions
<Table from Stage 9.>

## 📝 Provenance

- Files inspected: <list>
- Directories walked: <count>
- Techniques: <list>

**Clean-room attestation (ADR-0002):**
- No OMSI binaries were inspected.
- No GPL source code was read.
- No OMSI assets were committed.

## 🔗 Related Documents
- [Format Inventory](../formats/inventory.md)
- [Standards](../contributing/standards.md)
- [ADR-0002: Clean-Room Policy](../adr/0002-clean-room-policy.md)
```

---

## 6. Reporting Cadence

After each stage, pause and report:

```
## Schema Design Progress — Stage N

**Stage:** <name>
**Findings:** <summary>
**Surprises:** <anything unexpected>
**Blockers:** <if any>

Ready to proceed? (yes / adjust)
```

Do not proceed without confirmation. Schema design is easy to get
wrong early and expensive to fix later.

---

## 7. Constraints on the Proposal

The proposal must satisfy these non-negotiable properties:

1. **No table per folder.** The number of tables is fixed regardless
   of how many folders exist in the OMSI installation.
2. **No dynamic SQL.** The application must not generate `CREATE TABLE`
   statements at runtime.
3. **No file contents in the catalogue.** Only paths, sizes,
   timestamps, and extensions. Extracted metadata (friendly name,
   manufacturer, etc.) is future work.
4. **Every query in Stage 6 has a plan.** Even if it's "not
   supported by v1", the proposal must document why.

---

## 8. Exit Criteria

- ✅ All 9 stages completed.
- ✅ `docs/src/architecture/catalogue-schema.md` exists.
- ✅ The schema satisfies all constraints in Section 3.
- ✅ Every claim about OMSI's structure cites a sampled directory
  or file.
- ✅ Every query in Stage 6 has a sketch.
- ✅ Every index and view has a justification.
- ✅ Open Questions are tracked.
- ✅ No OMSI binary was inspected.
- ✅ No OMSI asset was committed.
- ✅ No Rust code was written.

When finished, respond with a summary containing:

- Top-level categories identified
- Number of entities per category (approximate)
- Number of tables, indexes, and views proposed
- Any constraints that could not be satisfied
- Recommended next step

Then respond with the exact string:
"SCHEMA PROPOSAL COMPLETE — ready for review"