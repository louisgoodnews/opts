# 🧪 Second Mutation Kit — Design & Execution

You are designing and running the second mutation-testing kit for the
OPTS project. The first kit resolved all high-priority questions in the
binary mesh formats (`.o3d`, `.x`), promoting both specs to 🟢
Specified. This second kit covers the remaining format families still
at 🟡 Draft: map tiles, textures, timetables, and `.sco` keywords.

This is a **two-phase task**. Phase A designs the kit. Phase B runs it.
You must complete Phase A and get user approval before starting Phase B.

You are NOT writing Rust code. You are NOT re-analyzing formats from
scratch. You are designing minimal, procedural, clean-room test
fixtures and running them in OMSI.

---

## 1. Configuration

### OMSI installation path
/home/louisgoodnews/nvme2n1/steamapps/common/OMSI 2

If this path does not exist or is inaccessible, STOP and report.

### OPTS repository root
The current working directory. Confirm by checking for a `Cargo.toml`
with `[workspace]` and a `docs/src/SUMMARY.md`.

### First kit location (for reference)
/home/louisgoodnews/Dokumente/opts-mutation-kit/

Read the first kit's `generate.py` and `CHECKLIST.md` to match its
structure, naming conventions, and procedural generation style. Do
not modify the first kit.

### Second kit location
/home/louisgoodnews/Dokumente/opts-mutation-kit-2/

Create this directory. All kit scripts, fixtures, and checklists go
here. Nothing in this directory gets committed to the OPTS repository.

### Scratch directory for observations
/tmp/opts-mutation-run-2/
├── logs/
├── screenshots/
└── notes/

### Specs to update
docs/src/formats/map/tiles.md
docs/src/formats/map/timetables.md
docs/src/formats/textures.md
docs/src/formats/text/sco.md

---

## 2. Legal and Safety Guardrails

Identical to the first kit's rules. Restated here for redundancy.

### ❌ Forbidden

- Do not disassemble, decompile, or inspect `OMSI.exe` or any DLL.
- Do not run Ghidra, IDA Pro, dnSpy, x64dbg, Cheat Engine, or any
  binary analysis tool.
- Do not modify any file in the OMSI installation directory.
  Mutated files go into a scratch or test map directory only.
- Do not commit any `.o3d`, `.x`, `.map`, `.sco`, `.dds`, `.bmp`,
  `.ttl`, or any other OMSI asset to the OPTS repository.
- Do not copy GPL source code into the specs.
- Do not distribute kit-generated fixtures outside the kit directory.
- Do not promote a spec to 🟢 if any high-priority question remains
  unresolved. Honest 🟡 beats a false 🟢.

### ✅ Permitted

- Design and run procedural, clean-room mutation fixtures.
- Load mutated files in OMSI using a dedicated test map.
- Load mutated files in black-box viewers where applicable.
- Record observations in `/tmp/opts-mutation-run-2/`.
- Update the affected spec files with resolved question entries.
- Update the Open Questions / Resolved Questions tables in the specs.
- Update the phase-1 README's status column.
- Update `dev/BOARD.md` to reflect milestone progress.

---

## 3. Preconditions

Before starting, verify and report:

1. `docs/src/formats/binary/o3d.md` is 🟢 Specified.
2. `docs/src/formats/binary/x.md` is 🟢 Specified.
3. `docs/src/architecture/coordinates.md` exists with the OMSI ↔ Bevy
   transformation documented.
4. The first kit at `/home/louisgoodnews/Dokumente/opts-mutation-kit/`
   exists and is unmodified.
5. `docs/src/formats/map/tiles.md` contains Q-M01, Q-M02, Q-M03.
6. `docs/src/formats/textures.md` contains Q-TX01, Q-TX03, Q-TX07.
7. `docs/src/formats/map/timetables.md` contains Q-T01, Q-T02.
8. `docs/src/formats/text/sco.md` contains Q-AUD-005 (or Q-G01).

If any precondition fails, STOP and report the specific failure.
Do not attempt to work around missing preconditions.

---

## 4. Question Inventory

The second kit must resolve the following questions. Confirm this list
against the current Open Questions tables in each spec before
designing the kit, and report any discrepancies.

### `docs/src/formats/map/tiles.md`

- **Q-M01** — Tile origin convention: where is `tile_0_0.map` located
  in world space? Which corner of a tile is the origin? Are tile
  coordinates 1-based, 0-based, or signed?
- **Q-M02** — `[spline]` record field meanings: which fields are
  position, direction, profile index, texture assignment?
- **Q-M03** — `[object]` record field meanings: which fields are
  position, rotation, scale, object reference?

### `docs/src/formats/textures.md`

