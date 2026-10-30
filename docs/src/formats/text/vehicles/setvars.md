# Setvars — Equipment Options of a Vehicle

| Metadata | Value |
| :--- | :--- |
| **Format Family** | Text (a `[setvar]` block inside `.cti` files) plus variable declarations |
| **Referenced By** | `.cti` repaint files, `model.cfg` `[CTC]` blocks, `*_varlist.txt` |
| **Parser Crate** | `opts-core` (`formats::cti`, `formats::varlist`) |
| **Status** | 🟢 Specified from the corpus and the vendors' documentation; catalogue, editors and `vehicle.setvars` implemented (M-0078) |
| **Last Updated** | 2026-10-18 |
| **Traces to** | [M-0078](../../../roadmap/phase-10/M-0078.md), field report R-16, [repaints](../repaints.md) |

A **setvar** is a script variable that a vehicle's repaint sets when the vehicle is created, so the scripts switch **equipment**: a display type, a seat layout, a door style, a language, a wheel cover. This page says where setvars come from, what the installed vehicles and their vendors' documentation show, and how OPTS derives and applies a per-vehicle list. Facts carry their source; ❓ marks guesses.

---

## 1. What a setvar is

- In a `.cti` file a `[setvar]` block gives a variable name and a value; it belongs to the repaint of the `[item]` that precedes it ([repaints](../repaints.md), [misc types](../misc-types.md)).
- The variable must be declared in one of the vehicle's variable lists (`[varnamelist]` in the `.bus`, [varnamelist](varnamelist.md)); the scripts read it like any other float variable. The value is the state **at spawn**; scripts may change it later.
- A setvar is therefore not a file format of its own: it is a **convention** that the vehicles' authors use, with the `.cti` as the carrier and the varlist as the symbol table. OMSI's own list of which variables are setvars does not exist; every vehicle family defines its own.

## 2. What OPTS did before M-0078

| Step | Code | Behaviour |
| :--- | :--- | :--- |
| Discovery | `new_game_screen/repaints.rs` (`discover_at`) | `.bus` → `model.cfg` → `[CTC]` schemes named `Colorscheme` → every `.cti` of the folder; the `[setvar]` lines of each repaint are kept |
| Option list | `new_game_screen/setvars_discovery.rs` (`setvars_from`) | the union of the names set by **any** repaint of the vehicle; the first value seen is the default; four names have a hand-written label, the rest are the name made readable (`vis_front_light` becomes *Vis front light*); every option is offered as a **0/1 switch** (`min_val: 0, max_val: 1`) |
| Apply | `omsi_world/vehicle.rs` (script engine set-up) | the chosen repaint's `[setvar]` values are set on the script engine, then the user's overrides (`OmsiSessionConfig.setvars`, from `NewGameSetup::custom_setvars`) |
| Remembered | `drive_history.rs`, the session save | the user's overrides travel with the drive and the save |

What this gets wrong, from the census below: the switch limit does not fit the roughly 200 options that are choices, numbers or fractions; the label of 778 of the 782 names is only the identifier; and an option that no repaint of the vehicle sets cannot be offered at all.

## 3. Census of the installed vehicles (2026-10-18)

`cargo run --release -p opts-core --example setvar_census -- "<OMSI root>"` over `Vehicles/**.bus` (1,492 files, 1,478 readable). The counts are per `.bus` file, and **many files are variants of one model**: 517 of the 1,095 vehicles with setvars belong to the three Kajosoft Citybus families (o530 486 files, 628c/628g 31), so "used by N vehicles" overstates breadth.

| Fact | Value |
| :--- | :--- |
| Vehicles with `.cti` files / with at least one `[setvar]` | 1,298 / 1,095 |
| `[setvar]` lines | 391,997 (all in repaint schemes named `Colorscheme`; the one other scheme name, `cp_vdvdisplay_brightness` on 10 vehicles, carries none) |
| Distinct setvar names | 782; per vehicle median 36, at most 83; 32 names belong to one vehicle only |
| Declared by the vehicle's varlists | 43,390 of 43,831 (vehicle, name) pairs (99 %); the 441 undeclared are mostly `vis_cti_*` and `cp_*` names that a repaint sets for a vehicle whose scripts do not declare them (❓ ignored by OMSI, or read by the host) |

**Shapes of the values that repaints give a name** (a lower bound of what the option accepts):

