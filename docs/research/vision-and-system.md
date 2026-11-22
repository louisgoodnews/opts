# OPTS as a system, not only an OMSI replication

| Metadata | Value |
| :--- | :--- |
| **Status** | 🔬 Research, not decided |
| **Date** | 2026-11-22 |
| **Relates to** | [vision](../src/vision.md), [Phase 18](../src/roadmap/phase-18/README.md), [feature research](../audit/feature-research-omsi-lotus-2026-11-22.md) |

## 1. The question

The vision says OPTS "can load the enormous library of OMSI 2 content, simulate realistic transit operations across a living city, and give modders a platform that will outlast any single game engine" ([vision](../src/vision.md)). Much of the current work is replication: reading OMSI files and making them drive as OMSI's do. The vision also asks for things OMSI does not do: a scenario editor, a scripting layer for scenarios, and an editor that writes new content.

This page asks what would make OPTS a system in its own right. It separates that from the feature gaps, which [feature research](../audit/feature-research-omsi-lotus-2026-11-22.md) and the [OMSI parity matrix](../audit/omsi-parity-matrix-2026-11-18.md) already list.

## 2. Three kinds of part

| Kind | Meaning | Test |
| :--- | :--- | :--- |
| **Replication** | Reproduces what OMSI 2 does, so OMSI content plays the same way. | An OMSI content pack from the owner's installation loads and behaves as in OMSI, as far as OMSI's behaviour is documented or measured. |
| **Compatibility** | Keeps OMSI files readable and, where the editor writes them, loadable in OMSI. Not a feature; a constraint on the other two kinds. | A file written by OPTS loads in OMSI 2 (manual acceptance checklist, [editor roadmap](../src/editor/roadmap.md)). |
| **OPTS's own** | Content and behaviour that have no OMSI equivalent: editor-made content, scenarios, scripts written for OPTS, multiplayer if decided. | A player can see the result without knowing OMSI. |

Replication is judged against OMSI. OPTS's own parts are judged against the player. The two need different tests, and the vision currently mixes them.

## 3. The parts, sorted

| Part | Kind | Current state (2026-11-22) | Player-visible test |
| :--- | :--- | :--- | :--- |
| Reading OMSI maps, vehicles and scenery | Replication | Working; the reader and renderer exist ([roadmap](../src/roadmap/README.md)) | A map of the owner's installation starts and a bus drives on its roads |
| Vehicle logic in `.osc` | Replication | VM wired to vehicles, with the rig ([script VM](../src/architecture/script-vm.md)) | A bus's doors, lights and gearbox respond to the player's controls as in OMSI's documented behaviour |
| Scenery scripts | Replication | Not run yet ([scenery benchmark](../src/architecture/scenery-benchmark.md)) | A scripted sign or light in a scenery object changes with the time of day |
| Timetables and passengers | Replication, then OPTS's own | Routes and timetables read ([M-0099](../src/roadmap/phase-4/M-0099.md)); passengers partial ([M-0101](../src/roadmap/phase-4/M-0101.md)) | A bus arrives at its stop at the timetabled time; passengers board and alight |
| Editor for OMSI content | Compatibility, and OPTS's own tool | Specified, not started ([editor README](../src/editor/README.md)) | A creator moves a stop on a map, saves it, and the game shows the change within seconds (E1 exit criterion) |
| Editor-made content that OMSI also loads | Compatibility | Not started | The same file loads in OMSI 2 and in OPTS, checked by hand |
| Scenarios with a goal and a score | OPTS's own | Not started. No scenario or challenge code exists ([M-0172](../src/roadmap/phase-18/M-0172.md)) | A player starts a named scenario, sees a goal, and gets a score at the end |
| Roaming emergency and service vehicles | Replication, then OPTS's own | Groups spawn from map data; siren priority open ([M-0173](../src/roadmap/phase-18/M-0173.md)) | A police car appears on the road at a plausible time and gives way to the player when its siren sounds |
| Scripting for scenarios and new content | OPTS's own | Decision open ([scripting-dsl.md](scripting-dsl.md)) | A creator writes a scenario that a player can start, without a Rust change |
| Multiplayer dispatch | OPTS's own | Vision item, no milestone ([ISS-046](../dev/issues/ISS-046-multiplayer-co-op-dispatch.md)) | Two players share one route, one driving and one dispatching |
| Career and progression | OPTS's own | Vision item, no milestone (RG-06) | A player's depot, money and vehicles change between sessions |
| Virtual reality | OPTS's own | Out of scope as a vision item (owner decision 2026-11-22, recorded in the [vision](../src/vision.md)) | Not tested |

## 4. What "a system" would mean, concretely

Four statements make the difference between a replica and a system. Each is a claim the owner can accept or reject.

1. **OPTS's content has its own format and its own editor.** New maps, scenarios and vehicles are written in files OPTS owns, or in OMSI's files when compatibility is required. The file a creator edits is never a hidden database ([editor README P1 and P4](../src/editor/README.md)).
2. **Scenarios are OPTS's own.** A scenario is a file, a rule set and a score. It does not depend on OMSI's situations (`.osn`), although it may read them.
3. **The simulation is the same for everyone who loads the same content.** Determinism ([script VM](../src/architecture/script-vm.md), seeded random) is what lets scenarios be scored fairly and saves be shared. A system needs this; a replica can live without it.
4. **The platform accepts content from people who did not write the engine.** This needs the editor, a documented scripting surface, and a distribution path. The vision lists modding as a pillar ([vision](../src/vision.md)); the sharing layer is RG-10 in the feature research and sits in Phase 6.

Statements 1 to 3 can be met without multiplayer or VR. Statement 4 needs the editor and a scripting decision before it can be tested.

## 5. The tests, restated

For OPTS's own parts, the test is the player-visible outcome in the table above. For replication, the test is OMSI's behaviour as documented or measured. For compatibility, the test is a file loading in both programs. A part that has none of the three tests should not be counted as done.

## 6. Quoting the vision

The vision's core line is: "OPTS is the open-source home for public transport simulation" ([vision](../src/vision.md)). It also says "The goal is not to reach the summit on a schedule." This research does not change either statement. It asks only that each part be tagged as replication, compatibility or OPTS's own, so that the roadmap can be checked against the vision.

## 7. Open questions

* **Q-VS-1.** Does "outlast any single game engine" require OPTS's own formats to be documented before the first editor release? Owner decision 4 in the [README](README.md) addresses the related OMSI question.
* **Q-VS-2.** Is multiplayer a system feature (statement 3 needs a shared simulation) or a separate product? Owner decision 12.
* **Q-VS-3.** Is career progression a scenario rule set, or its own system? Owner decision 11.
* **Q-VS-4.** Which of the OPTS-own parts must exist before the vision's Stage 5 ("Platform", [vision](../src/vision.md))? Not decided; the vision describes stages but not their contents.
