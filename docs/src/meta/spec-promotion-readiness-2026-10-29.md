# Which Draft Format Specs Could Be Promoted (2026-10-29)

| Metadata | Value |
| :--- | :--- |
| **Status** | 🟡 Preparation for the maintainer's review; nothing was promoted |
| **Milestone** | [M-0004](../roadmap/phase-1/M-0004.md) (promoting the remaining Draft specs to Specified) |
| **Author** | Claude Sonnet 5.5, from the spec files; the maintainer decides |

## What this is, and what it is not

M-0004 says a spec stays 🟡 Draft "until the maintainer review and the relevant mutation tests are done", and that 🟢 Specified "requires the spec's high-priority Open Questions to be resolved". The review and the experiments in OMSI are not mine to do, so this page prepares them: for each Draft spec it says whether anything *blocks* a review, so the maintainer can start with the ones that are ready.

**Method.** All `docs/src/formats/**/*.md` whose metadata status says Draft: **22**. The [Phase 1 page](../roadmap/phase-1/README.md) says 23; I did not trace the difference (a promotion since, or another counting rule). Nine further pages have the status *Stub* and are not covered here. For each: the open questions with a 🔴 priority that are not marked resolved (a regex over the `Q-…` table rows), and whether the headings of the template are there (overview, structure, field reference, examples, open questions, provenance, related documents). **Limits:** the headings are matched by name, so a spec that has its fields under another heading shows as "missing" below; I did not re-check any spec's content against the corpus; "ready" means "nothing in the rules blocks the review", not "correct".

## A. Format specs with nothing blocking a review (9)

| Spec | Questions (high / medium / low) | Note |
| :--- | :--- | :--- |
| [`binary/o3d-chunks.md`](../formats/binary/o3d-chunks.md) | none of its own | Defers all questions to [`o3d.md`](../formats/binary/o3d.md), which is already 🟢 Specified and has no open 🔴 question |
| [`map/tiles.md`](../formats/map/tiles.md) | 0 / 6 / 5 | Q-M07 (water marker, interpolation) needs an in-engine check but is rated medium |
| [`text/misc-types.md`](../formats/text/misc-types.md) | 0 / 2 / 4 | |
| [`text/scenery-scripts.md`](../formats/text/scenery-scripts.md) | 0 / 2 / 2 | |
| [`text/ticketpacks.md`](../formats/text/ticketpacks.md) | 0 / 2 / 2 | |
| [`wav.md`](../formats/wav.md) | 0 / 2 / 4 | |
| [`text/engine-config.md`](../formats/text/engine-config.md) | 0 / 0 / 2 | No heading called field reference: check that the fields are described |
| [`text/fonts.md`](../formats/text/fonts.md) | 0 / 2 / 2 | Same |
| [`text/opl.md`](../formats/text/opl.md) | 0 / 2 / 1 | Same |

## B. Format specs blocked by an open high-priority question (6)

Each question needs an experiment in OMSI itself (the spec says so), so none can be settled from the files.

| Spec | Open high-priority question |
| :--- | :--- |
| [`map/global-cfg.md`](../formats/map/global-cfg.md) | **Q-G01** the three trailing `[groundtex]` numbers: change one in a copy and look at the ground |
| [`map/timetables.md`](../formats/map/timetables.md) | **Q-T01** the second `[profile]` number and how a profile is chosen at run time; **Q-T02** are profile station indices 0- or 1-based |
| [`text/repaints.md`](../formats/text/repaints.md) | **Q-RP01** what the `[CTC]` name binds to (a script variable holding the index?). The repaint check in the [review checklist](../../dev/review-checklist.md) (item 2) bears on it |
| [`text/vehicles/predefined-variables.md`](../formats/text/vehicles/predefined-variables.md) | **Q-PV01** argument lists and results of each callback |
| [`text/vehicles/script-sets.md`](../formats/text/vehicles/script-sets.md) | **Q-SS01** does the order of the `[script]` list matter; **Q-SS02** are macros global across the scripts of one vehicle |
| [`textures.md`](../formats/textures.md) | **Q-TX07** image origin against the mesh UV convention; the spec says the mutation kit's T02 test answers it for `.bmp`: the maintainer can check whether that is enough to close it, or whether `.tga` needs its own |

## C. Analyses and reports, not format specifications (5)

[`inventory.md`](../formats/inventory.md), [`gap-analysis.md`](../formats/gap-analysis.md), [`community-crosscheck.md`](../formats/community-crosscheck.md) (Q-CC02 is open and high: the exact behaviour of `[matl_alpha]` modes), [`text/sco-sli-audit.md`](../formats/text/sco-sli-audit.md) (Q-AUD-005 is open and high: do the misspelled collision keywords work in OMSI) and [`text/vehicles/vehicle-corpus-audit.md`](../formats/text/vehicles/vehicle-corpus-audit.md).

They describe a corpus or compare sources; "Specified" does not mean much for them. A decision for the maintainer: give such pages another status (for example *Report*) instead of keeping them in the Draft count.

## D. A hub and a guide (2)

* [`text/cfg.md`](../formats/text/cfg.md) is an index of configuration formats with no questions of its own and no structure to specify; it could be marked as an index.
* [`text/vehicles/osc-guide.md`](../formats/text/vehicles/osc-guide.md) is a guide to reading the scripts. Its five open questions (Q-OSC-G1 to G5) all need OMSI experiments and carry no priority column, so they would have to be rated before the rule can be applied.

## What would move this forward

1. **Nine specs in A** can be reviewed now. After the review they can be set to 🟢 Specified, with the mutation-test condition checked per spec (this page did not check which tests exist).
2. **The six in B** wait for experiments in OMSI; the list above is the work order for them.
3. **C and D** need a decision about the status vocabulary, not work.

With A promoted, the Draft count would fall from 22 to 13, and to 6 once C and D are handled.