| Shape | Names |
| :--- | ---: |
| `0` and `1` | 262 |
| only `1` (a switch that a repaint turns on, otherwise unset) | 239 |
| only `0` | 64 |
| `1..N` in a row (a choice counted from 1) | 73 |
| `0..N` in a row (a choice counted from 0) | 82 |
| a set with gaps (codes) | 44 |
| one other value | 15 |
| not integers (door acceleration and speed, `-0.7`, `-0.1`) | 2 |

Free numbers exist: registration plate parts and vehicle numbers reach 221,930, `vehicle_id` 3,964. The most common names across the corpus are display, air-conditioning, seat, front-light, handrail, door-type and language options (`display_ext` takes 1 to 8, `cg_type` 1 to 6, `interior_seats` 1 to 6).

**Which lists declare them.** A list file named for setvars is common (`setvar_list.txt` in 517 vehicles with 34,686 pairs, `setvar_varlist.txt`, `lite_setvars.txt`), as are lists for visibility (`visual_varlist.txt` 252 vehicles, `c2_visual_varlist.txt`, `sichtbarkeiten_varlist.txt`) and for other groups (`klappern_varlist.txt` for door beeps, `steuerleitung_varlist.txt`). No list is called `*visible*`; the lists the project lead meant are the *visual* and *sichtbarkeiten* ones. **A list named for setvars is not the supported list:** the Kajosoft o530's `setvar_list.txt` declares 446 names while its repaints set 76 (and its vendor documents 83). Summed over the 560 vehicles that have such a list, the lists declare 228,690 (vehicle, name) pairs of which 192,988 are names that no repaint of that vehicle sets; the declaration is a symbol table, not a menu.

## 4. What the vendors document

Twelve vehicle families ship a setvar list or guide with their Add-on (the installation holds 1,760 PDFs, 97 of which mention setvars; the project lead allowed reading them in full on 2026-10-18, bought and code-free, [ADR-0002](../../../adr/0002-clean-room-policy.md); nothing of their text or images is reproduced here). A digest of one language per family:

| Family | Vehicle folder | Documented | Set by its repaints | Names look like |
| :--- | :--- | ---: | ---: | :--- |
| Citybus o530 (Kajosoft) | `Citybus 530 by Kajosoft` | 83 | 81 | plain words (`display_ext`, `front_light`) |
| Citybus o530 Facelift | `Citybus 530 Facelift by Kajosoft` | 83 | 81 | same |
| Citybus 628c / 628g LF | `Citybus 628c 628g LF by Kajosoft` | 38 | 35 | same |
| Heuliez GXx17 | `HeuliezBus_GXx17` | 9 | 9 | `ver_*`, `vis_*`, `var_*` |
| IVECO Interurbain (Crossway) | `CW_IVECOBUS_INTER_FAMILY` | 41 | 41 | `sv_*`, `*_req` |
| IVECO Low Entry | `CW_IVECOBUS_LOWENTRY_FAMILY` | 65 | 56 | `sv_*`, `*_req` |
| Irisbus Crossway Low Entry | `CW_IRISBUS_LOWENTRY_FAMILY` | 54 | 51 | `sv_*`, `*_req` |
| Valiant Citybus (Renault 7000/7700) | `Valiant_Citybus_Family` | 23 | 20 | `ver_*`, `vis_*`, `var_*` |
| MAN New Lions City | `MAN_NewLionsCity` | 58 | 58 | `vis_cti_*` |
| IVECO Crealis (Natural Power) | `ADCrealis-Fam` | 29 | 24 | `svr_*` |
| Citybus S31X (Setra S315NF/S319NF) | `Citybus_S31x` | 9 | 8 | `ver_*`, `vis_*` |
| IVECO Urbanway | (folder not matched) | 18 | – | `tp_*` and others |

**Cross-check against the install** (11 families with a matched folder): 464 of the 492 documented names are set by that family's repaints (94 %, 28 never set); of the 405 whose documented values are a plain range or list, **396 (98 %)** have only values inside it in the repaints. The 9 that do not (for example `wheels` and `window_optional` on the Facelift, the Valiant `var_plate` with codes up to 71, the Heuliez `ver_sieges_av` with 10) are documented as `0|1` or similar by the digest while repaints set more: either the digest misread a picture (several documents give their value tables as images) or the vendor lists only some values. The working range of an option is therefore the documented range **joined with** the observed values.

