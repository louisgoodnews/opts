# Indented keywords in OMSI text files

| Metadata | Value |
| :--- | :--- |
| **Status** | 🟡 Investigated 2026-10-14: the scanner is **unchanged**; indented keywords are most likely comments or disabled blocks, not data |
| **Code** | `opts_core::formats::text::scan_blocks_with` (keyword only at the start of a line); census `crates/opts-core/examples/keyword_indent_census.rs` |
| **Related** | [`bus.md`](vehicles/bus.md) Q-003, design `articulation-physics.md` (branch `design/articulation-physics`) Q-ART-14 |

## Question

The shared block scanner accepts a keyword only when `[` is the first character of the line. An indented `[keyword]` line is read as a parameter line of the previous block. The first census suggested that this drops real data: 7,112 `.sco`, 1,338 `.bus`, 1,128 `.ovh`, 1,354 `.cfg`, 524 `.sli`, 299 `.txt` files contain indented keyword lines. Is that a scanner bug?

## Measurement

`keyword_indent_census` (keyword line = exactly `[name]` after trimming; "not read" = raw keyword lines minus blocks the scanner returns):

| Extension | Files | With an indented keyword | Keyword lines not read |
| :--- | ---: | ---: | ---: |
| `.sco` | 56,956 | 7,112 | 8,209 |
| `.cfg` | 21,851 | 1,354 | 8,594 |
| `.txt` | 10,647 | 299 | 3,969 |
| `.ovh` | 1,255 | 1,128 | 2,162 |
| `.bus` | 1,497 | 1,338 | 1,755 |
| `.sli` | 16,531 | 524 | 968 |
| `.hum` | 61 | 59 | 177 |
| `.dsc` | 943 | 5 | 5 |

Most frequent: `.sco` `nightmapmode` 5,250, `light_enh_2` 616, `texttexture` 585, `matl*` ≈ 650, `mesh` 103; `.cfg` `pnt` 944, `newanim` 717, `mesh` 567, `ticket_sale*` ≈ 1,050; `.bus`/`.ovh` `registration_free` 997, `friendlyname` 1,350, `couple_back` 505; `.txt` `set_day_of_week`/`trafficdensity` 1,570 each; `.hum` `humangeom`, `links`, `walk_param` in 59 of 61 files.

## Evidence that they are not data

1. **Editor help blocks.** The scenery-object template carries the help text of optional keywords as indented blocks, e.g. `Buildings_RUE/Haus02a.sco`:

   ```
   <TAB>[NightMapMode]
           Modus wie Nachtbeleuchtung eingeschaltet werden soll:
   <TAB>0: wie Strassenbeleuchtung
   <TAB>1: Gebaeude mit durchgehender Beleuchtung  …

   [NightMapMode]
   2
   ```
   The indented block is documentation, the real keyword follows at the start of a line. This is the 5,250 `nightmapmode` "losses". Reading the indented block as a keyword makes 371 of the 1,139 object types of the Rheinhausen map fail to parse (the prose line is not a number): measured with an experimental scanner that accepted indentation, which is why it was **reverted**.
2. **Disabled references.** Do the files an indented keyword names exist, compared with the same keyword at the start of a line (`keyword_indent_census`, last section)?

   | Keyword | At column 0 | Indented |
   | :--- | :--- | :--- |
   | `.ovh` `[couple_back]` | 166 of 168 exist (98.8 %) | **18 of 497 (3.6 %)** |
   | `.ovh` `[sound]` | 1,028 of 1,153 (89.2 %) | 5 of 11 (45.5 %) |
   | `.bus` `[couple_back]` | 357 of 357 | 8 of 8 |
   | `.sco` `[mesh]` | 65,680 of 69,950 (93.9 %) | 89 of 103 (86.4 %) |

   The indented trailers of the rail coaches (`Bm238` family, `\t[couple_back]` naming `..\Bm238\Bm238.bus`, which does not exist) are dead references: a loader that honoured them would fail or attach nothing, and the add-ons evidently work. They look like blocks an author switched off by indenting them.
3. Only 14 indented bracket lines in the installation have a name that never occurs at the start of a line in the same extension (7 `.ovh`, 5 `.dsc`, 2 `.sco`): indented lines are copies of real keywords, i.e. help or disabled text, not an independent vocabulary.

## Decision

The scanner keeps its rule (keyword at the start of a line only). A test documents it (`text::tests::an_indented_keyword_stays_a_parameter_line`). The earlier hypothesis that OPTS loses real data here — recorded for a few hours as a "scanner bug", design Q-ART-14 and a risk row of M-0053 — is **not supported**; the corresponding notes are corrected.

## What stays open (Q-KI-01)

The evidence is corpus-internal; no OMSI test was made and no OMSI binary inspected ([ADR-0002](../../adr/0002-clean-room-policy.md)). Minorities look like real data and could be honoured by OMSI: `.bus` `[couple_back]` 8 of 8 existing targets, `.sco` `[mesh]` 89 of 103, `.sli` `[path]` 415 (all valid), indented `[model]`/`[paths]` of vehicles (22 of 22), and the 18 existing indented `.ovh` trailers. Also unexplained: `[friendlyname]` occurs only indented in 1,350 vehicle files (under the "ignored" reading OMSI would show their file names). **Test to settle it**, needing OMSI: take a scenery object whose only `[mesh]` is indented and see whether OMSI draws it; take a vehicle whose `[friendlyname]` is indented-only and read its name in the vehicle list. Run `keyword_indent_census` for the candidates.

## Consequences

- `[couple_back]` counts: 539 files read, 1,035 on raw text; the 496 difference is mostly dead references (3.6 % resolve). The chain and mass figures computed from the 539 stand; the `consist_census` numbers in the design (480 pairs, median 0.90) are the OPTS-visible ones.
- Nothing about the Rheinhausen missing/scrambled objects follows from indentation.
