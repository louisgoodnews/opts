# 🔗 Stage 3 — Relationship Mapping & `.ovh` Closeout

Stage 2's deep-dive identified `.bus` and `.ovh` as co-equal markers
for the `vehicle` category, but four follow-up questions remained
unanswered. This stage closes those, then maps the cross-mode
relationships that the schema's `entity_references` table must
capture.

This is a RESEARCH task. You are not writing Rust code. You are not
drafting the schema yet. You are producing the reference graph that
Stage 7 (Schema Proposal) will build on.

---

## 1. Configuration

### OMSI installation path
/home/louisgoodnews/nvme2n1/steamapps/common/OMSI 2

If this path does not exist or is inaccessible, STOP and report.

### OPTS repository root
The current working directory. Confirm by checking for a `Cargo.toml`
with `[workspace]` and a `docs/src/SUMMARY.md`.

### Output target
Append to `docs/src/architecture/catalogue-schema.md` under a new
section titled `## 🔗 Stage 3 — Relationship Findings`.

### Reference documents
- `docs/src/architecture/catalogue-schema.md` (read the Stage 2
  findings first)
- `docs/src/formats/text/misc-types.md` (`.ovh` spec)
- `docs/src/formats/text/vehicles/bus.md` (`.bus` spec)
- `docs/src/formats/map/lists.md` (AI list format)
- `docs/src/formats/text/trains.md` or equivalent (`.zug` format)
- `docs/src/formats/text/vehicles/predefined-variables.md` (for
  callback and trailer semantics)

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
- Do not promote any spec from 🟡 Draft to 🟢 Specified without
  evidence.
- Do not modify Phase 1 or Phase 2 milestones.

### ✅ Permitted

- Walk directories, count files, sample directory names.
- Read text files (`.cfg`, `.txt`, `.bus`, `.ovh`, `.sco`, `.sli`,
  `.otp`, `.zug`, `.hum`, `.osn`, `.olf`, `.oop`, `.cti`, `.org`,
  `.ailist`, `.ttl`, `.ttr`, `.ttp`).
- Cross-reference existing format specs.
- Append findings to `docs/src/architecture/catalogue-schema.md`.

---

## 3. Preconditions

Verify and report before starting:

1. Stage 2 deep-dive findings are present in
   `docs/src/architecture/catalogue-schema.md`.
2. Stage 2 concluded `.bus` and `.ovh` are co-equal vehicle markers.
3. The four follow-up questions from the Stage 2 review are still
   open (they are the first work of this stage).
4. `docs/src/formats/map/lists.md` exists.
5. A spec covering `.zug` exists (check `docs/src/formats/text/`).

If any precondition fails, STOP and report.

---

## 4. Part A — `.ovh` Closeout

Four specific investigations to close the review's open questions.

### Investigation A1 — `[type]` Enumeration

**Context:** Two `.ovh` samples showed `[type] 2` and one `[type] 3`.
The schema needs to know what `[type]` means to support queries like
"list all trams" or "list all AI cars."

**Task:**

1. Walk all `.bus` and `.ovh` files in `Vehicles/`.
2. Extract the `[type]` value from each (the line immediately after
   `[type]`).
3. Build a frequency table:

   | `[type]` value | `.bus` count | `.ovh` count | Sample folders |
   | :--- | ---: | ---: | :--- |

4. For each distinct `[type]` value, sample 3 folders and note their
   vehicle category (bus, tram, train, car, aircraft, trailer,
   subway, freight, other).
5. Cross-reference with `docs/src/formats/text/vehicles/bus.md` for
   any documented meaning of `[type]`.

**Deliverable:** A subsection titled "Investigation A1 — `[type]`
Enumeration" with:
- The frequency table
- The category correlation table
- A conclusion: what does each `[type]` value mean?
- Schema implication: is this a column (`vehicle_class`) or a tag
  (`entity_tags: [tram]`)?

### Investigation A2 — Alias Hypothesis Verification

**Context:** Stage 2 hypothesized that `.bus`/`.ovh` twins are
"compatibility aliases" so that map AI lists and train consists
resolve regardless of which extension they reference. This was
plausible but not verified.

**Task:**