What the documents agree on:
1. **A setvar is applied by a `[setvar]` line in a repaint's `.cti`**, name then value; one document warns that the line must precede each variable and that the value is the state at spawn. Two families (Kajosoft, IVECO Interurbain) add an in-game tablet or configurator; only Kajosoft documents labels for it (English, Polish, German).
2. **The `.cti` is not the only source.** A global constant file (`klappern_constfile.txt`) can override the door-beep setvar of the IVECO and Irisbus families.
3. **Names are not shared between families** (one concept, many names: seats are `ver_sieges`, `var_seats`, `svr_seat`, `vis_CTI_Sitztyp`, `interior_seats`), apart from the French-style vocabulary of Heuliez, Valiant and S31X. Typos exist in the vendors' own names (`cieply_guzik` and `cieply_guzyk`, `vis_enjo` and `vis_enjol`); the install decides which spelling is real.
4. **0 does not always mean off.** Several IVECO and Irisbus options are inverted (0 means active), hubcap values flip per model, a stop-request style above 2 means no button, a Valiant air-conditioning value of 10 means none. Value meanings are per vehicle variant, not a global convention.
5. **Availability depends on the variant** in every family (articulated or not, length, model year, CNG), and options depend on each other (one A/C value needs a second option set, one display option needs a cover switched off). Several vendors warn that setting a name they do not list can cause faults; two refuse support for it.
6. **Groups seen:** display, interior, doors, driver, exterior, wheels, climate, sound, drivetrain, identity (numbers, plates), body.
7. **Free numbers:** vehicle number and plate parts are not choices.

Gaps: the Kajosoft lists are mostly pictures (value text for about 17 of the 38 options of the 628), the wheel, stop-request and panel tables of the IVECO families are images, and a few rows are ambiguous (`doors_beep_disable_req` is described differently by the IVECO Low Entry and Irisbus documents). Families without a document (the C2 family, the Heuliez GX327, many AI-only models) have repaint evidence only.

## 5. The catalogue OPTS builds

One entry per setvar of a vehicle, merged from the evidence in this order of trust:

| Level | Evidence | Gives |
| :-: | :--- | :--- |
| A | the vehicle's repaints (`.cti` `[setvar]`) | the name, the observed values (a lower bound of the range), the repaint defaults |
| B | the vehicle's varlists | whether the name is declared (a script variable the vehicle reads) and the list it is declared in (group hint) |
| C | the vendor's documentation, as a **curated table in the repository** (our own words, keyed by vehicle family) | label, value meanings, groups, dependencies, availability by variant |

Rules:
- **Kind** from the shape: *switch* (values within 0 and 1, or only 1), *choice* (consecutive or coded integers), *number* (an identifier or plate: more than 100 distinct or very large values), *fraction* (non-integers). The working range is the observed values joined with the documented range.
- **Only vehicle-declared names are offered** (level B), unless a curated entry says otherwise; unlisted names are not shown by default (vendors warn against them) and can be typed in an advanced field with a warning.
- **Unverified is marked:** an entry without level C has no value meanings and shows the identifier as its label; ranges taken from images are flagged ❓.
- **The default** shown is the value of the repaint the drive starts with (not the first value seen among all repaints).
- **Apply** is unchanged in order: the repaint's setvars, then the user's overrides at spawn; a live change reaches a script only if the script reads the variable after initialisation (❓ Q-SV02).

## 6. What OPTS does now (M-0078)

