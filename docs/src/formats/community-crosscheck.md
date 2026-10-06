# Community Documentation Cross-Check

| Metadata | Value |
| :--- | :--- |
| **Format Family** | n/a (meta page: reconciles specs with public community documentation) |
| **File Extensions** | all |
| **Parser Crate** | `opts-core` |
| **Parser Module** | n/a |
| **Encoding** | n/a |
| **Status** | 🟡 Draft |
| **Last Updated** | 2026-10-04 |

---

## 📖 Overview

The OMSI community maintains a public **file-format wiki** ("Dateiformate", `https://reboot.omsi-webdisk.de/wiki/entry/13-dateiformate/` and its sub-pages). [ADR-0002](../adr/0002-clean-room-policy.md) allows reading public documentation as long as every fact is attributed. This page records what that wiki adds to, confirms in, or contradicts in the OPTS specs, which were derived independently from the installed files.

### How the wiki was used

| Rule | Why |
| :--- | :--- |
| **Facts only, own words.** Nothing is copied; no sentences, tables or examples are reproduced. | The wiki is licensed **CC BY-SA 4.0**. Copying text would impose share-alike terms on OPTS documentation (MIT/Apache-2.0). Facts are not copyrightable; wording is. |
| **Attribution.** Each wiki-derived fact is marked 📚 and the source page is listed in [Provenance](#-provenance). | ADR-0002 §5; the licence also requires attribution for adapted material. |
| **Secondary source.** The pages were read through an automatic summariser, not word by word. Treat 📚 facts as hints and confirm them in the corpus or by mutation test before promoting a spec to 🟢. | Summaries can drop or blur details. |
| **Corpus wins.** Where the wiki and the installed files disagree, both are recorded; neither overrides the other until tested. | The wiki describes intended use; the corpus shows what actually ships and works. |

Marks: ✅ wiki agrees with our corpus finding, 🆕 wiki supplies a fact we could not derive, ⚠️ conflict or tension, ❓ still open.

---

## 🔤 General Structure

### Coverage of the wiki's format index

The wiki's index lists these file types; the right column says where OPTS documents them.

| Wiki entry | Wiki description (paraphrased) | OPTS spec |
| :--- | :--- | :--- |
| `.bmp`, `.dds`, `.jpg`, `.png`, `.tga` | textures (alpha only in DDS and TGA; PNG discouraged) | [textures](textures.md) |
| `.surf` | surface structure bitmap | [textures](textures.md), [misc types](text/misc-types.md), see below |
| `.wav` | audio | not analysed (out of Phase 1 scope; 42,375 files exist) |
| `.bus`, `.ovh`, `.cfg` | vehicle / generic configs | [bus](text/vehicles/bus.md), [misc types](text/misc-types.md#ovh), [cfg](text/cfg.md) |
| `.cti`, `.dsc`, `.hum`, `.oft`, `.oop`, `.org`, `.osn`, `.otp`, `.owt`, `.ocu`, `.zug`, `.rpc` | assorted text types | [misc types](text/misc-types.md), [org](text/org.md), [global-cfg](map/global-cfg.md) |
| `.hof` | destinations, stops, routes | [hof](text/vehicles/hof.md) + [audit](text/vehicles/vehicle-corpus-audit.md) |
| `.sco`, `.sli` | objects, splines | [sco](text/sco.md), [sli](text/sli.md), [audit](text/sco-sli-audit.md) |
| `.osc`, `*_constfile.txt`, `*_varlist.txt` | scripts and lists | [osc](text/vehicles/osc.md), [const](text/vehicles/const.md), [varnamelist](text/vehicles/varnamelist.md) |
| `.map`, `.prt`, `.terrain`, `.rdy` | map tile and data | [tiles](map/tiles.md) |
| `.ttl`, `.ttp`, `.ttr` | timetables | [timetables](map/timetables.md) |
| `.o3d`, `.x` | meshes | [o3d](binary/o3d.md), [x](binary/x.md) |
| `.txt` | simple lists | [lists](map/lists.md) |
| `.odr` | driver record | [below](#odr--driver-record) (new) |
| `.ini` | add-on registration | [below](#ini--add-on-registration) (new) |
| `.dll` | plug-in interface | deliberately not specified |

The wiki index itself gives no layout details; the facts below come from its sub-pages (hof, object, script system, surface structure, font, textures, collision, configuration file, system variables).

### Grammar rules (📚 configuration-file page)

The wiki states that keywords sit alone on their line **with no surrounding whitespace**, that text outside keyword blocks is ignored, that lines indented with spaces or tabs act as comments, and that there are **four parameter styles**: none, a fixed number of lines, a count followed by that many lines, and an explicit end marker (`[description]…[end]`). ✅ This matches the grammar used in every OPTS text spec.

⚠️ The wiki says OMSI treats keywords as **case-sensitive**. The corpus has rare variant spellings in shipped `.sco` files: `[nightmapmode]` (2), `[Nightmapmode]` (2), `[lightmapmapping]` (12), `[Lightmapmapping]` (1) against 13,227 / 8,415 canonical uses. If OMSI were strictly case-sensitive those variants would silently do nothing. OPTS should match case-insensitively (superset behaviour) until a test shows otherwise ([Q-CC01](#-open-questions)).

---

## 📚 Field Reference

### 🆕 Facts that answer or narrow open questions

| Topic | Fact 📚 | Resolves / narrows |
| :--- | :--- | :--- |
| **`$msg`** | Writes the string on top of the string stack to the **debug console**. | `vehicle-corpus-audit` Q-AUD-OS01 (`$msg`): consistent with every corpus use (debug-style string building). |
| **`$cutBegin`, `$cutEnd`** | Remove *N* characters from the start / end of the top string. | ✅ confirms the inference from the `Year` examples. |
| **`$SetLengthC`** | Centres a string in a field of given length, padding with spaces (as `$SetLengthL` / `R` do on the other sides). | ✅ confirms. `$SetLengthM` is **not** mentioned on the wiki → still ❓. |
| **`[rendertype]`** | Values: `presurface` (drawn before the terrain), `surface` (with the terrain), `on_surface` (after the terrain), `1` before normal objects, `2` normal (default), `3` after normal objects, `4` after vehicles and pedestrians. | `sco-sli-audit` / `sco.md` Q-SCO-002. Corpus usage: `surface` 6,635; `4` 1,922; `3` 808; `on_surface` 267; `presurface` 158; `2` 86; `1` 28; `0` 4 (`0` is not on the wiki ❓). |
| **`.surf` files** | A bitmap using **only the red channel** as a height map: mid-grey = 0, white ≈ +2 cm, black ≈ −2 cm. It adds small-scale unevenness to a texture's driving surface. Named `<texture file>.<ext>.surf`. | `misc-types` Q-MS06 and `textures` (`.surf`). Corpus check: of 300 `.surf` bitmaps, 191 have G = B = 0, 69 are grey (R = G = B), 40 differ. ✅ largely consistent. |
| **Surface types 0–16** | 0 asphalt, 1 concrete, 2 cobblestone, 3 dirt, 4 grass, 5 gravel, 6 snow, 7 deep snow, 8 slush, 9 ice, 10 water, 11 mud, 12 sand, 13 metal, 14 wood, 15 small cobblestone, 16 interlocking pavers. | `texture-cfg` Q-TC01: the values 11 (8 uses), 15 (44) and 16 (20) found in `*.cfg` files are now explained; the in-file legend stopped at 8. ✅ corpus values all fall in 0–16. |
| **`.oft` font** | The alpha bitmap must have the **same pixel size** as the colour bitmap; white = visible, black = transparent, greys = anti-aliased edges. Characters are cut as straight rectangles; a font can define up to roughly 110 character codes; keep the file under 1 MB. | `misc-types` `.oft`: adds the alpha-bitmap rule. The `[newfont]`/`[char]` parameter lists agree with our census. ✅ |
| **Textures** | Alpha only in DDS and TGA; BMP and JPEG are opaque; PNG is discouraged (display errors). Sizes should be powers of two but need not be square (e.g. 64×2048, 1×1). Search order: the object's/spline's `texture` folder, then the global `Texture` folder; sub-folders and parent-folder references allowed. Season folders (`WinterSnowfall`, `WinterSnow`, `Spring`, `Summer`, `Fall`, `Winter`) override base textures; the variant must have the **same name and extension**. | `textures` Q-TX06 (4th channel of 32-bit BMP: wiki says BMP has no transparency ❓ unverified), Q-TX04 (NPOT is "should", not "must"; 12 % of corpus DDS are NPOT), Q-TX01 (folder order ✅ matches; extension fallback not mentioned ❓). Corpus has `WinterSnow` texture sub-folders ✅. |
| **Collision** | Five methods: none, bounding box, mesh, fixed, surface. `[boundingbox]` is given as width/length/height plus offsets measured from the **environment's origin, not the object's pivot**. `[fixed]` and `[surface]` are `.sco`-only; **surface collision stays active even when collisions are switched off** in the settings. | Adds semantics to `sco.md` collision section. ❓ axis order of the six values. |
| **System variables** | Five groups: read-only system variables (`(L.S.name)`), writable vehicle float variables, vehicle string variables, scenery-object variables, human variables. Many vehicle variables work without declaration; those marked "on-demand" must appear in the varlist to connect to the engine. Parts sharing a script (`LINK SCRIPTSHARE`) share one variable store. | `osc.md` / `varnamelist.md`; relevant to Q-AUD-VL01 (duplicate names) — shared stores may explain some duplicates ❓. Example names: `Timegap`, `GetTime`, `Throttle`, `Brake`, `Velocity`, `TrafficLightPhase` (0–11). |

### 🆕 Script-system facts (📚 script-system page)

- Reverse-Polish evaluation on **two stacks**, one for floats and one for strings; **8 float stack slots** and **8 float registers** (`s0`–`s7` store, `l0`–`l7` load). Operators on strings carry a `$` prefix (`$+`, `$=`, `$<` …); `d` / `$d` duplicate the top of each stack.
- Entry points `{init}`, `{frame}`, `{frame_ai}` (frame handler when the vehicle is not the player's), plus reusable `{macro:name}` and input/event handlers `{trigger:name}`; every block closes with `{end}`. Macros are called by name and may be called before their definition.
- Trigger naming: keyboard triggers come as `name` and `name_off`; mouse triggers on a mesh as `meshname`, `meshname_drag`, `meshname_off`. Sound triggers use `(T.F.name)` with the file name on the string stack.
- Syntax is described as **case-sensitive**. ⚠️ The corpus contains `$CutEnd` (7 uses) next to `$cutEnd` (13,917) and the typo `$SetLenghtL` (1) — either OMSI tolerates them or those lines silently fail ❓ ([Q-CC01](#-open-questions)).
- Control flow: condition on the stack, `{if}`, optional `{else}`, `{endif}`; nesting allowed.

### 🆕 Hof facts (📚 hof page)

- Four sections in this order: file info (`[name]`, `[servicetrip]`, `[global_strings]` — the last is OMSI 2 only), terminus list (`[addterminus]`, `[addterminus_allexit]` — the latter lets everyone exit and nobody board), stop list (`[addbusstop]`), routes (`[infosystem_trip]` + `[infosystem_busstop_list]`, which must reference stops from the stop list). ✅ matches `hof.md`.
- **String slots (OMSI 2 "universal" assignment):** a terminus row has up to **26** string slots (slot 0 IBIS line 1, 16 chars upper-case; 1–3 ANNAX matrix; 4 roller-blind texture file; 5 IBIS line 2, 20 chars; 6 plug-in sign texture; 7–9 Krüger/rear display; 11–12 full-matrix; 13–16 LAWO matrix; 17–25 system-specific). A stop row has **9** slots (0 IBIS 1; 1–2 interior displays; 3 IBIS 2; 4–8 special). Corpus: stop rows with 9 lines occur 2,157 times in a 2,500-file sample; typical rows have 4–5 populated lines ✅ plausible.
- Passengers react **only to names and IBIS codes**, not to the strings. Terminus codes are three digits for IBIS; four or more digits for non-IBIS devices; ≥ 1000 for plug-in signs. Announcement audio file names must match the stop strings exactly (without `.wav`); a `_#terminus` suffix marks the version played at the final stop. The actual route is defined in the editor; the hof only controls displays.
- Strings are **tab-separated** in OMSI 2 files; ⚠️ the wiki says the file must be **ANSI**. The corpus has **2,540 UTF-16LE** hofs and 5,383 UTF-8/ASCII ones. Whether OMSI reads UTF-16 correctly (or those files were converted for OPTS-style tools, e.g. a hof editor) is ❓ ([Q-AUD-HF02](text/vehicles/vehicle-corpus-audit.md#-open-questions)).

### 🆕 Object facts (📚 object page)

- `[friendlyname]` is the only mandatory keyword; `[groups]` is a count followed by that many folder names.
- `[matl_alpha]`: the wiki lists modes 0 (opaque), 1 (full alpha) and 2 (partial alpha). ⚠️ **Corpus counts of the values** (25,508 uses, reading the value line): `2` ×12,119, `1` ×10,290, `0` ×2,201, `3` ×896, `16` ×2. `sco.md` states "`1` in 99 % of uses"; that figure does **not** hold for the whole corpus (the draft's sample may have differed). Mode `3` is not on the wiki ❓ ([Q-CC02](#-open-questions)).
- `[petrolstation]` makes the neighbourhood usable for refuelling/washing; `[crossing_heightdeformation]` bends the object to a curved base plane; `[terrainhole]` cuts a hole using a mesh. These confirm names found in `sco-sli-audit` (`[petrolstation]` 58 files, `[terrainhole]` 161 files).
- `[NightMapMode]` values 0–4 ✅ identical to `sco.md`. `[complexity]` 0–3 ✅ identical.

### `.odr` — driver record

The wiki lists `.odr` as a driver record; the install has **one** such file, `Drivers/OMSI-Fan.odr` (UTF-16LE with BOM, 640 bytes). It is a keyword file (`[ident]`, `[busstops]`, `[hektom]`, `[crashs]`, `[tickets]`, `[rating]`, `[perbusinfo]`) written by OMSI. [H] Layout seen:

| Keyword | Lines | Observed |
| :--- | ---: | :--- |
| `[ident]` | 4 | name (`OMSI-Fan`), a one-letter code (`M`), two dates `yyyymmdd` (`19990101`, `20131221`). ❓ |
| `[busstops]` | 3 | `8`, `0`, `0`. ❓ |
| `[hektom]` | 1 | `27` (kilometres driven? ❓) |
| `[crashs]` | 4 | `0 0 0 1`. ❓ |
| `[tickets]` | 2 | `0 0`. ❓ |
| `[rating]` | 5 | `17 17 0 0 0`. ❓ |
| `[perbusinfo]` | 2 | vehicle description (`C2-Fam-Vol-1 C2G 3-Door`) and `27`. ❓ |

A banner (`Driver File`, `Created with Omsi Version …`) precedes the first keyword. Only one sample exists, so every field meaning is ❓.

### `.ini` — add-on registration

44 `.ini` files: `addons.ini` and `RegAddons/*.ini` hold registered add-ons (`[addon.0]` with `Name=`, `ArtNr=`, `Steamname=`, `SteamArtNr=` — `key=value` lines), a few `desktop.ini`/`viewstate` files are Windows folder metadata, and `OMSI-Tools_Updater.ini` belongs to a tool. They are **not game data**; OPTS can ignore them. [H]

---

## 🧪 Examples

How a wiki-derived fact is recorded in a spec (pattern for contributors):

```markdown
| `$cutEnd` | Removes N characters from the end of the top string. | 📚 community wiki (script system page); ✅ consistent with corpus example `(L.S.Year) $IntToStr 2 $cutEnd` |
```

Never paste the wiki's sentences; restate the fact and cite the page.

---

## ❓ Open Questions

| ID | Question | Priority |
| :--- | :--- | :--- |
| Q-CC01 | Is OMSI really case-sensitive for keywords and `$` functions? The corpus has variant spellings that work in shipped content. Test: a `.sco` using `[NIGHTMAPMODE]` and a script using `$CUTEND`. | 🟡 Medium |
| Q-CC02 | `[matl_alpha]` modes 0–3 (and `16`): exact behaviour. Wiki lists 0–2; the corpus is dominated by `2` and `1`. Needs T03-style test (alpha texture under each mode). | 🔴 High |
| Q-CC03 | `[rendertype]` `0`: meaning (4 uses, not on the wiki). | 🟢 Low |
| Q-CC04 | Do the 2,540 UTF-16 hofs work in OMSI despite the wiki's "ANSI" statement? | 🟡 Medium |
| Q-CC05 | `.odr` field meanings (single sample); does OMSI create one per driver in `Drivers/`? | 🟢 Low |
| Q-CC06 | Do the wiki's stack-size limits (8 slots/registers) really cap scripts? Longest corpus expressions use fewer than 8 live values ❓ unmeasured. | 🟢 Low |

---

## 🔄 Update 2026-10-04 (M-0006): register of the 31 pages

All 31 pages listed by the project lead were read; the table says what each contributed. Three are **editor/tool tutorials with no format facts** (`12-kreuzungsbau`, `21-bildbearbeitung-mit-alphakanal`, `69-verkehrsregeln` beyond category names) and one is an asset list (`4-liste-…`).

| # | Wiki page | Technical content | Used in |
| ---: | :--- | :--- | :--- |
| 1 | `1-scriptsystem` | script system | [osc](text/vehicles/osc.md) |
| 2 | `3-system-und-vordefinierte-lokalen-variablen` | variable groups, on-demand declaration | [varnamelist](text/vehicles/varnamelist.md), osc |
| 3 | `4-liste-aller-originalen-objekte-und-splines` | list of ≈ 1,500 original assets with a copyright note — **no format facts** | — |
| 4 | `5-blender-import-export-von-3d-modellen` | authoring conventions (axes, units, export settings) | [o3d](binary/o3d.md) Addendum 2, [x](binary/x.md), [textures](textures.md) |
| 5 | `7-oberflaechenstrukturen` | `.surf` height map, surface types 0–16 | this page, [textures](textures.md) |
| 6 | `8-objekt` | `.sco` keywords, rendertype, alpha, collision | [sco](text/sco.md), [audit](text/sco-sli-audit.md) |
| 7 | `9-splinedateien-sli` | `.sli` keywords, path types, patchwork | [sli](text/sli.md) |
| 8 | `10-fahrzeuge-in-omsi-ordnerstruktur` | vehicle folder contents, budgets | [bus](text/vehicles/bus.md), [model_cfg](text/vehicles/model_cfg.md) |
| 9 | `11-textur` | formats, alpha, sizes, lookup, season folders | [textures](textures.md) |
| 10 | `12-kreuzungsbau` | editor/Blender **tutorial**; gives no keyword specs; points to an unofficial SDK manual that is not part of the install | — |
| 11 | `13-dateiformate` | index of types | this page |
| 12 | `14-hofdatei` | `.hof` structure, string slots, codes | [hof](text/vehicles/hof.md) |
| 13 | `15-font` | `.oft` keywords, alpha bitmap rule | [misc types](text/misc-types.md) |
| 14 | `16-fahrgastwege` | passenger paths and cabin keywords | [paths_cfg](text/vehicles/paths_cfg.md), [passengercabin_cfg](text/vehicles/passengercabin_cfg.md) |
| 15 | `17-licht` | light types, `[light_enh_2]`, interior/spot lights | [model_cfg](text/vehicles/model_cfg.md), [audit](text/sco-sli-audit.md) |
| 16 | `18-auskommentieren` | comment methods | [sco](text/sco.md), [osc](text/vehicles/osc.md), [misc types](text/misc-types.md) |
| 17 | `19-fahrtzielanzeiger-an-bussen` | display systems, macros, variables | [hof](text/vehicles/hof.md) |
| 18 | `20-ki-verkehr-von-ailists-pkw-und-menschen` | AI lists, unscheduled traffic, lists | [lists](map/lists.md) |
| 19 | `21-bildbearbeitung-mit-alphakanal` | alpha-editing **tool how-to**; no format rules | [textures](textures.md) |
| 20 | `22-chronologiefunktion` | chrono folders and `Chrono.cfg` | [timetables](map/timetables.md) |
| 21 | `23-tracks-vs-stationlinks` | Type 1 vs Type 2 trips | [timetables](map/timetables.md) |
| 22 | `59-kollisionen` | five collision methods | [sco](text/sco.md) |
| 23 | `62-konfigurationsdateien` | keyword grammar, parameter styles | this page, [sco](text/sco.md) |
| 24 | `69-verkehrsregeln` | editor **tutorial** for traffic rules; categories only, no parameter structures | [tiles](map/tiles.md) |
| 25 | `70-radverkehr-gestaltung-und-implementierung` | cyclist setup (paths, groups, constants) | [tiles](map/tiles.md), [bus](text/vehicles/bus.md) |
| 26 | `75-fahrkarten` | `.otp` ticket packs | [misc types](text/misc-types.md) |
| 27 | `81-debug-modus` | `-debug` launch parameter and tools; says nothing about `logfile.txt` | [bus](text/vehicles/bus.md) |
| 28 | `92-animationen` | `[newanim]` semantics | [model_cfg](text/vehicles/model_cfg.md) |
| 29 | `93-haltestellenansagen` | announcement folders and file naming | [hof](text/vehicles/hof.md) |
| 30 | `122-zugverkehr-in-omsi-2` | `.zug`, switches, signals, type-1 trips | [timetables](map/timetables.md), [misc types](text/misc-types.md) |
| 31 | `141-ki-menschen` | `.hum` keywords, bones, T-pose | [misc types](text/misc-types.md) |

### New conflicts and findings from this pass

| ID | Finding |
| :--- | :--- |
| Q-CC07 | ⚠️ **`[light_enh_2]`**: wiki lists 25 parameters (two variables, a "cone vector"); the files have at most 24 (one variable) — see [audit](text/sco-sli-audit.md). |
| Q-CC08 | ⚠️ **`.hum` `[links]`**: wiki says 22 values; corpus has 22 (22 files) and **28 (36 files)**. |
| Q-CC09 | ⚠️ **Keyword naming**: wiki calls the passenger-cabin light-assignment command `[interiorlight]`; the files use `[illumination_interior]` (3,056 uses). |
| Q-CC10 | ⚠️ **Undocumented in specs, documented/seen**: `[pathlink_oneway]` (887 uses) was missing from `paths_cfg.md`; path type `3` (aircraft) was missing from `sli.md`. Both now added. |
| Q-CC11 | 🆕 **`.o3d` axes**: Y is up and Z is longitudinal in file space (corpus measurement), versus the authoring tool's Z-up/+Y-forward — see `o3d.md` Addendum 2. Sign of forward and handedness remain for test T01. |
| Q-CC12 | 🆕 **Trip types**: `[station]` is the Type-1 (track) station list, not legacy — corrected in `timetables.md`. |

### Questions resolved or narrowed in this pass

| Spec | ID | Result |
| :--- | :--- | :--- |
| `timetables.md` | Q-T06 (relabelled) | `[station]` = Type 1; `[trip]` line 1 = `.ttr` name (3,655/3,661) |
| `lists.md` | Q-L01, Q-L03, Q-L06 | depot reference, typgroup columns, default share, parklist variants |
| `misc-types.md` | Q-MS05 | `.zug` orientation flag |
| `sli.md` | Q-SLI-001 | `[patchwork_chain]` structure |
| `sco.md` | Q-SCO-002, Q-SCO-004 | rendertype values; light line layout |
| `model_cfg.md` | Q-007 | `[interiorlight]` has 8 lines |
| `osc.md` | Q-OSC-005, audit Q-AUD-OS01 | `%` modulo; `$msg` debug output |
| `texture-cfg.md` | Q-TC01 | surface types up to 16 (see above) |
| `o3d.md` | Q-005 (partly) | axis assignment |

---

## 📖 Shipped Manuals (M-0008 review, 2026-10-04)

The project lead authorised reading the manuals in the install's `docs/` folder. Reviewed: `Manual_OMSI2_EN.pdf` (80 pages; the DE edition has the same structure), `OMSI2_QuickRef.pdf` (text extracted with a standard PDF-to-text tool). The manual is a **gameplay manual** (controls, the bundled MAN buses, Berlin routes, ticket prices, destination-code tables). It contains **no file-format or script reference**, so it adds little to the specs; the facts it does confirm or add are below (📖, paraphrased; no text copied):

- **Situation** = a saved game state: the chosen map, all buses with place, condition and settings, the time and the weather. ✅ matches the `.osn` blocks ([global-cfg](map/global-cfg.md)).
- **Driver profiles**: a first install has only "Unknown Driver"; a new driver profile is created from the start menu (this is the `Drivers/*.odr` file above).
- **Chronology event** = a change to the map that exists only for a period (e.g. a construction site from May to September, a new bus type from 1992) ✅ consistent with [timetables](map/timetables.md).
- **Circuits** (tours) such as `Mo-Fr 1` can only be selected when appropriate to the current date and time; night lines are only selectable at night (0–4 h in Berlin); a maintenance circuit brings the AI bus back to the depot when the player takes over; an "additional bus" runs alongside the AI bus ✅ matches the `[newtour]` day-type names and depot trips.
- **Passengers board only buses whose displayed destination they expect**, so the destination display must be set from the hof ✅ ([hof](text/vehicles/hof.md)).
- **Ticketing**: an "easy" and an "advanced" mode; since 1993 buses have a ticket printer, older ones ticket pads; change is given with coin buttons; prices change over the simulated period 1986–1994; the old Deutsche Mark has **no 20-pfennig coin** ✅ the DM money configuration lists 5, 10 and 50 Pf and 1, 2, 5 DM coins.
- **IBIS line/course input**: a five-digit "Line/Course" value (three-digit line plus a two-digit course code, e.g. `00500` for line 5, `13000` for line 130, `09210` for 92E) selects what the matrix display shows; the manual tabulates the display patterns per two-digit code (e.g. `00` plain line number, `04` line + `N`, `98` chessboard test, `99` all-in/all-out switch). This is the player-side meaning of `IBIS_LinieKurs` / `IBIS_Linie_suffix` ([predefined variables](text/vehicles/predefined-variables.md)).
- **Keyboard shortcuts** can be rebound for any event that a mouse click or existing shortcut can trigger (so `Inputs/keyboard.cfg` lists *actions*); `Shift+Z` cycles the status display.
- **Destination tables** list, per Berlin destination, a roller (blind) number, a matrix code and the matrix code since 29 May 1994 — i.e. a terminus has **separate codes per display system** ✅ matches the hof terminus rows with multiple code columns.

---

## 📝 Provenance

- **Shipped manuals (M-0008, 2026-10-04):** `docs/Manual_OMSI2_EN.pdf` and `OMSI2_QuickRef.pdf` read via text extraction with the project lead's permission; paraphrased, no format reference found.
- **M-0006 (2026-10-04):** all 31 pages listed in the milestone were read (register above); per-page facts are recorded in the specs named in the register. The page text was read through an automatic summariser, so each 📚 fact is a hint to be confirmed. No tool source code was opened.
- **Wiki pages read** (2026-10-04, via an automatic page summariser; CC BY-SA 4.0, source: OMSI community wiki "Reboot"): `…/wiki/entry/13-dateiformate/` (index), `14-hofdatei`, `8-objekt`, `1-scriptsystem`, `7-oberflaechenstruktur`, `15-font`, `62-konfigurationsdatei`, `11-texturen`, `3-system-und-vordefinierte-lokalen-variablen`, `59-kollision`. No wording is reproduced; every 📚 item is a restatement of a fact.
- **Corpus checks run for this page:** keyword spelling census (`.sco`), `[rendertype]` and `[matl_alpha]` value tally over all 50,499 `.sco`, `.surf` channel test on 300 files, `.hof` row-length tally on 2,500 files, `.odr`/`.ini` inspection.
- **Not read:** the wiki's Blender-plug-in page and any tool source code.
- **Attribution:** "Dateiformate" and related pages, OMSI community wiki (`reboot.omsi-webdisk.de`), licensed CC BY-SA 4.0 (https://creativecommons.org/licenses/by-sa/4.0/). OPTS reproduces no wiki text; this attribution is given as a courtesy and for traceability.

---

**Clean-room attestation (ADR-0002):**
- No OMSI binaries were inspected.
- No GPL source code was read.
- No OMSI assets were committed.

## 🔗 Related Documents

- [ADR-0002: Clean-Room Policy](../adr/0002-clean-room-policy.md)
- [`.sco`](text/sco.md), [`.hof`](text/vehicles/hof.md), [`.osc`](text/vehicles/osc.md), [textures](textures.md), [texture property files](text/texture-cfg.md), [misc types](text/misc-types.md)