1. Find all `.ailist` files under `maps/`.
2. Grep for `.ovh` and `.bus` references. Count references by
   extension.
3. Find all `.zug` files under `Vehicles/` and `Trains/`.
4. Grep for `.ovh` and `.bus` references. Count references by
   extension.
5. For folders where both a `.bus` and an `.ovh` exist (the 24
   twin folders from Stage 2), check whether any AI list or `.zug`
   file references both extensions for the same vehicle.
6. Look for any case where an AI list references a vehicle that
   *only* exists as `.bus`, and any case where it references a
   vehicle that *only* exists as `.ovh`.

**Deliverable:** A subsection titled "Investigation A2 — Alias
Hypothesis" with:
- Count of `.bus` vs. `.ovh` references in AI lists
- Count of `.bus` vs. `.ovh` references in `.zug` files
- Whether twins are referenced by both extensions or just one
- A conclusion: is the alias hypothesis confirmed, refuted, or
  still open?
- Schema implication: does the catalogue need to expose both
  extensions for the same entity, or does it store one primary
  marker and one alias?

### Investigation A3 — Trailer Composition

**Context:** The `misc-types.md` spec names **trailers** as an
`.ovh` use case, but Stage 2 didn't investigate trailers. This
matters because a drivable articulated bus has a tractor segment
and one or more trailer segments — are they one entity or many?

**Task:**

1. Find all `.bus` and `.ovh` files containing `[couple_back]` or
   `[couple_front]` (search by keyword, not filename).
2. For each, extract the filename referenced by `[couple_back]`.
3. Determine whether trailer references point to `.bus` or `.ovh`
   files.
4. Sample 3 articulated vehicles. For each, record:
   - The tractor file path
   - The trailer file path(s)
   - Whether the tractor and trailer share a folder or live in
     separate folders
5. Check whether trailer files have their own `[couple_back]` (for
   double-articulated vehicles).

**Deliverable:** A subsection titled "Investigation A3 — Trailer
Composition" with:
- Count of `[couple_back]` references by target extension
- Whether trailers live in the same folder as tractors
- Whether double-articulated vehicles exist in the corpus
- A conclusion: is a multi-segment vehicle one entity or many?
- Schema implication: does the schema need an `entity_composition`
  table, or is `entity_references` with `role = 'trailer'`
  sufficient?

### Investigation A4 — Spec Reconciliation

**Context:** The Stage 2 finding contradicts `misc-types.md`'s
framing of `.ovh` as a *secondary* format. This drift must be
resolved.

**Task:**

1. Re-read `docs/src/formats/text/misc-types.md`, specifically the
   `.ovh` section.
2. Compare the spec's description against Stage 2's findings.
3. Produce a diff of the discrepancies:

   | Spec claim | Investigation finding | Action |
   | :--- | :--- | :--- |

4. For each discrepancy, decide:
   - **Update the spec** — evidence is conclusive, spec was wrong.
   - **Flag for review** — evidence is suggestive but not
     conclusive.
   - **Leave as-is** — spec was correct, investigation was
     misinterpreted.

**Deliverable:** A subsection titled "Investigation A4 — Spec
Reconciliation" with:
- The discrepancy table
- The decision for each
- If any spec update is warranted, produce a proposed edit as a
  unified diff (do NOT apply it — the user must approve)

---

## 5. Part B — Cross-Mode Relationship Mapping

Once Part A is closed, map the reference graph that the schema's
`entity_references` table must capture.

### Stage B1 — Enumerate Reference Types

For each of these source → target pairs, determine whether a
reference exists, what form it takes, and its cardinality:

| Source | Target | Cardinality |
| :--- | :--- | :--- |
| Vehicle | Pool (Sounds) | 1 → 1 |
| Vehicle | Pool (Texture) | 1 → N |
| Vehicle | File-entity (Fonts) | 1 → N |
| Vehicle | File-entity (Money) | 1 → 1 |
| Vehicle | Folder-entity (Vehicle) | 1 → N (trailers) |
| Map | Folder-entity (Vehicle) | 1 → N (AI lists) |
| Map | Pool (Sounds) | 1 → N |
| Map | Pool (Texture) | 1 → N |
| Map | File-entity (Splines) | 1 → N |
| Map | File-entity (Weather) | 1 → 1 |
| Sceneryobject | Pool (Texture) | 1 → N |
| Sceneryobject | Pool (Sounds) | 1 → N |
| Spline | Pool (Texture) | 1 → N |
| Human | Pool (Texture) | 1 → N |