- **Catalogue:** `opts_core::setvars::SetvarCatalogue` builds the list of one vehicle as described in §5 from the repaints' `[setvar]` lines, the variables the vehicle declares (`VehicleRepaints::declared`) and the curated vendor data (`opts-core/data/setvar_families.json`, twelve families, 510 setvars, matched through the folder below `Vehicles`). Across the installed vehicles with setvars it gives a median of 58 options per vehicle (the repaints set about 36; the rest are documented options the vehicle declares) and leaves out 439 undeclared names.
- **Wizard:** the equipment dialog offers a **switch**, a **choice box** or a **number field** per option; the default is the value of the chosen repaint; labels and descriptions come from the vendor document where there is one. Decimal options (door dynamics) are not offered; their repaints keep setting them.
- **In-game vehicle menu:** ‹ and › step a choice through its values or a number by one (`menu.do step <option> [-1|1]`).
- **Respawn metadata (M-0110):** each curated option may carry `respawn` (`true`: vendor says restart, `false`: live test saw a reaction); the catalogue exposes it as `Respawn::{Required, Live, Unknown}`, the Alt menu page marks `Required` options and `vehicle.setvars` lists it (protocol 1.31).
- **Protocol:** `vehicle.setvars` ([control protocol](../../../architecture/control-protocol.md)) lists the options with kind, range, value, default, group and evidence and changes several at once, all or none; in a drive the change is set on the player's scripts at once and kept for a respawn.
- **Apply order** is unchanged: the repaint's setvars, then the user's overrides at spawn.
- **Alt menu *Setvars* (M-0110):** the options of the **active session vehicle** with a toggle or stepper each; a change is set on its scripts at once and kept on its session record (`SessionVehicles::set_setvar`), so the session save and *Save current vehicle* carry it. `vehicle.setvars` in a drive now reads and writes the active session vehicle (before: the vehicle the drive started with). Whether a script reads an option again after start stays open per option (Q-SV02): an option the vendor says needs a restart (`respawn: required`, three of 510 documented) is marked *needs respawn*; for the rest the page offers no promise; *Spawn* on a saved configuration applies every option from the start.

### Live check on the real game

`scripts/setvars-check.py` is the screenshot recipe: it starts a drive on the installed OMSI, lists the options, changes some while the simulation runs, and compares the script variable and a crop of the screenshot (against a noise baseline of two unchanged screenshots) after each. Run on the Kajosoft Citybus o530 (`01a_o530_e2_2`, Grundorf, external camera, 2026-10-18):

| Result | Detail |
| :--- | :--- |
| The catalogue | 83 options, the number the vendor documents; family `citybus-o530-kajosoft`; repaint `Beige` |
| Live changes accepted | `air_conditioning=1`, `display_ext=1` and `4`, `grill_flat=1`, `logo_front=1`, `front_light=1`: the script variable took the value each time |
| Refused by the vehicle's own range | `air_conditioning=4` (accepts 0 to 3), `front_light=2`, `mirror_right=2`, `wheels=2` and `front_light=3` (0 or 1 on this model; the Facelift's repaints set 2) |
| Visible in the picture | **`air_conditioning=1`:** extra units at the far end of the roof, 1,283 changed pixels against a noise level of 25 (the crop around the bus, 3 % tolerance) |
| Marginal | `display_ext=4` (a thin line at the lower edge of the front display), `grill_flat`, `logo_front`, `front_light`: 88 to 105 pixels, a few times the noise but not clearly visible from this angle; `display_ext=1`: 7 pixels, within the noise |

So the model reacts live to at least the roof air-conditioning option; whether the front-face options react live, or are read by the scripts only at start, was not decided by this run (Q-SV02 stays open, with a recipe to settle it per option from other camera angles).

## ❓ Open Questions

| ID | Question | How to settle it |
| :--- | :--- | :--- |
| Q-SV01 | Are the 441 undeclared (vehicle, name) pairs ignored by OMSI or read by the host (the research report counts 872 buses that read `.cti` livery names)? | Test in OMSI with an undeclared name; cross-check [bus script integration](../../../architecture/bus-script-integration.md) |
| Q-SV02 | Which setvars react to a change after the vehicle exists? (`air_conditioning` on the Citybus o530 does and is marked `live`; the vendors document three options that need a restart and they are marked `required`; the front-face options were inconclusive from the external camera behind the roof; all others are `unknown`.) | `scripts/setvars-check.py` with `--crop` around the part that should change, from a camera that sees it |
| Q-SV03 | Which values does a family accept beyond what its repaints set (the digest's `0|1` against observed 2)? | Read the vendor tables that are images against a screenshot of the in-game configurator; or set the value in a test `.cti` |
| Q-SV04 | Does OMSI apply a `[setvar]` of a repaint that is not the active one (a vehicle with several `.cti` files in the folder)? | Experiment protocol ([OMSI experiments](../../../architecture/omsi-experiment-protocol.md)) |

---

## 📝 Provenance
- Census: `crates/opts-omsi/examples/setvar_census.rs` over the installed `Vehicles` folder, 2026-10-18; the figures above are its output. Live check: `scripts/setvars-check.py`, 2026-10-18.
- Vendor documents: one language per family of the setvar lists and guides in the `Addons` and `Vehicles/*/Templates` folders, read in full by an analysis pass; names, ranges and meanings are restated in our own words; the cross-check numbers come from comparing that digest with the census per vehicle folder. No text or image of a PDF is committed.
