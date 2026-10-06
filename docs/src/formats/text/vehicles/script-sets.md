# Vehicle Script Sets — Architecture and Conventions

| Metadata | Value |
| :--- | :--- |
| **Format Family** | Convention / architecture (text files) |
| **File Extensions** | `.osc`, `*_varlist.txt`, `*_constfile.txt`, `*_stringvarlist.txt` |
| **Parser Crate** | `opts-core` (loader that assembles a script set) |
| **Parser Module** | `opts_core::formats::osc::set` |
| **Encoding** | CP1252 / UTF-8 / UTF-16LE (per file) |
| **Status** | 🟡 Draft — statistics corpus-verified; per-script semantics inferred |
| **Last Updated** | 2026-10-04 |

---

## 📖 Overview

A vehicle does not have "a script": it has a **set** of up to a few dozen small scripts, each owning one subsystem (lights, brakes, drivetrain, doors, display matrix, …), plus companion variable and constant lists. The [`.osc` spec](osc.md) describes the language; this page describes how real vehicles **organise** their scripts, so that OPTS can load a whole set and wire the subsystems together. The data comes from all 2,537 `.bus`/`.ovh` vehicle definitions and 4,783 vehicle `.osc` files (plus the shared `Scripts/AI_Cars` library); scenery scripts are in [scenery-scripts](../scenery-scripts.md).

Evidence tags: **[C]** counted over the corpus, **[H]** seen in files, 📚 community wiki, ❓ inferred. No behaviour was tested in OMSI.

## 🔤 General Structure

### Files of a set

```
Vehicles/<vehicle>/
  <vehicle>.bus | .ovh        [script] N + N script paths, [varnamelist] N + …, [constfile] N + …, [stringvarnamelist] N + …
  script/                      (477 vehicles; variants: script-g ×5, script_ki ×2, script_117l ×1)
    main.osc  lights.osc  bremse.osc  antrieb.osc …
    <script>_varlist.txt  <script>_constfile.txt  <script>_stringvarlist.txt
```

[C] Size of the lists a definition declares (counts as the first line of each block):

| Block | Definitions with it | Median | 90th pct | Max |
| :--- | ---: | ---: | ---: | ---: |
| `[script]` | 2,259 | 4 | 30 | 36 |
| `[varnamelist]` | 2,524 | 16 | 25 | 33 |
| `[constfile]` | 2,236 | 2 | 19 | 26 |
| `[stringvarnamelist]` | 1,614 | 4 | 7 | 14 |

The medians hide two populations: **`.bus` (player buses)** list a median of **26** scripts (90th percentile 30), **`.ovh` (AI and other vehicles)** a median of **1** (90th percentile 4, maximum 6). A player bus is a full subsystem set; an AI vehicle is usually one "main AI" script.

### Order and the main script