For each row, either confirm or refute with evidence from a sample
file.

### Stage B2 — Reference Syntax

For each confirmed reference type, record the syntax used to express
it. Examples:

- **Path string in a keyword line:** `[sound] sound\foo.cfg`
- **Bare filename in a keyword line:** `[friendlyname] MAN`
- **Path string in a `.ailist` file:** `Vehicles\Citr_BX\BX.ovh`
- **Relative path with backslashes:** `Texture\Asphalt\road1.bmp`

Note the platform conventions. OMSI is Windows-native; paths use
backslashes. The scanner must normalize these consistently.

### Stage B3 — Reference Cardinality

For each reference type, determine:

- Is it always 1 → 1?
- Can it be 1 → N? How is N expressed?
- Are there optional references (0 → 1)?
- Are there cycles? (Vehicle A references Vehicle B, which
  references Vehicle A)

### Stage B4 — Relationship Table

Produce the consolidated table:

| Source Mode | Source Category | Target Mode | Target Category | Syntax | Cardinality | Optional? |
| :--- | :--- | :--- | :--- | :--- | :--- | :--- |
| Folder-entity | vehicle | Pool | sounds | `[sound]` path | 1 → 1 | No |
| Folder-entity | vehicle | Folder-entity | vehicle | `[couple_back]` filename | 1 → 1 | Yes |
| Folder-entity | map | Folder-entity | vehicle | `.ailist` path string | 1 → N | No |
| ... | ... | ... | ... | ... | ... | ... |

---

## 6. Part C — Schema Implications

Based on Parts A and B, produce a summary of what the schema must
support:

1. **Entity identity.** How many `category` values are there? What
   are they?
2. **Marker extensions.** Does each `category` have exactly one
   marker extension, or can categories have multiple (like `vehicle`
   with `.bus` and `.ovh`)?
3. **Composition.** Does the schema need explicit composition
   (trailer relationships), or can they be expressed as references?
4. **Reference storage.** Should `entity_references` have a fixed
   schema (source_id, target_id, role) or a flexible one
   (source_id, target_kind, target_id, metadata)?
5. **Pool indexing.** Pools have no entities. How does the schema
   support "find the texture file X referenced by entity Y"?
6. **Reverse lookups.** Does the application need "which entities
   reference file X?" If so, what index supports it?

For each item, state the requirement and the evidence that supports
it.

---

## 7. Reporting Cadence

Report after each of these checkpoints:

1. Investigation A1 (`[type]` enumeration) — approval before A2
2. Investigation A2 (alias verification) — approval before A3
3. Investigation A3 (trailer composition) — approval before A4
4. Investigation A4 (spec reconciliation) — approval before Part B
5. Part B relationship table — approval before Part C
6. Part C schema implications — approval before exit

Do not proceed between checkpoints without confirmation.

Format:

```
## Stage 3 — Checkpoint N

**Checkpoint:** <name>
**Findings:** <summary>
**Schema implications:** <bulleted list>
**Blockers:** <if any>

Ready to proceed? (yes / adjust)
```

---

## 8. Exit Criteria

- ✅ All four `.ovh` investigations (A1–A4) complete with
  conclusions.
- ✅ Cross-mode relationship table populated with syntax,
  cardinality, and optionality.
- ✅ Schema implications summarized with evidence for each.
- ✅ Any spec updates proposed (not applied) as unified diffs.
- ✅ Findings appended to
  `docs/src/architecture/catalogue-schema.md`.
- ✅ No OMSI binary was inspected.
- ✅ No OMSI asset was committed.
- ✅ No Rust code was written.

When finished, respond with a summary containing:

- The conclusions of A1–A4
- The number of reference types identified
- The number of schema implications
- Any spec updates proposed
- Any new open questions
- Recommended next step

Then respond with the exact string:
"STAGE 3 COMPLETE — ready for Stage 4"