# Predefined Variables, Callbacks and Accessor Syntax

| Metadata | Value |
| :--- | :--- |
| **Format Family** | Reference (plain-text lists shipped in `program/`) |
| **File Extensions** | `program/*.txt` |
| **Parser Crate** | `opts-core` (data tables for the script engine) |
| **Parser Module** | `opts_core::formats::osc::predefined` |
| **Encoding** | ASCII, CP1252 (`Info_scriptcode.txt`) |
| **Status** | 🟡 Draft — lists complete; meanings largely inferred |
| **Last Updated** | 2026-10-04 |

---

## 📖 Overview

OMSI ships ten small text files in `program/` that list **what the engine itself provides to scripts**: predefined variables per object class, callbacks, and a short description of the script code. They are the authoritative vocabulary behind the [`.osc` language](osc.md): a script may use any name from these lists without declaring it, while everything else must be declared in a varlist ([variable lists](varnamelist.md)). This page documents the lists, the **accessor syntax** `(<kind>.<scope>.<name>)`, and how the 5,821 installed scripts use them.

Evidence tags: **[C]** counted over all 5,821 `.osc` files, **[H]** read in the shipped files, 📚 community wiki (restated, CC BY-SA 4.0), 📖 shipped manual (paraphrased), ❓ unverified. No behaviour has been tested in OMSI; names are facts, meanings below are inferred from names unless tagged.

### Source files

| File | Names | Role |
| :--- | ---: | :--- |
| `varlist_roadvehicle.txt` | 138 | predefined **vehicle** variables |
| `varlist_system.txt` | 22 | predefined **system** variables (read-only) |
| `varlist_scenobj.txt` | 9 | predefined **scenery-object** variables |
| `varlist_human.txt` | 4 | predefined **human** variables |
| `stringvarlist_roadvehicle.txt` | 7 | predefined vehicle **string** variables |
| `callbacklist_roadvehicle.txt` | 25 | callbacks available to vehicle scripts |
| `callbacklist_scenobj.txt` | 3 | callbacks available to scenery scripts |
| `callbacklist_scripttex.txt` | 17 | the script-texture drawing API |
| `gamectrl_axes.txt` | 5 | controller axes |
| `Info_scriptcode.txt` | — | short German description of the script code |

## 🔤 General Structure

### What `Info_scriptcode.txt` says (paraphrased [H])

- Evaluation uses **two temporary stack systems** — a *permanent* one and an *additional* one — each with **8 float slots numbered 0–7**. A step either imports a value or applies an operation; the new result goes into slot 0, every older value moves back by one and the last slot is dropped. Example in the file: `1 4 +` leaves 5 in slot 0.
- Operations: `+ * - /` (a **division by zero resets the stack to 0**), `pi`, `sin` (of slot 0) and `random` (a random number from 0 up to the value in slot 0).
- Numbers are written with a decimal point. `l0`…`l7` copy a slot of the additional stack into slot 0 of the permanent stack; `s0`…`s7` write slot 0 of the permanent stack into a slot of the additional stack. (The wiki calls the additional stack "registers" 📚.)
- Variables, system/environment values and triggers are reached with `(#.#.#name)`: the **first letter** is *L* load float, *X* load string, *T* execute sound trigger or *S* save to variable; the **second letter** is *L* local, *M* map, *S* system.

### Accessor syntax as used in the corpus [C]