The `[script]` list is an ordered list of paths. The **first** entry is a "main" script in most definitions: `main.osc` (504 definitions), `main_ai.osc` (370), then vehicle-specific mains (`ivecw_main.osc` 99, `br132.osc` 92, `bm238.osc` 67, `a3l_82.osc` 57, `typu.osc` 52, `br474.osc` 40, `main_ai_police_fd.osc` 39, `br243.osc` 38). A main script is small (`main.osc` median 31 lines) and mostly writes debug variables and fires the shared `collision` / `malfunction_reset` triggers. Whether list order affects execution order is ❓ ([Q-SS01](#-open-questions)).

### Companion lists

Each script normally has a `<script>_varlist.txt` declaring the variables it introduces; many have a `_constfile.txt` (tuning values and curves) and some a `_stringvarlist.txt`. The definition (`.bus`/`.ovh`) lists them all — scripts do not name their own lists. A shared vocabulary of variable names ties the scripts together: a subsystem writes its outputs with `(S.L.name)` and other subsystems read them with `(L.L.name)` ([predefined variables](predefined-variables.md#accessor-syntax-as-used-in-the-corpus-c)).

### Header convention

Scripts open with a comment banner. Over all 5,821 scripts [C]: banner line of `#` characters 4,494; a copyright line `'(c) …` 3,695; `Script Version` 3,611; `Omsi release` 3,453; a revision history 3,253; and a **`Needs:` list in 3,133 (54 %)**. The `Needs:` lines name the subsystems a script depends on, so the dependency graph can be read from the headers. Most-cited needs: `engine` 962, `none` 913, `elec` 639, `antrieb` 557, `bremse` 536, `cockpit` 517, `lights` 253, `door`/`doors` 437, `heizung` 169, `ibis` 136, `wiper` 129, `rain` 106. So the electrical system (`elec`) and engine sit at the bottom of the graph and nearly everything else needs them ❓ (read from headers, not tested).

## 📚 Field Reference

### The standard subsystem scripts

Vehicles (player and AI) share a **common library of subsystem scripts by file name**. Count = vehicles that ship a script with this name; "owns" lists the variables it writes most often and the macros it defines most often [C] (variable and macro names are facts; meaning is inferred from names ❓).

| Script | Vehicles | Median lines | Owns (written variables) | Typical macros / triggers |
| :--- | ---: | ---: | :--- | :--- |
| `lights.osc` | 129 | 538 | `lights_sw_blinker`, `lights_abbl`, `lights_blinkgeber`, `spot_select` | macros `lights_init`/`lights_frame`; triggers `kw_scheinwerfer_toggle`, `blinker_left_move` |
| `bremse.osc` | 122 | 1,263 | `bremse_p_tank01…04`, `bremse_feststell` | `bremse_init`/`_frame`/`_kompressor`; triggers `parking_brake_toggle/_set/_release` |
| `main_ai.osc` | 94 | 71 | `exhaust_steam_freq`, `tire_wet_freq`, `m_wheel`, `brakeforce` | AI vehicles' single script |
| `antrieb.osc` | 86 | 240 | `antrieb_getr_aktugang`, `m_wheel`, `antrieb_schaltmoment`, `antrieb_getr_gang` | `antrieb_init`/`_frame`, `calc_eng_m`; triggers `kw_s_n`, `kw_s_1` |
| `auspuff.osc` | 85 | 27 | `auspuff_leben`, `auspuff_vel`, `auspuff_freq`, `auspuff_alpha` | `auspuff_frame` |
| `rain.osc` | 85 | 40 | `rain_window_*_wetness` | `rain_frame` |
| `dirt.osc` | 84 | 36 | `dirt_wiped`, `dirt_norm`, `dirt_reflektor_*` | `dirt_frame`; trigger `veh_wash` |
| `forcefeedback.osc` | 84 | 45 | `ff_vib_period`, `ff_vib_amp` | `ff_frame` |
| `klappern.osc` | 84 | 37 | `klappern_vol`, `klappern_lasta` | `klappern_frame` (rattle sounds) |
| `sound_volume.osc` | 84 | 32 | `snd_outsidevol` | `sound_volume_frame` |
| `elec.osc` | 83 | 188 | `elec_i_battery`, `elec_battery_load`, `elec_v_battery` | `elec_init`/`_frame` |
| `collision.osc` | 82 | 122 | `collision_energy`, `elec_failure_general`, `antrieb_failure_general` | `collision_init`; trigger `collision` |
| `spray.osc` | 82 | 15 | `tire_wet_freq`, `tire_wet_live` | `spray_frame` |
| `heizung.osc` | 81 | 430 | `cabinair_temp`, `fanheater_t_target` | `heizung_init`/`_frame` |
| `wiper.osc` | 81 | 213 | `wiperrunning`, `wiperrun`, `wiper_intervall_sw` | `wiper_init`/`_frame` |
| `cashdesk.osc` | 78 | 356 | `cashdesk_changetimer1…`, change-giving state | `cashdesk_init`/`_frame`; triggers `cashdesk_changer_0_05`… |
| `main.osc` | 78 | 31 | `debug_0…3` | triggers `collision`, `malfunction_reset` |
| `engine.osc` | 76 | 565 | `engine_internal_throttle`, `engine_injection_on`, `engine_m` | `engine_init`/`_frame`, `engine_moment`; triggers `veh_tank`, `kw_m_enginestart` |
| `cockpit.osc` | 67 | 2,494 | switch-state variables (`cockpit_wischer_drehschalter`, gear selection) | `cockpit_init`, `traegheit`; triggers `automatic_r/_d` |
| `articulation.osc` | 64 | 76 | `articulation_0_alpha_volume`, `articulation_knickschutz_warning` | `articulation_init`/`_frame` |
| `vmatrix.osc` | 60 | 4,940 | `matrix_fontsel1…4`, matrix strings | `matrix_init`, `matrix_setsteckschild`; triggers `matrix_seitenschildklemme_*` |
| `ticketprinter.osc` | 52 | 372 | `ticketprinter_ticket_selection`, `_pos`, `_druckt` | `ticketprinter_init`/`_frame` |
| `door.osc` | 43 | 2,091 | `doorspeed_0…`, `door_0…` | `door_init`/`_frame`, `door0_calc`; triggers `bus_doorfront0` |
| `terminus.osc` | 38 | 27 | `refresh_strings`, `target_index_int` | `settarget_matrix`; trigger `ai_scheduled_settarget` |

Pattern: most subsystem scripts expose **`<name>_init`** and **`<name>_frame`** macros that the shared main/`{init}`/`{frame}` blocks call (helper macro `traegheit` ("inertia") is defined 566 times across the corpus). The long scripts are the display matrix (`vmatrix`, median 4,940 lines), cockpit logic (2,494), doors (2,091) and brakes (1,263).

### Shared AI scripts — `Scripts/AI_Cars/`

32 files shared across vehicle packs: generic `main_AI.osc`, `main_AI_lkw.osc`, `main_AI_police.osc`, `main_AI_police_FD.osc`, `collision_hupe.osc`/`collision_horn.osc` and their varlists, plus city-specific families (`HH3_*`, `HH4_*` for Hamburg: `main_AI`, `collision_hupe`, `polizei_LED`, `taxi`; `TH_*` for the Thuringia packs: `TH_main_AI`, `TH_main_AI_lkw`, `TH_Trabant_AI`, `TH_lights_varlist.txt`) and shared lists (`AI_varlist.txt`, `AI_varlist_lkw.txt`, `lights_varlist.txt`, `police_varlist.txt`). **353** `.ovh`/`.bus` definitions reference this folder (path relative to the OMSI root, e.g. `Scripts\AI_Cars\main_AI.osc`) — so a script path can leave the vehicle folder and point to a global library.

### Folder variants

| Folder | Vehicles | Remark |
| :--- | ---: | :--- |
| `script` | 477 | the norm |
| `script-g` | 5 | alternative set (carries 279 `.osc` + 391 `.txt`); likely a variant for another vehicle type or generation ❓ |
| `script_ki` | 2 | AI-specific scripts |
| `script_117l` | 1 | vehicle-variant specific |

## 🧪 Examples

Skeleton of a player bus's definition (descriptive; not an OMSI file):

```
[script]
26
script\main.osc
script\engine.osc
script\elec.osc
…                       (lights, brakes, doors, display matrix, ticket printer, …)
[varnamelist]
16
script\engine_varlist.txt
…
```

## ❓ Open Questions

| ID | Question | Priority |
| :--- | :--- | :--- |
| Q-SS01 | Does the order of the `[script]` list matter (execution order of `{frame}` blocks, macro visibility)? Needs a mutation test. | 🔴 High |
| Q-SS02 | Are macros global across all scripts of one vehicle (e.g. `engine_frame` called from `main.osc`)? 99.9 % of `M.L` calls resolve to a macro defined *somewhere* in the corpus; resolution within one vehicle is unchecked. | 🔴 High |
| Q-SS03 | What does `script-g` stand for and when is it selected? | 🟢 Low |
| Q-SS04 | Are the `Needs:` headers authoritative or merely documentation? | 🟢 Low |
| Q-SS05 | Per-script variable ownership: the table lists the most frequent writes only; a complete producer/consumer map would need a per-vehicle analysis. | 🟡 Medium |

## 📝 Provenance

- **Corpus (2026-10-04):** `[script]`/`[varnamelist]`/`[constfile]`/`[stringvarnamelist]` blocks of all 2,537 vehicle `.bus`/`.ovh`; header-line census of all 5,821 `.osc`; per-script census (written variables `S.L`, macros, triggers) for 24 standard script names in `Vehicles/*`; folder census of `Vehicles/*/script*`; file list of `Scripts/AI_Cars`.
- **Community wiki (📚):** vehicle-folder page (script files, varlist registry); the meaning of "on-demand" variables.
- **Not read:** per-script logic beyond samples; the manuals contain no script-set description.

**Clean-room attestation (ADR-0002):**
- No OMSI binaries were inspected.
- No GPL source code was read.
- No OMSI assets were committed.

## 🔗 Related Documents

- [`.osc`](osc.md), [predefined variables](predefined-variables.md), [variable lists](varnamelist.md), [constant files](const.md), [`.bus`](bus.md), [misc types](../misc-types.md#ovh)
- [Scenery scripts](../scenery-scripts.md), [Vehicle-side audit](vehicle-corpus-audit.md)
