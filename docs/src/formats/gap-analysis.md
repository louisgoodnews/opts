# 🕳️ Documentation Gap Analysis

| Metadata | Value |
| :--- | :--- |
| **Format Family** | Analysis (generated report) |
| **File Extensions** | all |
| **Parser Crate** | n/a |
| **Parser Module** | n/a |
| **Encoding** | n/a |
| **Status** | 🟡 Draft |
| **Last Updated** | 2026-10-04 |

---

## 📖 Overview

This page compares the OPTS specifications with the OMSI 2 installation and lists what is **not documented**, whether as a file type, a folder structure, or a mechanism that spans several files. Method: top-level and second-level folder census, per-container sub-folder census (all 512 vehicle folders, 41 maps, 154 scenery packs, 40 ticket packs, 72 spline packs, 24 money packs, 11 human packs), file-name census, and a search of the docs for every folder or file name. Evidence tags: **[C]** counted over the whole install, **[H]** seen in files. No OMSI binary was inspected.

**What is well covered:** the file *grammars* of meshes, textures, `.sco`/`.sli`, vehicle configs, `.hof`, `.osc`, varlists/constfiles, map tiles and map-level lists, timetables, and most small extension types (see the [formats overview](overview.md) and [inventory](inventory.md)). **What is thin:** the *systems* that tie files together (repaints, scripting infrastructure, ticket/voice packs, fonts, weather/scenario folders) and the engine-level configuration files.

## 🔤 General Structure

### Coverage by installation folder

✅ specified · ◐ grammar of some files only, folder meaning missing · ❌ nothing beyond the inventory · 🛠 tool or add-on material (out of scope)

