# OMSI Experiment Protocol for the Bus Script Integration

| Metadata | Value |
| :--- | :--- |
| **Status** | 🟡 Protocol (2026-10-14); no experiment has been run |
| **Serves** | [Bus script integration strategy](bus-script-integration.md) (section 8), [callbacks reference](../formats/text/vehicles/callbacks.md), [pending OMSI verification](pending-omsi-verification.md) |
| **Needs** | a running OMSI 2 with the reference vehicle (MAN NG272, `MAN_NL_NG`), SD200, HH109 and the NF6D; a screen recorder; about two hours in two sessions |
| **AI side** | the AI experiments are in [AI calibration protocol](ai-calibration-protocol.md); run them in the same sittings when convenient |

The point of this page is that a session can follow a checklist instead of improvising, and that **everything is recorded**: a short screen recording or a screenshot answers questions that nobody thought of at the time. Order matters: finish a session's list before leaving the game.

## Ground rules

1. **Never edit the installation.** Where an experiment needs a changed script or file, copy the vehicle folder to a new name (for example `Vehicles/MAN_NL_NG_EXP`) and edit the copy; delete the copy afterwards or keep it outside the repository.
2. **Record, do not only observe.** Use an external recorder for OMSI (OBS or the operating system's recorder; 1080p, 30 fps, game audio off is fine). The OPTS tools help on the *other* side: the F1 reporting and the new `screenshot` and `capture` commands record what **OPTS** shows for the same scenario, so the two can be compared side by side. They cannot record OMSI.
3. **Name files `<session>-<id>-<what>.<ext>`**, for example `s1-R1-front-sign-line130.png`, and put a text file `README.txt` beside them with date, vehicle, map, OMSI version and settings.
4. **Commit immediately** (screenshots and notes, no OMSI assets such as extracted textures or scripts) into `docs/src/meta/data/omsi-experiments/<date>/`, so nothing lives only on one machine. Keep recordings out of the repository (they are large) and list their file names and a checksum in the README.
5. **One change per run** where a script copy is edited; write the change into the README before running.
6. **Say what you saw in one sentence per item** even with a recording; the sentence is the index.

## Session 1: route data and the `M_Wheel` unit (about 1 hour)

Route data first: it is the first coverage cliff (96.8 % of the buses) and blocks the data layer (P3); `M_Wheel` blocks the physics contract (P1).

### Route data (R1-R6)

Setup: the reference bus (NG272) on a map whose `.hof` it uses (find the hof by opening the vehicle's `Matrix`/`IBIS` script folder or the map's hof suite; note its file name). Use the in-game bus-stop and IBIS dialogs (line and course entry, destination selection). Record the **screen**, including the cockpit display and the outside view of the front sign, side sign and roller blind.

| ID | Do | Record | Learns |
| :--- | :--- | :--- | :--- |
| R1 | Select five routes that differ: a terminus with two display lines, one with a special string, one with a roller-blind texture, one with an "extended" IBIS-2 text, one empty destination. For each: front sign, side sign, roller blind, IBIS display, the line and course you entered | one screenshot per display per route, plus the entered code | which hof column feeds which display (`GetTerminusString` columns 0-25), centring and truncation rules |
| R2 | Enter line 130 course 01, then 130 course 02, then line 131; also a route code with a one-digit line | what the IBIS shows after each entry; whether an invalid code is rejected | `GetRouteIndex` input `line × 100 + course`, `-1` for "not found" behaviour |
| R3 | Drive a route with at least six stops (or use the stop selector); at each stop record the stop name on the display and on the announcement text, and the stop's sequence number | per stop: displayed name, announced name, number | the `[infosystem_busstop_list]` order, `GetRouteBusstopIdent`, `GetBusstopString` columns 0-8 |
| R4 | Start a **scheduled** drive with a known timetable file; at three stops arrive early, on time and late (deliberately); record the delay display each time | arrival clock time, scheduled time, displayed delay and its colour or text ("too early"/"too late") | the sign of `GetTTDelay`, times in seconds of day, `GetTTBusstopArr/Dep`, `GetTTBusstopIndex` stepping |
| R5 | During the drive change the destination from the IBIS | what changes on each display, whether the stop list changes | `GetTTTerminusIndex`, `GetRouteTerminusIndex`, `ai_scheduled_settarget` effects |
| R6 | A free drive without any timetable | what every display shows; any error message | the neutral values the host must return when no timetable is active |

### `M_Wheel` unit (E1)

| ID | Do | Record | Learns |
| :--- | :--- | :--- | :--- |
| E1 | On a flat straight road with no wind and no grade: full throttle from standstill with each of three vehicles whose masses are in their files (a light solo bus, SD200, NG272 empty) | per vehicle: time to 20, 40 and 50 km/h, the distance to 50 km/h, gear used, the loaded mass (note passengers: none), the vehicle file and `[mass]` | the unit and distribution of `M_Wheel` (Q-BSI-1) from `m · a = F` with the wheel radius of the driven axle |
| E1b | Full brake from 50 km/h on the same road, same vehicles | stopping distance and time | the brake-force unit (`Brakeforce`) |

## Session 2: script textures, passengers and routing (about 1 hour)

These need **edited copies** of vehicle folders (rule 1). Find the call sites in the copy with a text search, for example `grep -n -i "sttextout" <copy>/Script -r`. The edits below are ours and untested in OMSI ❓; if a display breaks, note what happened: that is data too.

| ID | Do | Record | Learns |
| :--- | :--- | :--- | :--- |
| E2 | In the copy's display script change the last two arguments of one `STTextOut` call (`a`, `b`) one at a time: `a` in 0, 1, 2, 3; `b` in 0, 3, 10, 100, -1; load the vehicle each time | a screenshot of the display for each value | meaning of the two arguments (spacing, alignment, clip) |
| E3 | Change the colour arguments of one `STSetColor` call to distinct values (alpha 17, red 90, green 140, blue 200); then once drawing over an existing colour | screenshots; if a pixel read-back is easy, the values read | argument order and blending |
| E4 | In the copy, write the result of `GetHeightAbovePoint` for z in -1, 0, 0.15, 0.5 and 2 into a displayed string or use `$msg`; park beside a known 0.15 m kerb; repeat kneeling on and off | the numbers (photograph the display or the message window) | frame, sign and no-hit value |
| E5 | Call `STLoadTex` in the copy with an existing, a missing and a differently sized file | screenshots; what the stack-dependent code after it does | semantics, scaling, base path |
| E6 | One passenger on a known seat or link of the rear part of the articulated NG272; write `GetHumanCountOnSeat`/`OnPathLink` for indices 0-199 to a displayed list, repeat with two passengers on one link | which indices are non-zero | index space, count vs flag |
| E8 | Bind a key in `Inputs/keyboard.cfg` *copy* to a trigger defined in two parts (the NF6D C-car door) | which parts react | trigger routing (Q-BSI-5) |

## Deferred

E7 (events and triggers the engine raises itself: `ai_scheduled_settarget`, `veh_tank`, `collision`, `malfunction_reset`, `ev_ticketsale_*`) and every sound experiment wait for P6 and the audio decision. If OMSI is open anyway, one 10-minute recording of an ordinary drive with the sound on is cheap and useful later.

## After each session

1. Commit the notes and screenshots (rule 4) and send the path.
2. The agent reads the recordings, writes the answers into [callbacks reference](../formats/text/vehicles/callbacks.md), [`hof.md`](../formats/text/vehicles/hof.md) and the constants (`M_WHEEL_UNIT`), moves the item in [pending OMSI verification](pending-omsi-verification.md) to answered and lists the follow-up questions for the next session.

**Clean-room attestation (ADR-0002):** observation of the running product and of the user's own edited copies; no binaries inspected, nothing from the installation committed.
