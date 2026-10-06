# Repaint System — `[CTC]`, `.cti`, Slots and Templates

| Metadata | Value |
| :--- | :--- |
| **Format Family** | Mechanism spanning text files (`model.cfg` / `.sco` hooks, `.cti` presets) |
| **File Extensions** | `.cti` (3,152), `.rpc` (142), template bitmaps |
| **Parser Crate** | `opts-core` |
| **Parser Module** | `opts_core::formats::repaint` |
| **Encoding** | CP1252 / UTF-8 (per file) |
| **Status** | 🟡 Draft — chain verified against the corpus; selection behaviour unverified |
| **Last Updated** | 2026-10-04 |

---

## 📖 Overview

A **repaint** lets one mesh set appear in many liveries (and equipment variants) without duplicating the model. The mechanism has three parts:

1. a **hook** in the object's configuration — `[CTC]` plus `[CTCTexture]` entries in a vehicle `model.cfg`, a scenery `.sco` or a human `model.cfg`;
2. **preset files** (`.cti`) in the folder the hook names, each holding named repaints;
3. optional **variables** (`[setvar]`) that a repaint sets so scripts switch equipment.

This page specifies the chain end to end. The `.cti` grammar itself is in [misc types](misc-types.md#cti); the `[CTC]`/`[CTCTexture]` keyword lines are in [`model_cfg`](vehicles/model_cfg.md) and [`sco`](sco.md). Evidence tags: **[C]** counted over the corpus, **[H]** seen in files, ❓ inferred. No behaviour was tested in OMSI.

### Scale [C]

| Item | Count |
| :--- | ---: |
| `.cti` files | 3,152 (2,978 in `Vehicles/*/Texture/…`, 22 in `Humans/*/texture`, ~50 in scenery) |
| `[item]` blocks | ≈ 22,500 (≈ 7.6 per repaint name on average; one file has 684) |
| distinct repaint names | 2,949 |
| folders containing `.cti` | 896 (median 1 file, maximum 42) |
| `[CTC]` blocks | 1,745 (1,513 in `.cfg`, 232 in `.sco`) |
| `[CTCTexture]` blocks | 10,453 in `.cfg`, 243 in `.sco` |
| `[setvar]` lines | 58,602 (719 distinct variable names) |
| `[visible]` lines | 73 (in 11 files) |

## 🔤 General Structure

### The chain

```
object config                       preset folder                          result
─────────────                       ─────────────                          ──────
[CTC]                               Texture\Werbung_EN92\
  Colorscheme                         BSAG 4306.cti
  Texture\Werbung_EN92   ───────►       [item] BSAG 4306 / farbschema_tex1 / EN92_1_BSAG_4306.dds
  0                                     [item] BSAG 4306 / farbschema_trans / BSAG_tr.tga
[CTCTexture] farbschema_tex1 / EN92_1.tga   ◄── slot + default texture       [setvar] <variable> / <value>
[CTCTexture] farbschema_trans / trans.tga
```

1. **`[CTC]`** has **three lines**: a *name*, a *texture folder* (relative path, backslashes), a *default index*. In **1,725 of 1,745** blocks all three are present and the default is `0`; 20 omit the third. The name is `Colorscheme` in **1,542** blocks (88 %) — also the name of an engine-predefined variable ([predefined variables](vehicles/predefined-variables.md)) — and otherwise a free label (`Lackierung` 73, `Werbung` 69, `TextureNumber` 27, `Holz`, `Farbe`, `Dach`, and for scenery `ad`). A config has a median of 1 `[CTC]` (maximum 3): one switch for the whole body, or separate ones for body, advert and interior.
2. **`[CTCTexture]`** follows its `[CTC]` and has **two lines**: a *slot name* and the **default texture file** (the one used when the default index is chosen). A `[CTC]` has a median of 1 slot, up to 50.
3. **Slots** are the names a preset targets. In the `.cti` files the most used slot names are `farbschema_tex1` (3,282 items), `farbschema` (2,583), `farbschema_tex2` (1,991), `farbschema_tex4` (1,112), `farbschema_tex3` (668), `farbschema_tex9` (595), `farbschema_tex5` (528), `Wagenkasten` (471), `farbschema_tex8` (435), `WerbungInnen` (410). The numbered `farbschema_texN` slots correspond to the body textures of the model. Slot names are matched **by string**: 3,017 of 3,704 distinct (file, slot) pairs found in resolvable folders are declared by the owning config (81 %; the rest sit in folders my lookup could not resolve ❓).
4. **Folder resolution:** the folder is relative to the object (`texture\busstop_ad` for a scenery stop), or leaves it with `..` (`..\..\..\..\Vehicles\MB_T1\texture`: a scenery transporter reuses a vehicle's presets). 1,204 of 1,743 folders resolve with the four base folders tried (root, object folder, its parent, grand-parent) ❓.
5. **`.cti` files** in that folder are scanned and **merged**: any number of files, any number of `[item]` blocks (text in the files themselves). Several `[item]` blocks with the **same item name** form one repaint (e.g. `BSAG 4306` fills `farbschema_tex1`, `farbschema_trans` and `farbschema_seat1`). Item names may have leading spaces in the file (`" Beige (AL)"`) — trim them ❓. A file may be switched off by renaming it `.off` ([off](../off.md)).
6. **Default index `0`** selects the textures declared by `[CTCTexture]` (the unpainted state) ❓.

### Presets also configure equipment

`[setvar] <variable> <value>` lines (58,602) set **script variables when the repaint is chosen**. The variable names are not colours but **equipment options**: `display_ext` (1,304 uses), `cg_inw` (1,173), `validator_rear` (1,168), `ibis_type` (1,150), `air_conditioning` (1,114), `validator_front` (1,110), `driver_cabin` (1,094), `language_ibis` (1,042), `language_vdv` (1,040), `front_light` (1,028), `reg_number_window_sticker` (928), `fahrgastpendel_type` (892), `exterior_emergency_door` (875), `suspension_lift` (875). Values are small integers: `1` ×35,645, `2` ×9,101, `0` ×7,314, `3` ×3,788, `4` ×996, `5` ×909. So a "repaint" is really a **vehicle variant preset**: livery textures plus option numbers the scripts read (cf. [script sets](vehicles/script-sets.md)). `[visible] <variable> <value>` (73) similarly toggles mesh visibility variables ([scenery scripts](scenery-scripts.md)).

### How a repaint gets chosen

- **By name:** the ailist depot rows carry the repaint name in their third column ([lists](../map/lists.md)); this is a string match against `[item]` names ❓.
- **By the user:** the vehicle-selection dialog lists the repaints found in the CTC folder ❓ (UI, not testable from files).
- **In maps:** static vehicles placed as scenery objects store the repaint **texture file names** as per-object string variables (e.g. `Repaint_C2\…dds`), see [tiles](../map/tiles.md) `[object]`.
- No `.bus`/`.ovh` keyword names a repaint (a census of all keywords containing *paint*, *lack*, *farb*, *ctc*, *colour*: 0 hits).

### Authoring side: templates and the RepaintTool

- `Vehicles/<v>/Templates/` (4 vehicles: MB_O305G, CW_IVECOBUS_INTER, CW_IRISBUS_LOWENTRY, CW_IVECOBUS_LOWENTRY): template bitmaps, PDFs and `.rpc` files.
- `SDK/RepaintTool/<MAN D | MAN NG 272 | MAN SD>/`: the tool, template bitmaps (133) and **142 `.rpc` files** in the install.
- **`.rpc` grammar [H]:** a text file whose **first five lines are the template bitmaps in a fixed order — Basic, Alpha, Mask, Add, Multiply** (relative paths such as `templates\D86_02_BS.bmp`; a blank line means "none"); **everything after line five is ignored** and used for comments (the shipped files add a title and a reference value `170` in the prose). 133 of 142 files show five template lines within their first six non-empty lines, 9 show four (one slot blank).
- Top-level `Repaints/` and `Temp_City_RegionBus_*` hold `.pdn` authoring files (not loaded).

## 🧪 Examples

Three traced chains (paraphrased from real files; no text copied):

1. **Vehicle (MAN EN92):** `model_EN92.cfg` → `[CTC] Colorscheme / Texture\Werbung_EN92 / 0`; slots `farbschema_tex1` (default `EN92_1.tga`), `farbschema_trans` (`trans.tga`), `farbschema_wheelF` (`GN92_Rad_v.bmp`), … → folder holds one `.cti` per livery, e.g. one that fills `farbschema_tex1` and `farbschema_trans`, another `farbschema_tex1`, `farbschema_trans` and `farbschema_seat1`.
2. **Scenery (bus-stop shelter):** `busstop_hut.sco` → `[CTC] ad / texture\busstop_ad / 0`; `ads.cti` with slot `farbschema` supplies the advert textures.
3. **Human:** `Humans/aXYZ/model/man02.cfg` → `[CTC] Colorscheme / Texture\man02 / 0`; `Humans/aXYZ/texture/…/*.cti` give clothing variants.

A preset with equipment options (descriptive): `[item] HVL / farbschema_tex1 / EN92_1_HVL.dds` … `[setvar] vis_wimpel_invisible / 1`.

## ❓ Open Questions

| ID | Question | Priority |
| :--- | :--- | :--- |
| Q-RP01 | What does the `[CTC]` name (`Colorscheme`, `ad`, `Lackierung`) bind to — a script variable that stores the chosen index? Is the index the position in a sorted list of repaint names? | 🔴 High |
| Q-RP02 | Does default index `0` mean "the `[CTCTexture]` defaults" or "the first repaint alphabetically"? | 🟡 Medium |
| Q-RP03 | Are item names compared case-sensitively and with the leading spaces seen in files? | 🟡 Medium |
| Q-RP04 | In which order are `.cti` files in a folder merged, and what happens on a duplicate item+slot? | 🟡 Medium |
| Q-RP05 | Which of the 719 `[setvar]` variables are engine-predefined vs vehicle-declared? (They are mostly declared in the vehicle's own varlists — a join of the two would settle it.) | 🟡 Medium |
| Q-RP06 | `model_cfg` Q-011 (`[CTC]` parameters) is answered here: name, folder, default index. | ✅ |

## 📝 Provenance

- **Corpus (2026-10-04):** all 3,152 `.cti` (items, slots, `[setvar]`, `[visible]`); all `[CTC]`/`[CTCTexture]` blocks in `.cfg`/`.sco` (1,810 files); folder and slot consistency check between configs and the `.cti` files they point at; census of `.bus`/`.ovh` keywords for repaint-related names; `.rpc` grammar read from the shipped files; template folders listed.
- **Not tested:** how OMSI selects the repaint or what `Colorscheme` stores.

**Clean-room attestation (ADR-0002):**
- No OMSI binaries were inspected.
- No GPL source code was read.
- No OMSI assets were committed.

## 🔗 Related Documents

- [misc types (`.cti`)](misc-types.md#cti), [`model.cfg`](vehicles/model_cfg.md), [`.sco`](sco.md), [lists (ailists)](../map/lists.md), [predefined variables](vehicles/predefined-variables.md), [`.off`](../off.md)