- **Q-TX01** — Texture name resolution: if a `.dds` is referenced but
  missing, does OMSI fall back to `.bmp`, `.tga`, `.png`, `.jpg`?
  In what order? Is the fallback case-sensitive?
- **Q-TX03** — Alpha blend mode semantics: how do `.bmp` alpha
  channels, `.dds` DXT1 vs. DXT5 alpha, and `.tga` 32-bit alpha differ
  in their effect? Does the mesh's `[matl_alpha]` value override?
- **Q-TX07** — UV wrapping mode: default repeat vs. clamp. How is
  `[matl_texadress_clamp]` interpreted? Does it apply per-material or
  per-mesh?

### `docs/src/formats/map/timetables.md`

- **Q-T01** — Timetable profile (`.ttl`) semantics: what does a
  "profile" mean in the timetable structure? How do entries relate to
  trips?
- **Q-T02** — Trip profile (`.ttp`) semantics: how are trip
  assignments distinguished from timetable profiles? What does the
  `.ttr` variant add?

### `docs/src/formats/text/sco.md`

- **Q-AUD-005** — Misspelled collision keywords: if a `.sco` file uses
  a documented misspelling (e.g., `collision_mesh` vs. `collsion_mesh`),
  does OMSI accept it, ignore it, or fail to load?
- **Q-G01** — `[groundtex]` number meanings: what do the integer
  parameters select? Surface type? Texture bank? Priority?

---

## 5. Phase A — Kit Design

This phase produces the kit. Do not run anything in OMSI during
Phase A.

### Stage A1 — Kit Skeleton

Create the kit directory with these files:

```
opts-mutation-kit-2/
├── README.md             # kit purpose, scope, how to run
├── CHECKLIST.md          # test-by-test execution checklist
├── generate.py           # procedural fixture generator
├── deploy.py             # copies fixtures into the test map
├── fixtures/             # generated fixtures (output dir)
│   ├── map/
│   ├── texture/
│   ├── timetable/
│   └── sco/
└── template_map/         # minimal test map skeleton
```

Do NOT reuse files from the first kit. Design this kit
independently.

### Stage A2 — Test Map Design

1. Design a minimal test map for this kit. Requirements:
   - Uses only self-authored scenery objects (no OMSI assets).
   - Has one clearly-labeled region per test group.
   - Is small enough to load in OMSI in under 30 seconds.
   - Is documented in `template_map/README.md`.

2. The map should be regenerable from `generate.py` — do not check
   in a pre-built map binary. The generator produces the map.