| Folder / file | Files | Status | Note |
| :--- | ---: | :---: | :--- |
| `Vehicles/` | 246,465 | ✅ ◐ | formats ✅; **repaint system and script-set organisation** ◐ ([§1](#1-repaint-system-cti-ctc-templates), [§2](#2-vehicle-and-scenery-script-sets)) |
| `Sceneryobjects/`, `Splines/` | 221,209 / 30,059 | ✅ ◐ | formats ✅; **scenery script library** ◐ ([§2](#2-vehicle-and-scenery-script-sets)) |
| `maps/` | 102,693 | ✅ | formats ✅; chrono composition and map variants in [chrono](map/chrono.md) (M-0012) |
| `program/` | 10 | ✅ | specified in [predefined variables](text/vehicles/predefined-variables.md) (M-0008) |
| `Scripts/` | 32 | ✅ | covered in [script sets](text/vehicles/script-sets.md) (M-0008) |
| `TicketPacks/` | 12,465 | ✅ | [ticket packs](text/ticketpacks.md) (M-0011) |
| `Fonts/` | 2,095 | ✅ | [fonts](text/fonts.md) (M-0011) |
| `Weather/` | 56 | ✅ | [environment](text/environment.md) (M-0010) |
| `Situations/`, `Tutorials/` | 91 / 1,466 | ✅ | [environment](text/environment.md) (M-0010) |
| `Money/` | 334 | ✅ | [money](text/money.md) (M-0011) |
| `Inputs/`, `envir.cfg`, `options.cfg` | 5 + 2 | ✅ | [engine config](text/engine-config.md), [environment](text/environment.md) (M-0010) |
| `plugins/` | 2 | ✅ | [`.opl`](text/opl.md) (M-0010); the DLL is deliberately not analysed |
| `Humans/`, `Drivers/` | 463 / 2 | ✅ ◐ | `.hum`, `.odr` ✅/◐ |
| `Languages/`, `RegAddons/`, `Trains/`, `Sounds/`, `helper/`, `template/`, `option_presets/` | — | ✅ ◐ | covered by [misc types](text/misc-types.md), [wav](wav.md), [lists](map/lists.md), stubs |
| `docs/` (20 PDFs), root `Readme_Liesmich_*.txt` ×9, `Change-Log_Patch_1.1_EN.txt`, `KI-Pack-1-Handbuch.pdf` | — | ◐ | manual reviewed (M-0008): gameplay only, facts recorded in the [cross-check](community-crosscheck.md#shipped-manuals-m-0008-review-2026-10-04); timetable PDFs and the AI-pack handbook not read ([§11](#11-official-documents-shipped-in-the-install)) |
| `Addons/`, `OMSI-Tools/`, `OmniNavigation/`, `DEMImport/`, `SDK/`, `Repaints/`, `_Straßenbahn/`, `GUI/`, `Screenshots/` | — | 🛠 | add-on documents, installers, a Java navigation app, an SRTM height-data importer, repaint tool, authoring files, splash bitmaps |

---

## 📚 Field Reference — the gaps

### 1. Repaint system (`.cti`, `[CTC]`, templates)

> ✅ **Closed by M-0009** — see [repaints](text/repaints.md).

**What exists in the install [C]:** 3,152 `.cti` files (2,978 in `Vehicles/*/Texture/…`, 22 in `Humans/*/texture`, ~50 in scenery), holding ≈ 22,500 `[item]` blocks (median 3 per file, one file has 684), **58,602 `[setvar]`** and 73 `[visible]` lines; 2,949 distinct repaint names; 896 folders hold `.cti` files (median 1 per folder, maximum 42). The model configs define the hooks: `[CTC]` ×1,513 and `[CTCTexture]` ×10,453 in `.cfg`, ×232 / ×243 in `.sco`.

**What the docs say:** the `.cti` grammar ([misc types](text/misc-types.md#cti)), a one-line `[CTC]`/`[CTCTexture]` description ([model_cfg](text/vehicles/model_cfg.md), [sco](text/sco.md)); `model_cfg` Q-011 asks what `[CTC]`'s parameters are.

**Missing:**
- The **whole chain**: `[CTC] <name>, <texture folder>, <default index>` (e.g. `Colorscheme`, `Texture\Werbung_EN92`, `0`) → OMSI scans that folder for `*.cti` → each `[item]` names a *repaint* and fills a **slot** (`farbschema_tex1`…`_tex9`, `farbschema_trans`, `Wagenkasten`, `WerbungInnen`…) that a `[CTCTexture] <slot>, <default texture>` entry in `model.cfg` declares → `[setvar]` pairs set script variables when the repaint is chosen. The predefined variable `Colorscheme` ([§3](#3-program-omsis-own-reference-lists)) carries the choice.
- **Conventions**: slot naming, the "one repaint = several items sharing a name" rule, default-index semantics, disabling a file by renaming it `.off` ([off](off.md)).
- **Assignment**: repaint names also appear in the ailist depot rows ([lists](map/lists.md)) and in vehicle selection; the link is unspecified.
- **Authoring side**: `Vehicles/<v>/templates/` (png/bmp/pdf/`.rpc`, 4 vehicles), `SDK/RepaintTool` (`.rpc` ×65 + template bitmaps), top-level `Repaints/` (`.pdn`).
- **Proposed spec:** `formats/text/repaints.md` (🟡 Medium; needs only text analysis, ~1 session).

### 2. Vehicle and scenery script sets

> ✅ **Closed by M-0008** — see [script sets](text/vehicles/script-sets.md) and [scenery scripts](text/scenery-scripts.md).

**What exists [C]:** 4,783 `.osc` in 388 vehicles (median 2, maximum 135 per vehicle), including 279 in `script-g` folders; each script usually has a companion `<name>_varlist.txt` (and sometimes `_constfile.txt`, `_stringvarlist.txt`): 7,266 `.txt` in `Vehicles/*/script`. A standard set of **subsystem scripts recurs** by file name: `lights.osc` 129 vehicles, `bremse.osc` 122, `main_ai.osc` 94, `antrieb.osc` 86, `auspuff.osc` 85, `rain.osc`/`dirt.osc`/`forcefeedback.osc`/`klappern.osc`/`sound_volume.osc` 84 each, `elec.osc` 83, `collision.osc` 82, `spray.osc` 82, `heizung.osc` 81, `wiper.osc` 81, `cashdesk.osc` 78, `main.osc` 78, `engine.osc` 76, `cockpit.osc` 67, `articulation.osc` 64, `vmatrix.osc` 60, `ticketprinter.osc` 52, `ibis-2.osc` 50, `door.osc` 43, `terminus.osc` 38. **Shared AI scripts** live in the global `Scripts/AI_Cars/` (HH3/HH4 `main_AI`, `collision_hupe`, `polizei_LED`, `taxi` + varlists) and are referenced by 353 `.ovh`/`.bus`. **Scenery scripts**: 1,020 `.osc` in 92 packs (`ampel1`…`ampel5` traffic-light controllers, `werbung`, `busstopdisplay`, `clock`/`uhr`, `people_standing`, `sound_varmapping`, `visibleflag_sommerhalbjahr`).

**What the docs say:** the language ([osc](text/vehicles/osc.md)), token census ([vehicle audit](text/vehicles/vehicle-corpus-audit.md)), the per-file varlist/const grammar, and the `[script]` keyword in `.bus`.

**Missing:** a description of the **script set as an architecture** — what each standard subsystem script owns, which variables connect them (the shared vocabulary in `*_varlist.txt`), the load order given by `[script]`, the `script`/`script-g`/`script_ki` folder variants, and the global `Scripts/AI_Cars/` library. Also a catalogue of the **scenery script library** and the contract between an `ampel*.osc` and `.sco` `[trafficlight]`/phase data.
- **Written:** [`script-sets.md`](text/vehicles/script-sets.md) and [`scenery-scripts.md`](text/scenery-scripts.md).

### 3. `program/` — OMSI's own reference lists

> ✅ **Closed by M-0008** — see [predefined variables](text/vehicles/predefined-variables.md).

**What exists [C][H]:** ten small plain-text files shipped with the game:

| File | Lines | Content |
| :--- | ---: | :--- |
| `Info_scriptcode.txt` | 90 | a German description of the script language: two 8-slot float stacks (a permanent and an additional one, accessed by `l0…l7`/`s0…s7`), operators and operands, and the accessor syntax `(<L/X/T/S>.<L/M/S>.name)` — first letter *load float / load string / sound trigger / save*, second letter *local / map / system* |
| `varlist_roadvehicle.txt` | 138 | the **predefined vehicle variables** (`Refresh_Strings`, `Envir_Brightness`, `StreetCond`, `Spot_Select`, `Colorscheme`, `M_Wheel`, `n_Wheel`, `Throttle`, `Brake`, `Clutch`, `Velocity`, …) |
| `varlist_system.txt` | 22 | system variables (`Timegap`, `GetTime`, `Pause`, `Time`, `Day`, `Month`, `Year`, `DayOfYear`, `mouse_x/y`, `PrecipType`, `PrecipRate`, `coll_*`, `Weather_Temperature`, `SunAlt`, …) |
| `varlist_scenobj.txt` / `varlist_human.txt` | 9 / 4 | scenery (`NightlightA`, `InUse`, `TrafficLightPhase`, `TrafficLightApproach`, `Signal`, `NextSignal`, `Switch`, …) and human variables (`PAX_State`, `HeightOfSeat`, …) |
| `stringvarlist_roadvehicle.txt` | 7 | `ident`, `number`, `act_route`, `act_busstop`, `SetLineTo`, `yard`, `file_schedule` |
| `callbacklist_roadvehicle.txt` | 25 | engine **callbacks** scripts can call: `GetTerminusString`, `GetBusstopString`, `GetRouteIndex`, `GetTTDelay`, `GetTTBusstopDep`, `GetTicketName`, `GetTicketValue`, `GiveChangeCoin`, `GetHumanCountOnPathLink`, `GetHeightAbovePoint`, … |
| `callbacklist_scenobj.txt` | 3 | `GetArrBusLine`, `GetArrBusTerminus`, `GetArrBusTimeDiff` |
| `callbacklist_scripttex.txt` | 17 | the **script-texture drawing API**: `STNewTex`, `STLock`, `STDrawRect`, `STTextOut`, `STLoadTex`, `STSetColor`, `GetFontIndex`, `TextLength`, … (for `[scripttexture]` in `model.cfg`) |
| `gamectrl_axes.txt` | 9 | the five controller axes (steering/force feedback, throttle, brake, clutch, combined) |

**What the docs say:** nothing (the docs mention only `GiveChangeCoin`, `GetTerminusString`/`GetTerminusIndex` and a few variables from scripts). The corpus accessor `M.V.<callback>` used for callbacks is **not** in the in-file legend, which lists only the L/X/T/S × L/M/S letters ❓.
- **Written:** [`predefined-variables.md`](text/vehicles/predefined-variables.md).

### 4. Chrono folders

> ✅ **Closed by M-0012** — see [chrono](map/chrono.md).

**What exists [C]:** 455 chrono folders in 19 of 41 maps. Contents (all folders): overlay tiles `tile_X_Y.map` 1,526, `Chrono.cfg` 427, `Chrono_<LANG>.dsc` 393, a `TTData` sub-folder in 452, **`unsched_trafficdens.txt` 163** (a per-chrono traffic-density override), `ailists_#upd.cfg` 13, `parklist_p*.txt` and `registrations.txt` (1 each), and 18 `tile_X_Y.map.chronoterrrelevant` marker files (UTF-16 text containing the word *Dummy*, i.e. presence is the signal ❓ — probably "this overlay changes terrain"). Some chrono folders contain nested date-named sub-folders (e.g. `02.11.25 - 14.00 Uhr`, `Messependel 19.11. - 23.11.25`).

**What the docs say:** `Chrono.cfg` keywords, overlay tile blocks, the Type-1/Type-2 trip handling ([timetables](map/timetables.md), [tiles](map/tiles.md)).

**Missing:** the **composition rules** of a chrono folder (which override files are allowed: traffic density, AI lists, parked/registration lists), the meaning of the `.chronoterrrelevant` marker, nested date folders, and how overlapping periods are resolved. Also the other map-level variant folders: `Fahrgastaufkommen` (`.cmd` + alternative `global.cfg` per scenario, 3 maps), `Alternative AI-Listen` (6 variants), `car_use`, `menschen`, `services`, backups.
- **Proposed spec:** extend `timetables.md` or add `formats/map/chrono.md` (🟡 Medium).

### 5. TicketPacks

> ✅ **Closed by M-0011** — see [ticket packs](text/ticketpacks.md).

**What exists [C]:** 40 packs, 56 `.otp`, 11,842 voice WAVs (4.2 GB), 470 ticket meshes. A pack folder holds `<Pack>.otp`, **ticket meshes `Ticket_<n>_single.o3d` and `Ticket_<n>_block.o3d`** for ticket type *n*, ticket textures (`Fahrscheine_*.bmp/jpg`), and **voice folders named by voice id**: `F1`–`F4`, `FY1`, `FO1`, `M1`–`M6`, `MY1` (31 packs have the full set; reading *F* female, *M* male, *Y* young, *O* old ❓). Each voice folder has ≈ 1,050 WAVs with a shared **utterance vocabulary of 76 name patterns**: `ticket_<n>_<m>` (2,169), `pardon_<n>` (989), `hello_<n>` (700), `toodark/toocold/toohot/toolate_<n>` (≈ 690 each), `goodmorning_<n>`/`goodevening_<n>` (347), `thanks`, `toowet`, `toobad_a/b/c`, `pardonticket`, `badchange`, `badticket_a/b`, `toofew_a/b`, `toomuch_a/b`.

**What the docs say:** the `.otp` grammar and `[voice]` in `.hum` ([misc types](text/misc-types.md)); [wav](wav.md) lists the voice files in its inventory.

**Missing:** pack layout, the ticket-mesh naming rule, the voice-id convention, and the **event → file-name vocabulary** (which situation plays `toocold_3`, what `ticket_2_5` means, how a human's `[voice]` selects a folder and how a number is chosen).
- **Proposed spec:** `formats/text/ticketpacks.md` (🟡 Medium; mostly name analysis plus a mutation test).

### 6. Fonts

> ✅ **Closed by M-0011** — see [fonts](text/fonts.md).

**What exists [C]:** `Fonts/` holds 2,095 files (1,055 `.oft`, 1,021 bitmaps) in a flat layout plus sub-folders `St-Servan` and `Roboto`; 1,094 distinct font names are defined by `[newfont]`. Of 32,373 `[texttexture]`/`[texttexture_enh]` font-name lines in `.sco`/`.cfg`, **31,672 (97.8 %) match a defined `[newfont]` name** — fonts are looked up **by name**, not by path.

**What the docs say:** the `.oft` grammar and the alpha-bitmap rule ([misc types](text/misc-types.md)).

**Missing:** the folder as a **name registry** (collision behaviour when two `.oft` define the same name, which folders are scanned), naming conventions (`NN_<name>.oft` + `<name>.bmp` + `<name>_alpha.bmp`), and the link to `GetFontIndex`/`TextLength`/`STTextOut` of the script-texture API ([§3](#3-program-omsis-own-reference-lists)).
- **Proposed spec:** a section in `misc-types.md` or `formats/text/fonts.md` (🟢 Low–Medium).

### 7. Weather, situations, environment

> ✅ **Closed by M-0010** — see [environment](text/environment.md).

**Weather [C]:** 9 presets `<Name>.owt` + 45 `<Name>_<LANG>.dsc` descriptions (5 languages), **`clouds.cfg`** (`[cloudtype]` ×4: name, texture, size in metres, `ovc`/`sct` cover class) and **`ICAO.txt`** (105 airport lines `ICAO - Name`, used by the `[currWeather_ICAO]` option). Docs: `.owt` field list only.

**Situations [C]:** 29 authored scenarios `.osn` with `.osn.owt` (28), `_<LANG>.dsc` (20) and preview mosaics `<name>.osn_<a>_<b>.dds` (14). They use the same blocks as `laststn.osn` plus `[tt_active]`/`[settimetable]` (22 of 29). `Tutorials/` holds 4 `.osn` and 1,160 HTML pages ([html](html.md)).

**Environment:** root `envir.cfg` (215 lines): `[sky_textures]` (day, dusk, night), `[twilight_start_end]`, `[lightcolor_a/b/c]` tables (63/63/15 lines) ❌.

- **Proposed spec:** `formats/text/environment.md` covering `envir.cfg`, `clouds.cfg`, `ICAO.txt`, the Weather folder and the scenario folder layout (🟡 Medium).

### 8. Money

> ✅ **Closed by M-0011** — see [money](text/money.md).

**What exists [C]:** 24 packs, 55 `.cfg`, 246 coin/note meshes, textures. The configs are keyword files — `[currency]` (name, decimals) ×55, `[coin]` ×464 and `[bill]` ×134 (each: mesh file, value) — with an in-file explanation, and year variants (`DM.cfg`, `DM_91.cfg`, …). Referenced by `[moneysystem]` in `global.cfg`/`Chrono.cfg` and by the change-giving callback `GiveChangeCoin` ([§3](#3-program-omsis-own-reference-lists)).

**Missing:** the format, the pack layout, and how a chrono switches money systems.
- **Proposed spec:** `formats/text/money.md` (🟢 Low effort; ~½ session).

### 9. Engine-level configuration

> ✅ **Closed by M-0010** — see [engine config](text/engine-config.md).

| File | Content seen | Docs |
| :--- | :--- | :--- |
| `Inputs/keyboard.cfg` (656 lines) | 130 `[entry]` blocks: action name, key code, a third value (modifier ❓) under `[game]`/`[vehicles]` | only mentioned by [kyb](kyb.md) |
| `Inputs/gamectrler.cfg` (189 lines) | per controller: `[ctrl]`, `[axis]` (16 values), `[buttons]`, `[ffscale]` (×4 controllers) | ❌ |
| `envir.cfg` | see §7 | ❌ |
| `options.cfg`, `option_presets/*.oop` | same keywords | ◐ keyword list |

- **Proposed spec:** one `formats/text/engine-config.md` (🟢 Low).

### 10. Plug-in declarations (`.opl`)

> ✅ **Closed by M-0010** — see [opl](text/opl.md).

`plugins/GPM.opl` is a **text file** declaring what a native plug-in exposes: `[dll]` (file name), `[varlist]` (11 names), `[stringvarlist]` (149 names, all `INEO_…`), `[systemvarlist]` (4 names). The plug-in's DLL must not be analysed (policy), but the `.opl` text — the variables a vehicle script can reach through the plug-in — is documentable. No other plug-in exists in this install.
- **Proposed spec:** a short `formats/text/opl.md` (🟢 Low).

### 11. Official documents shipped in the install

`docs/` contains the OMSI 2 manual (DE/EN/PL), a quick-reference card, a poster and timetable PDFs; the root holds nine release-note files and `KI-Pack-1-Handbuch.pdf`. The release notes are patch histories and contain **no keyword or format descriptions** (checked: zero bracketed keywords). The manuals are user-level; whether they contain format facts is **unknown** — they are copyrighted shipped documentation, so reading them is a policy decision for the project lead rather than something this pass did.

---

## 🧪 Examples

How the repaint chain reads in real files (paraphrased from three files; no OMSI text copied):

```
model.cfg:   [CTC] Colorscheme, Texture\Werbung_EN92, default 0
             [CTCTexture] farbschema_tex1 -> EN92_1.tga
Texture\Werbung_EN92\<name>.cti:   [item] <repaint name> / farbschema_tex1 / EN92_1_<name>.dds
                                   [setvar] <variable> / <value>
```

## ❓ Open Questions

| ID | Question | Priority |
| :--- | :--- | :--- |
| Q-GAP01 | ~~Which of the proposed specs does the project lead want turned into milestones?~~ — all five proposals approved 2026-10-04 and executed as M-0008–M-0012. | ✅ |
| Q-GAP02 | ~~May the shipped manuals be read?~~ — yes, approved 2026-10-04; reviewed in M-0008. | ✅ |
| Q-GAP03 | Is `M.V.<name>` the accessor for engine callbacks, given that it is missing from the legend in `Info_scriptcode.txt`? | 🟡 Medium |
| Q-GAP04 | What does the `.chronoterrrelevant` marker change? Narrowed in [chrono](map/chrono.md) (Q-CH01); needs a test. | 🟢 Low |

## Proposed work — status: all five proposals executed (M-0008–M-0012, 2026-10-04)

| # | Proposal | Specs | Priority | Effort |
| ---: | :--- | :--- | :--- | :--- |
| 1 | **Script reference** | `predefined-variables.md` (§3), `script-sets.md`, `scenery-scripts.md` (§2) | 🔴 | 3–4 sessions |
| 2 | **Repaint system** | `repaints.md` (§1) | 🟡 | 1 session |
| 3 | **Engine & environment configs** | `environment.md` (§7), `engine-config.md` (§9), `opl.md` (§10) | 🟡 | 2 sessions |
| 4 | **Passenger content** | `ticketpacks.md` (§5), `money.md` (§8), fonts (§6) | 🟡 | 2 sessions |
| 5 | **Chrono and map variants** | `chrono.md` (§4) | 🟡 | 1 session |

Per the project's rules these become roadmap items only through a P-002/P-006 cycle.

## 📝 Provenance

- **Scans (2026-10-04):** folder and file census of the whole installation; sub-folder census of all vehicle, map, scenery, ticket-pack, human, spline and money packs; `.cti` content census (3,152 files); `[CTC]`/`[CTCTexture]` census in `.cfg`/`.sco`; `.osc` name census for vehicles and scenery; chrono-folder composition census; ticket-pack voice-file name census (normalised patterns); font-name resolution check (`[newfont]` vs `[texttexture]`); keyword census of `envir.cfg`, `keyboard.cfg`, `gamectrler.cfg`, `clouds.cfg`, 55 money configs and 29 `.osn`; read of the ten `program/*.txt` files and `plugins/GPM.opl` (plain text); a search of all docs for each folder and file name.
- **Not opened:** all executables and DLLs (including `plugins/GPM.dll`); the PDFs.
- **Not read for this page:** the release notes were scanned for bracketed keywords only.

**Clean-room attestation (ADR-0002):**
- No OMSI binaries were inspected.
- No GPL source code was read.
- No OMSI assets were committed.

## 🔗 Related Documents

- [Inventory](inventory.md), [Formats overview](overview.md), [Community Cross-Check](community-crosscheck.md)
- [`.osc`](text/vehicles/osc.md), [model.cfg](text/vehicles/model_cfg.md), [timetables](map/timetables.md), [misc types](text/misc-types.md)