The in-file legend is incomplete: the installed scripts use `$` as the second letter for strings (the legend's *X*, "load string", never occurs — 0 uses), and two combinations that the legend does not list, `M.L` and `M.V`.

| Accessor | Uses | Distinct names | Meaning | Most used names |
| :--- | ---: | ---: | :--- | :--- |
| `(L.L.name)` | 800,062 | 12,714 | Load a **local float** variable (vehicle/scenery/human) | `elec_busbar_main` 22,693, `velocity` 13,099, `engine_n` 12,876 |
| `(S.L.name)` | 620,095 | 17,843 | **Save** to a local float variable | `antrieb_getr_aktugang` 4,162, `bremse_p_tank04` 3,784, `doorspeed_0` 3,644 |
| `(L.$.name)` | 74,503 | 1,164 | Load a **local string** variable | `ineo_datatransferstring` 8,372, `matrix_nr` 7,079, `matrix_terminusl1` 5,416 |
| `(S.$.name)` | 101,850 | 2,971 | **Save** to a local string variable | `ineo_datatransferstring` 8,380, `matrix_terminusl1` 3,532, `matrix_terminusl2` 3,472 |
| `(L.S.name)` | 78,218 | 20 | Load a **system** variable (read-only) | `timegap` 50,122, `gettime` 5,559, `time` 5,121 |
| `(C.L.name)` | 110,340 | 1,821 | Load a **constant** (`[const]` of a constfile) | `elec_busbar_minv` 8,657, `antrieb_i_achse` 3,261, `matrix_height` 2,508 |
| `(F.L.name)` | 22,634 | 941 | Evaluate a **curve** (`[newcurve]` of a constfile) at the value on the stack | `bremse_suspension_regulatorcurve` 854, `antrieb_wandler_lowrpmsoftness` 742, `retarder_fadein` 641 |
| `(M.L.name)` | 107,665 | 1,807 | **Call a macro** defined with `{macro:name}` | `bremse_druckausgleich` 4,621, `matrix_pixelrefresh` 3,811, `traegheit` 3,214 |
| `(M.V.name)` | 66,767 | 42 | **Call an engine callback** from the callback lists | `getfontindex` 6,160, `getterminusstring` 4,891, `textlength` 4,296 |
| `(T.L.name)` | 64,075 | 2,215 | Fire a **sound/event trigger** (`ev_…` names, handled by the sound configuration) | `ev_ibis_piep` 1,598, `ev_printer_button_on` 1,307, `ev_schaltruck` 1,267 |
| `(T.F.name)` | 724 | 18 | Fire a trigger that takes a **file name from the string stack** (e.g. announcements `ev_ibis_ansagen`) | `ev_ibis_ansagen` 315, `ev_funkspruch` 98, `ev_efad_ansage` 58 |
| `(T.S.name)` | 6 | 1 | Trigger, system scope (6 uses, one name) | `smetak` 6 |

**Findings.** `M.V.<name>` is **always a callback**: all 66,767 uses name an entry of the three callback lists (42 distinct names). `M.L.<name>` is **always a macro call**: 107,558 of 107,665 uses (99.9 %) name a `{macro:…}` defined somewhere in the corpus. The other pairs read or write variables, constants, curves or triggers as the table says. Names are written without regard to case in the corpus (`Velocity`/`velocity`), so lookup must be case-insensitive ❓ (see [community cross-check](../../community-crosscheck.md) Q-CC01).

## 📚 Field Reference

> **Machine-readable since 2026-10-18.** The inputs the engine feeds are the table `INPUT_TABLE` in `opts-sim/src/script/inputs.rs` (name, unit, producer, whether scripts may also write it, whether a real value reaches it), the expected outputs are `OUTPUT_TABLE` (consumer, whether OPTS reads it); the game's feed is generated from the table and `script_rig` reports the names scripts read that it does not feed ([Script Integration Report](../../../architecture/script-integration-report.md)). This page stays the reference for meanings and evidence.

### Predefined variables

"Uses" counts all accessor uses of that name over all scripts; **14 of 180 names are never used** by any installed script (`relrange`, `Axle_Steering_1_L`, `Axle_Steering_1_R`, `Axle_Steering_2_L`, `Axle_Steering_2_R`, `Axle_Steering_3_L`, `Axle_Steering_3_R`, `NoSound`, `InUse`, `LastMovedDist`, `PAX_State`, `HeightOfSeat`, `act_route`, `file_schedule`). A name may appear in several lists (`Colorscheme`, `wearlifespan`, `Time`); it is then predefined in each class.

#### Vehicle (138 names)

| Name | Uses in `.osc` | Accessor(s) | Meaning |
| :--- | ---: | :--- | :--- |
| `Velocity` | 13,099 | L.L | vehicle speed 📚 |
| `Throttle` | 8,808 | L.L, S.L | accelerator pedal, 0–1 📚 |
| `M_Wheel` | 4,688 | L.L, S.L | torque passed to the physics ❓📚 |
| `Brake` | 2,521 | L.L, S.L | brake pedal 📚 |
| `wearlifespan` | 2,208 | L.L | **Lifespan factor**, not a wear amount: 133 scripts fail the drive when the error distance exceeds `Fehlertoleranz * WearLifespan` and draw breakdown distances with `random(Fehlertoleranz * WearLifespan)`; OPTS feeds 1000 (no breakdowns), 0 broke the MAN NG313 at the first metre |
| `Cabinair_Temp` | 2,169 | L.L, S.L |  |
| `n_Wheel` | 1,696 | L.L | mean wheel speed (rpm) 📚 |
| `AI_Engine` | 1,678 | L.L |  |
| `Axle_Springfactor_0_R` | 1,672 | L.L, S.L |  |
| `AI` | 1,429 | L.L |  |
| `Axle_Springfactor_1_R` | 1,393 | L.L, S.L |  |
| `AI_Scheduled_AtStation` | 1,284 | L.L, S.L |  |
| `schedule_active` | 1,133 | L.L |  |
| `Velocity_Ground` | 1,040 | L.L |  |
| `Axle_Springfactor_0_L` | 1,020 | L.L, S.L |  |
| `Axle_Springfactor_1_L` | 992 | L.L, S.L |  |
| `Brakeforce` | 931 | L.L, S.L |  |
| `Wheel_RotationSpeed_1_R` | 928 | L.L |  |
| `Axle_Steering_0_L` | 884 | L.L |  |
| `Axle_Suspension_0_R` | 863 | L.L |  |
| `Axle_Springfactor_2_R` | 820 | L.L, S.L |  |
| `Spot_Select` | 806 | L.L, S.L | 0-based index of the shining `[spotlight]` in declaration order; `-1` = none (SD200 `lights.osc`: `0` high beam, `1` dipped, `-1` off; TW6000 `lights.osc`: `0`/`-1`) |
| `Wheel_RotationSpeed_1_L` | 781 | L.L |  |
| `AI_Light` | 772 | L.L, S.L |  |
| `AI_Blinker_L` | 758 | L.L, S.L |  |
| `AI_Blinker_R` | 754 | L.L, S.L |  |
| `Refresh_Strings` | 736 | S.L | request to refresh string variables ❓ |
| `GivenTicket` | 722 | L.L, S.L |  |
| `kmcounter_km` | 719 | L.L |  |
| `AI_target_index` | 685 | L.L, S.L |  |
| `Axle_Suspension_1_L` | 656 | L.L |  |
| `Axle_Suspension_0_L` | 652 | L.L |  |
| `Axle_Suspension_1_R` | 644 | L.L |  |
| `PrecipRate` | 607 | L.L, L.S | precipitation rate; OPTS sets `0…1` = the `.owt` intensity / 255, 0 when dry (the scale is a guess ❓) |
| `Axle_Springfactor_2_L` | 603 | L.L, S.L |  |
| `PAX_Exit1_Req` | 595 | L.L, S.L |  |
| `PAX_Exit0_Req` | 566 | L.L, S.L |  |
| `kmcounter_m` | 546 | L.L |  |
| `Wheel_RotationSpeed_0_L` | 531 | L.L |  |
| `Wheel_RotationSpeed_0_R` | 531 | L.L |  |
| `target_index_int` | 504 | L.L, S.L |  |
| `Debug_0` | 496 | S.L |  |
| `Cabinair_absHum` | 496 | L.L, S.L |  |
| `PAX_Exit2_Req` | 462 | L.L, S.L |  |
| `Snd_Microphone` | 456 | L.L, S.L |  |
| `Debug_1` | 436 | S.L |  |
| `PAX_Exit3_Req` | 427 | L.L, S.L |  |
| `A_Trans_Y` | 426 | L.L |  |
| `humans_count` | 417 | L.L |  |
| `A_Trans_X` | 416 | L.L |  |
| `PAX_Exit1_Open` | 408 | S.L |  |
| `PAX_Exit0_Open` | 392 | S.L |  |
| `Debug_2` | 374 | S.L |  |
| `PAX_Entry0_Open` | 348 | L.L, S.L |  |
| `PAX_Exit3_Open` | 330 | S.L |  |
| `PAX_Entry1_Open` | 328 | L.L, S.L |  |
| `Debug_3` | 325 | S.L |  |
| `Debug_4` | 320 | S.L |  |
| `Wheel_Rotation_1_R` | 314 | L.L, S.L |  |
| `PAX_Exit2_Open` | 306 | S.L |  |
| `tank_percent` | 305 | L.L, S.L |  |
| `Debug_5` | 302 | S.L |  |
| `StreetCond` | 299 | L.L |  |
| `Axle_Suspension_2_R` | 293 | L.L |  |
| `Clutch` | 292 | L.L, S.L |  |
| `Axle_Suspension_2_L` | 291 | L.L |  |
| `AI_Interiorlight` | 277 | L.L, S.L |  |
| `FF_Vib_Period` | 264 | S.L |  |
| `PAX_Entry2_Req` | 250 | L.L, S.L |  |
| `PAX_Entry3_Req` | 250 | L.L, S.L |  |
| `PAX_Entry2_Open` | 230 | L.L, S.L |  |
| `PAX_Entry3_Open` | 206 | L.L, S.L |  |
| `Wheel_RotationSpeed_2_L` | 201 | L.L |  |
| `Wheel_RotationSpeed_2_R` | 201 | L.L |  |
| `Axle_Springfactor_3_R` | 199 | L.L, S.L |  |
| `FF_Vib_Amp` | 183 | S.L |  |
| `Axle_Brakeforce_0_R` | 180 | L.L, S.L |  |
| `Snd_Radio` | 173 | L.L, S.L |  |
| `Axle_Brakeforce_0_L` | 171 | L.L, S.L |  |
| `Axle_Brakeforce_1_L` | 169 | L.L, S.L |  |
| `Axle_Brakeforce_1_R` | 169 | L.L, S.L |  |
| `PAX_Entry4_Req` | 162 | L.L, S.L |  |
| `Axle_Springfactor_3_L` | 158 | L.L, S.L |  |
| `PAX_Entry5_Req` | 150 | L.L, S.L |  |
| `PAX_Entry4_Open` | 144 | L.L, S.L |  |
| `train_me_reverse` | 141 | L.L |  |
| `PAX_Exit4_Req` | 140 | L.L, S.L |  |
| `PAX_Entry5_Open` | 137 | L.L, S.L |  |
| `Envir_Brightness` | 135 | L.L | ambient brightness ❓ |
| `PAX_Exit5_Req` | 128 | L.L, S.L |  |
| `Snd_OutsideVol` | 128 | S.L |  |
| `DirtRate` | 126 | L.L |  |
| `A_Trans_Z` | 119 | L.L |  |
| `PAX_Exit4_Open` | 102 | S.L |  |
| `Driver_Seat_VertTransl` | 100 | L.L |  |
| `PAX_Exit5_Open` | 100 | S.L |  |
| `PAX_Entry1_Req` | 92 | L.L, S.L |  |
| `PAX_Entry0_Req` | 89 | L.L, S.L |  |
| `Colorscheme` | 84 | L.$, L.L, S.L | chosen repaint index (see repaints) |
| `TrafficPriority` | 80 | L.L, S.L |  |
| `Axle_Brakeforce_2_L` | 79 | L.L, S.L |  |
| `Axle_Brakeforce_2_R` | 79 | L.L, S.L |  |
| `train_frontcoupling` | 79 | L.L |  |
| `TrafficPriorityWarningNeeded` | 78 | L.L |  |
| `PAX_Entry6_Open` | 64 | L.L, S.L |  |
| `PAX_Entry7_Open` | 57 | S.L |  |
| `train_backcoupling` | 55 | L.L |  |
| `PrecipType` | 50 | L.L, L.S | what falls; OPTS uses the `.owt` coding 0 none, 1 rain, 2 snow ❓ |
| `PAX_Entry6_Req` | 48 | L.L, S.L |  |
| `PAX_Entry7_Req` | 48 | L.L, S.L |  |
| `AI_Brakelight` | 36 | L.L |  |
| `Axle_Suspension_3_L` | 32 | L.L |  |
| `Axle_Suspension_3_R` | 30 | L.L |  |
| `Axle_Steering_0_R` | 30 | L.L |  |
| `Wheel_Rotation_2_R` | 28 | L.L, S.L |  |
| `PAX_Exit7_Req` | 28 | L.L, S.L |  |
| `PAX_Exit6_Open` | 25 | S.L |  |
| `Wheel_Rotation_0_L` | 24 | L.L, S.L |  |
| `PAX_Exit6_Req` | 23 | L.L, S.L |  |
| `PAX_Exit7_Open` | 22 | S.L |  |
| `Wheel_Rotation_2_L` | 18 | L.L, S.L |  |
| `Axle_Brakeforce_3_L` | 16 | L.L, S.L |  |
| `Axle_Brakeforce_3_R` | 16 | L.L, S.L |  |
| `Wheel_Rotation_0_R` | 14 | L.L, S.L |  |
| `Wheel_Rotation_1_L` | 14 | L.L, S.L |  |
| `Dirt_Norm` | 13 | S.L |  |
| `Wheel_Rotation_3_L` | 8 | L.L, S.L |  |
| `Wheel_Rotation_3_R` | 8 | L.L, S.L |  |
| `Wheel_RotationSpeed_3_L` | 8 | L.L |  |
| `Wheel_RotationSpeed_3_R` | 8 | L.L |  |
| `Cabinair_relHum` | 2 | L.L |  |
| `relrange` | 0 | — |  |
| `Axle_Steering_1_L` | 0 | — |  |
| `Axle_Steering_1_R` | 0 | — |  |
| `Axle_Steering_2_L` | 0 | — |  |
| `Axle_Steering_2_R` | 0 | — |  |
| `Axle_Steering_3_L` | 0 | — |  |
| `Axle_Steering_3_R` | 0 | — |  |

#### System (22 names; read with `(L.S.name)`)

| Name | Uses in `.osc` | Accessor(s) | Meaning |
| :--- | ---: | :--- | :--- |
| `Timegap` | 50,122 | L.S | duration of the current frame (s) 📚 |
| `GetTime` | 5,685 | L.S, M.L | absolute time since start 📚 |
| `Time` | 5,143 | L.$, L.L, L.S, S.$, S.L |  |
| `mouse_y` | 3,629 | L.S |  |
| `mouse_x` | 3,084 | L.S |  |
| `Day` | 2,833 | L.S |  |
| `Month` | 2,320 | L.$, L.S, S.$ |  |
| `wearlifespan` | 2,208 | L.L | **Lifespan factor**, not a wear amount: 133 scripts fail the drive when the error distance exceeds `Fehlertoleranz * WearLifespan` and draw breakdown distances with `random(Fehlertoleranz * WearLifespan)`; OPTS feeds 1000 (no breakdowns), 0 broke the MAN NG313 at the first metre |
| `Weather_Temperature` | 2,168 | L.L, L.S | outside air temperature (°C) 📚 |
| `Year` | 1,749 | L.S |  |
| `PrecipRate` | 607 | L.L, L.S | precipitation rate; OPTS sets `0…1` = the `.owt` intensity / 255, 0 when dry (the scale is a guess ❓) |
| `Weather_AbsHum` | 546 | L.S | absolute humidity of the outside air; OPTS computes g/m³ from the temperature and the dew point of the weather preset ❓ unit |
| `DayOfYear` | 304 | L.S, S.L |  |
| `coll_energy` | 292 | L.S |  |
| `coll_pos_y` | 286 | L.S |  |
| `coll_pos_z` | 137 | L.S |  |
| `coll_pos_x` | 60 | L.S |  |
| `PrecipType` | 50 | L.L, L.S | what falls; OPTS uses the `.owt` coding 0 none, 1 rain, 2 snow ❓ |
| `SunAlt` | 40 | L.S, S.L |  |
| `AutoClutch` | 37 | L.S |  |
| `Pause` | 16 | C.L, L.L, L.S, S.L |  |
| `NoSound` | 0 | — |  |

#### Scenery object (9 names)

| Name | Uses in `.osc` | Accessor(s) | Meaning |
| :--- | ---: | :--- | :--- |
| `Refresh_Strings` | 736 | S.L | request to refresh string variables ❓ |
| `TrafficLightPhase` | 424 | L.L | signal phase 0–11 📚 |
| `Switch` | 255 | L.L, S.L |  |
| `Colorscheme` | 84 | L.$, L.L, S.L | chosen repaint index (see repaints) |
| `NightlightA` | 40 | L.L |  |
| `Signal` | 19 | L.L |  |
| `NextSignal` | 18 | L.L |  |
| `TrafficLightApproach` | 17 | L.L |  |
| `InUse` | 0 | — |  |

#### Human (4 names)

| Name | Uses in `.osc` | Accessor(s) | Meaning |
| :--- | ---: | :--- | :--- |
| `Colorscheme` | 84 | L.$, L.L, S.L | chosen repaint index (see repaints) |
| `LastMovedDist` | 0 | — |  |
| `PAX_State` | 0 | — |  |
| `HeightOfSeat` | 0 | — |  |

#### Vehicle strings (7 names)

| Name | Uses in `.osc` | Accessor(s) | Meaning |
| :--- | ---: | :--- | :--- |
| `number` | 4,189 | L.$, S.$ |  |
| `yard` | 1,738 | L.$ |  |
| `act_busstop` | 978 | L.$, S.$ |  |
| `SetLineTo` | 544 | L.$, S.$ |  |
| `ident` | 142 | L.$, S.$ |  |
| `act_route` | 0 | — |  |
| `file_schedule` | 0 | — |  |

### Callbacks (called with `(M.V.name)`)

Callbacks take their arguments from the stack and leave a result on it; the exact argument lists are not in the shipped files. **Inferred signatures** for all 39 callbacks the vehicle scripts use (argument counts, result types, confidence) are in [Engine Callbacks: Inferred Signatures](callbacks.md) (2026-10-14). The names suggest groups: *terminus/bus-stop strings and indices* (`GetTerminusString`, `GetBusstopString`, `GetTerminusIndex`, …), *timetable queries* (`GetTT…`), *tickets and money* (`GetTicketName`, `GetTicketValue`, `GiveChangeCoin`), *passenger counts* (`GetHumanCountOnPathLink`, `GetHumanCountOnSeat`) and *geometry* (`GetHeightAbovePoint`).

#### Vehicle callbacks (25)

| Callback | Uses via `(M.V.name)` | Scripts |
| :--- | ---: | ---: |
| `GetTerminusString` | 4,891 | 621 |
| `GetTTBusstopIndex` | 4,013 | 330 |
| `GiveChangeCoin` | 3,486 | 126 |
| `GetTTBusstopName` | 3,362 | 247 |
| `GetBusstopString` | 2,953 | 271 |
| `NrSpecRandom` | 2,708 | 274 |
| `GetDepotStringGlobal` | 2,491 | 445 |
| `GetTerminusIndex` | 2,049 | 342 |
| `GetTerminusCode` | 2,004 | 422 |
| `GetTTBusstopCount` | 1,993 | 200 |
| `GetTTTerminusIndex` | 1,630 | 190 |
| `GetRouteBusstopIdent` | 1,530 | 161 |
| `GetBusstopIndex` | 1,430 | 261 |
| `GetTTLineString` | 1,412 | 216 |
| `GetRouteIndex` | 1,146 | 173 |
| `GetBusstopCount` | 1,072 | 170 |
| `GetTTDelay` | 865 | 205 |
| `GetTTBusstopDep` | 774 | 267 |
| `GetTicketValue` | 755 | 151 |
| `GetTicketName` | 730 | 147 |
| `GetHumanCountOnSeat` | 723 | 90 |
| `GetHumanCountOnPathLink` | 653 | 225 |
| `GetTTBusstopArr` | 437 | 117 |
| `GetRouteTerminusIndex` | 340 | 171 |
| `GetHeightAbovePoint` | 241 | 138 |

#### Scenery object callbacks (3)

| Callback | Uses via `(M.V.name)` | Scripts |
| :--- | ---: | ---: |
| `GetArrBusTerminus` | 437 | 130 |
| `GetArrBusTimeDiff` | 360 | 134 |
| `GetArrBusLine` | 355 | 128 |

#### Script texture callbacks (17)

| Callback | Uses via `(M.V.name)` | Scripts |
| :--- | ---: | ---: |
| `GetFontIndex` | 6,160 | 226 |
| `TextLength` | 4,296 | 224 |
| `STTextOut` | 2,576 | 208 |
| `STUnlock` | 1,481 | 239 |
| `STLock` | 1,465 | 239 |
| `STSetColor` | 1,311 | 237 |
| `STDrawRect` | 1,097 | 169 |
| `STFilter` | 1,045 | 238 |
| `STNewTex` | 980 | 219 |
| `STReadPixel` | 586 | 146 |
| `STGetR` | 482 | 146 |
| `STLoadTex` | 240 | 129 |
| `STGetG` | 104 | 52 |
| `STGetB` | 104 | 52 |
| `STDrawPixel` | 0 | 0 |
| `STCopyColor` | 0 | 0 |
| `STGetA` | 0 | 0 |

The **script-texture API** (17 callbacks: `STNewTex`, `STLock`, `STUnlock`, `STDrawRect`, `STTextOut`, `STLoadTex`, …) lets a script draw onto a texture that a `model.cfg` `[scripttexture]` entry declares (`GetFontIndex` and `TextLength` resolve fonts by name, see the font registry in the gap analysis); it explains why display scripts call `GetFontIndex` 6,160 times.

##### Verified Argument & Calling Conventions (M-0094)

| Callback | Arguments (bottom to top) | Result | Verified Semantics |
| :--- | :--- | :--- | :--- |
| `STNewTex` | `tex: f32` | – | Clears the canvas buffer to transparent black (0). |
| `STLock` | `tex: f32` | – | Locks canvas for drawing operations. |
| `STUnlock` | `tex: f32` | – | Unlocks canvas and flags GPU texture for re-upload. |
| `STSetColor` | `tex, A, R, G, B` (0–255) | – | Sets active brush color. In matrix displays, `A` is passed from the Red channel of logical textures to mask lit LEDs. |
| `STDrawRect` | `tex, x1, y1, x2, y2` | – | Fills axis-aligned rectangle from (x1, y1) to (x2, y2) with active color. |
| `STDrawPixel` | `tex, x, y` | – | Draws a single pixel with active color. |
| `STReadPixel` | `tex, x, y` | – | Samples pixel at (x, y) into internal read buffer. |
| `STGetR` | `tex: f32` | `R: f32` | Returns red channel (0.0..255.0) of last sampled pixel. |
| `STGetG` | `tex: f32` | `G: f32` | Returns green channel (0.0..255.0) of last sampled pixel. |
| `STGetB` | `tex: f32` | `B: f32` | Returns blue channel (0.0..255.0) of last sampled pixel. |
| `STGetA` | `tex: f32` | `A: f32` | Returns alpha channel (0.0..255.0) of last sampled pixel. |
| `STFilter` | `tex: f32` | – | Semantics ❓; OPTS treats it as a sampling-mode flag and leaves the pixels unchanged (a blur would compound across progressive refreshes). |
| `STLoadTex` | string: `path`, float: `tex` | `msg: str` | Loads texture file from disk (relative to vehicle or OMSI root). Returns empty string on success. |
| `STTextOut` | string: `text`, floats: `tex, x, y, font, mode, spacing` | – | Renders text string using `.oft` glyph cells, advancing pen by glyph width + spacing + font gap. |
| `STCopyColor`| `tex: f32` | – | Copies color state. |
| `GetFontIndex` | string: `font_name` | `idx: f32` | Resolves `Fonts/*.oft` font name (case-insensitive) to font index handle. |
| `TextLength` | string: `text`, float: `font_idx` | `width: f32` | Calculates total text width in pixels based on font glyph advances. |

### Controller axes (`gamectrl_axes.txt`)

Five axes: 0 steering wheel (force feedback), 1 throttle, 2 brake, 3 clutch, 4 combined throttle/brake.

## 🧪 Examples

Reading a predefined variable and a callback (descriptive, same grammar as real scripts):

```
(L.S.Timegap)                       // frame time: the most used predefined name, 50,122 uses
(L.L.Velocity) 3.6 *                // vehicle speed scaled
(M.V.GetTerminusString)             // callback; arguments from the stack
(T.L.ev_ibis_piep)                  // fire an event trigger
```

## ❓ Open Questions

| ID | Question | Priority |
| :--- | :--- | :--- |
| Q-PV01 | Argument lists and results of each callback (not documented in the install). Needs observation of call sites and a mutation test. | 🔴 High |
| Q-PV02 | Exact meaning and units of each predefined vehicle variable beyond the 📚 examples. | 🟡 Medium |
| Q-PV03 | Is the second letter `V` of `M.V` literally "vehicle"? What do scenery scripts use (they have 3 callbacks)? | 🟢 Low |
| Q-PV04 | The 14 unused predefined names: are they used by native code or plug-ins only? | 🟢 Low |
| Q-PV05 | Behaviour of a division by zero (`stack reset to 0`) in scripts that rely on it. | 🟡 Medium |

## 📝 Provenance

- **Files read in full (2026-10-04):** all ten `program/*.txt`.
- **Corpus scan:** every accessor `(<a>.<b>.<name>)` in all 5,821 `.osc` files was extracted and tallied; `{macro:…}` definitions were collected corpus-wide; each predefined name was counted per accessor.
- **Community wiki (📚):** script-system and system-variables pages (descriptions of ten well-known names; the register/stack equivalence). Facts restated.
- **Not used:** the shipped manuals contain no variable or callback reference.

**Clean-room attestation (ADR-0002):**
- No OMSI binaries were inspected.
- No GPL source code was read.
- No OMSI assets were committed.

## 🔗 Related Documents

- [`.osc`](osc.md), [variable lists](varnamelist.md), [constant files](const.md), [`model.cfg`](model_cfg.md)
- [Vehicle-side corpus audit](vehicle-corpus-audit.md), [Community cross-check](../../community-crosscheck.md), [Gap analysis](../../gap-analysis.md)
