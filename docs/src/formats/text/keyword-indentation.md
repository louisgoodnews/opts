# Indented keywords in OMSI text files

| Metadata | Value |
| :--- | :--- |
| **Status** | 🟢 Implemented (scanner rule); OMSI behaviour 🟡 inferred, not tested |
| **Code** | `opts_core::formats::text::{keyword_line, scan_blocks_with}`; census `crates/opts-core/examples/keyword_indent_census.rs` |
| **Date** | 2026-10-14 |
| **Related** | [`bus.md`](vehicles/bus.md) Q-003, design `articulation-physics.md` (branch `design/articulation-physics`) Q-ART-14 |

## Finding

The shared block scanner accepted a keyword only when `[` was the **first character of the line**. Add-on authors indent keywords in many files, so every indented keyword was read as a *parameter line of the previous block*: the block it should have started never existed, and the previous block gained a stray parameter. Nothing was logged.

Measured on the reference installation (`keyword_indent_census`, a keyword line is a line that is exactly `[name]` after trimming; "lost" = raw keyword lines minus blocks the scanner returned):

| Extension | Files | With an indented keyword | Keyword lines lost before | after |
| :--- | ---: | ---: | ---: | ---: |
| `.sco` | 56,956 | 7,112 | 8,209 | 0 |
| `.cfg` | 21,851 | 1,354 | 8,594 | 0 |
| `.txt` (traffic and lists) | 10,647 | 299 | 3,969 | 0 |
| `.ovh` | 1,255 | 1,128 | 2,162 | 0 |
| `.bus` | 1,497 | 1,338 | 1,755 | 0 |
| `.sli` | 16,531 | 524 | 968 | 0 |
| `.hum` | 61 | 59 | 177 | 0 |
| `.dsc` | 943 | 5 | 5 | 0 |

Most frequent lost keywords: `.sco` `nightmapmode` 5,250, `light_enh_2` 616, `texttexture` 585, `matl` 225, `mesh` 103, `collision_mesh` 97; `.cfg` `pnt` 944, `newanim` 717, `light_enh_2` 640, `mesh` 567, `ticket_sale*` ≈ 1,000; `.bus`/`.ovh` `registration_free` 997, `friendlyname` 1,350, `couple_back` 505, `boogies` 347, `rail_body_osc` 201; `.sli` `path` 415, `rail_enh` 344, `patchwork_chain` 91; `.txt` `set_day_of_week`/`trafficdensity` 1,570 each, `group` 783; `.hum` `humangeom`, `links`, `walk_param` in 59 of 61 files.

Consequence that was measured: of 1,035 vehicle files with a `[couple_back]` only 539 were read as having one (497 `.ovh` hold it indented); chains, trailer masses and the `bus.md` Q-003 evidence were all computed from the 539.

## Rule

A line is a keyword line when, after trimming trailing whitespace,
- it starts with `[` at column 0 and ends with `]` (unchanged), **or**
- it is indented by spaces or tabs and the text between the brackets is a plain identifier (`A–Z a–z 0–9 _`).

An indented line with other bracketed text (`\t[coupling_front] / [coupling_back]`, found in `Bm238/ABm225_ob.ovh`) is **not** a keyword: it stays a parameter or note, as before. The identifier test is deliberately narrower than the column-0 rule: only 14 indented bracket lines in the whole installation have a name that never occurs at the start of a line in a file of the same extension (7 in `.ovh`, 5 in `.dsc`, 2 in `.sco`), so the indented lines are overwhelmingly real keywords.

Tests: `text::tests::indented_keywords_start_blocks`, `an_indented_note_with_brackets_is_not_a_keyword`; corpus check: the census reports `raw == parsed` for every extension (`lost 0`).

## Does OMSI honour indentation? (Q-KI-01, inferred, not tested)

No OMSI binary was inspected ([ADR-0002](../../adr/0002-clean-room-policy.md)) and no OMSI test was run. The evidence is indirect but strong:
- In **1,350 vehicle files** (463 `.bus`, 887 `.ovh`) `[friendlyname]` occurs *only* indented; the same for `[registration_free]` in 997 files. If OMSI ignored those lines, these vehicles would list without a name; they are well-known add-ons (BR481, NF6D, Chicago L).
- In **490 `.ovh` files** `[couple_back]` occurs only indented; the multi-car trams and coaches that use it work in OMSI.
- 295 `.cfg` files hold `ticket_sale_change_point`/`ticket_sale_money_point` blocks only indented (working ticket machines).

An OMSI check is still wanted: pick a vehicle whose `friendlyname` is indented-only and read its name in OMSI's vehicle list. If OMSI ignored the keyword, the name would be missing there. **Q-KI-01: open** until someone with OMSI does it.

## What changes for OPTS

Everything the indented keywords describe is now read: night maps and light maps, text textures, materials, meshes and collision meshes of scenery objects, animations and viewpoints of vehicle models, ticket machines, rail bodies and bogies, trailers, spline paths and rails, traffic densities per day of the week, human geometry. Nothing was removed. Expect visible differences on maps whose objects use indented keywords; a before/after of Rheinhausen is in the scenery investigation.

## Not covered

`formats::map::global` reads `[entrypoints]` from raw lines and `formats::constfile` has its own trimmed scanner; neither used the column-0 rule for the data measured here. Parameter lines that happen to look like `[identifier]` would now start a block; the census found none that matter (see above) but a format that allows such values must pass an `arity` or be checked.