3. For texture tests, the map needs a flat quad facing the camera
   with a UV-mapped test pattern. The pattern encodes which quadrant
   is which (like the first kit's `uvtest.bmp`, but re-authored).

4. For `.sco` tests, the map needs labeled scenery objects using
   each variant. Collision testing requires walkable geometry.

### Stage A3 — Fixture Design

For each question in Section 4, design a minimal fixture:

1. **Q-M01 (tile origin):** Two adjacent tiles with a single object
   placed at a known relative position. Load in OMSI and observe
   where the object lands in world coordinates.

2. **Q-M02 (spline fields):** A single spline between two known
   points, with one field mutated at a time. Observe the spline's
   rendered shape.

3. **Q-M03 (object fields):** A single object placement, with one
   field mutated at a time. Observe the object's rendered transform.

4. **Q-TX01 (name fallback):** A mesh referencing a texture that
   exists only as `.bmp`, then only as `.tga`, then only as `.png`,
   then only as `.jpg`. Observe which loads.

5. **Q-TX03 (alpha blend):** Four quads, each with a different
   format of alpha (`.bmp` indexed alpha, `.dds` DXT1, `.dds` DXT5,
   `.tga` 32-bit). Compare rendering side by side.

6. **Q-TX07 (UV wrap):** A quad with UV coordinates outside `[0,1]`,
   first with default addressing, then with `[matl_texadress_clamp]`.
   Observe whether the texture repeats or clamps.

7. **Q-T01/T02 (timetable profiles):** A minimal `.ttl` and `.ttp`
   with distinct trip assignments, loaded with the IBIS system.
   Observe which trips appear in the bus's onboard computer.

8. **Q-AUD-005 (misspelled keywords):** A `.sco` file using a
   deliberately misspelled collision keyword. Load and check whether
   the object collides as if spelled correctly.

9. **Q-G01 (`[groundtex]`):** A mesh with several `[groundtex]`
   entries using different integers. Load in a scenario with rain
   or wet ground, observe which texture appears.

### Stage A4 — Procedural Generation

Extend `generate.py` to produce all fixtures from code. Requirements:

- All fixtures are self-authored binary or text files.
- No OMSI assets are referenced or copied.
- Regeneration is deterministic (seeded RNG where needed).
- Each fixture has a comment header explaining what it tests.

### Stage A5 — Checklist

Write `CHECKLIST.md` following the first kit's format. Each entry
must include:

- Test ID (T-NNN)
- Spec and question(s) it addresses
- Fixture filename
- Deploy location
- Observation protocol (what to look for)
- Expected outcomes (multiple-choice where possible)
- A blank "Actual observation" field

### Stage A6 — Report and Pause

Produce a "Kit Design Report" containing:

- Number of fixtures designed per format family
- Total questions covered
- Any questions in Section 4 not covered by the kit
- Any questions the kit covers that are not in Section 4
- Estimated run time for Phase B
- Any design decisions worth flagging

Then respond with:
"KIT 2 DESIGN COMPLETE — awaiting user approval before Phase B"

**Do not proceed to Phase B without explicit user approval.**

---

## 6. Phase B — Kit Execution

Run only after the user approves Phase A's design.

### Stage B1 — Deploy Fixtures

1. Copy the test map from `template_map/` into the OMSI installation
   (a copy, not the original).
2. Run `deploy.py` to place fixture files into the correct OMSI
   directories.
3. Launch OMSI and load the test map manually.
4. Confirm all fixtures appear as expected before starting tests.

### Stage B2 — Run Tests

For each test in `CHECKLIST.md`:

1. Apply the specified mutation or load the specified fixture.
2. Observe the outcome in OMSI.
3. Record in `/tmp/opts-mutation-run-2/logs/T-NNN.json`:
   - Test ID
   - Spec and question(s)
   - Fixture used
   - Observation (free text)
   - Conclusion (hypothesis resolved / partially resolved / open)
4. Update `CHECKLIST.md` with the actual observation.

### Stage B3 — Synthesize per Format

After all tests in a format family are complete, synthesize:

1. For each question in that family, state:
   - Resolved (with evidence test IDs)
   - Partially resolved (with what remains)
   - Still open (with blocker)
2. Update the spec's Open Questions table:
   - Move resolved questions to a new "Resolved Questions" section.
   - Update partially resolved questions with findings.
3. Update the spec's Provenance section with:
   - A "Mutation Test Evidence" subsection
   - Kit version, test dates, test count
4. If all high-priority questions are resolved:
   - Update the spec's Status from 🟡 Draft to 🟢 Specified.
   - Update the phase-1 README's status column.

### Stage B4 — Update Tracking

1. Update `docs/src/roadmap/phase-1/M-0002.md` or create a new
   milestone file for the second mutation pass (M-0013 by default).
2. Update `docs/src/roadmap/README.md` and root `ROADMAP.md`.
3. Update `dev/BOARD.md`:
   - Move completed work to Recently Done.
   - Unblock dependent parser items.

### Stage B5 — Close Out

Produce a "Mutation Pass 2 Completion Report" containing:

- Test counts per family
- Questions resolved / partially resolved / still open
- Specs promoted to 🟢
- Specs remaining 🟡 with reasons
- Any new questions discovered during testing
- Board changes
- Legal attestation (same wording as first kit)

Then respond with:
"MUTATION PASS 2 COMPLETE — ready for P-001"

---

## 7. Reporting Cadence

After each stage, pause and report:

```
## Mutation Kit 2 — Stage N

**Stage:** <name>
**Progress:** <what was done>
**Blockers:** <if any>
**Next:** <next stage>

Ready to proceed? (yes / adjust)
```

Do not proceed to the next stage without confirmation.

---

## 8. Exit Criteria

Phase A:
- ✅ Kit directory exists with all files in Stage A1.
- ✅ Test map designed and documented.
- ✅ All Section 4 questions have a fixture.
- ✅ `generate.py` regenerates all fixtures deterministically.
- ✅ `CHECKLIST.md` is complete.
- ✅ User has approved the design.

Phase B:
- ✅ All tests in `CHECKLIST.md` executed.
- ✅ All Section 4 questions resolved or documented as blocked.
- ✅ Every affected spec has an updated Open Questions table.
- ✅ Every resolvable spec is now 🟢 Specified.
- ✅ Phase-1 README reflects current status.
- ✅ `dev/BOARD.md` reflects current status.
- ✅ Every updated spec has mutation-test evidence in Provenance.
- ✅ No OMSI binary was inspected.
- ✅ No GPL source code was read.
- ✅ No OMSI asset was committed.

---

## 9. Final Reminder

The first kit succeeded because it was small, focused, and procedural.
This second kit will succeed for the same reasons. Resist the
temptation to expand scope. If a question turns out to be unresolvable
by fixture design, mark it as blocked and move on — a spec with three
honest blockers is more valuable than one with false confidence.

Begin with Section 3 (Preconditions) and report the results before
starting Phase A.