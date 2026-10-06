# `.sco` / `.sli` — Corpus Audit

| Metadata | Value |
| :--- | :--- |
| **Format Family** | Text (keyword-based) |
| **File Extensions** | `.sco`, `.sli` |
| **Parser Crate** | `opts-core` |
| **Parser Module** | `opts_core::formats::sco`, `opts_core::formats::sli` |
| **Encoding** | CP1252 / UTF-8 mix (see below) |
| **Status** | 🟡 Draft — addendum to [`sco.md`](sco.md) and [`sli.md`](sli.md) |
| **Last Updated** | 2026-10-04 |

---

## 📖 Overview

[`sco.md`](sco.md) and [`sli.md`](sli.md) were drafted from a subset of the install. This page **audits them against the entire OMSI 2 directory tree** and records what the larger corpus adds:

- confirmed claims (so the drafts can be marked 🟢 for those items),
- keywords the drafts do not mention (47 in `.sco`, 3 in `.sli`),
- misspelled keywords OMSI evidently accepts,
- encoding statistics,
- a rule for counting parameters that explains apparent discrepancies.

It does **not** restate the drafts. Existing text is unchanged; this page only adds. Evidence tags: **[C]** count over the whole corpus, **[H]** inspected in individual files, ❓ unverified.

> ⚠️ No mutation test in OMSI has been run. "What a keyword does" below is taken from the files' own embedded documentation or inferred from names and values; it is marked ❓ where inferred.

---

## 🔤 General Structure

### Corpus size

| Type | Files in whole install | Draft states |
| :--- | ---: | ---: |
| `.sco` | **50,499** | 49,762 |
| `.sli` | **13,363** | 12,382 |

The drafts counted only `Sceneryobjects/` and `Splines/`; `.sco`/`.sli` files also occur under other add-on folders and in texture folders (see [`.sli` in texture folders](#sli-files-in-texture-folders)). Absolute keyword counts below therefore exceed those in the drafts. [C]

### Encoding and line endings [C]

| Property | `.sco` (50,499) | `.sli` (13,363) |
| :--- | ---: | ---: |
| Not valid UTF-8 (treat as CP1252) | 10,725 | 2,154 |
| Valid UTF-8 containing non-ASCII | 1,792 | 753 |
| UTF-8 BOM (`EF BB BF`) at file start | 1,133 | 373 |
| Contains `CRLF` | 50,489 | 13,363 |

A parser must (a) strip a leading BOM before looking for the first `[`, (b) decode leniently (UTF-8, falling back to CP1252), (c) accept `CRLF` and also bare `LF` (10 `.sco` files have no CRLF). The BOM is **not** mentioned in either draft.

### Counting parameters

A keyword's parameters are the **non-comment lines up to the next keyword line**. The drafts say this ("read until the next keyword"). A naïve count of non-blank lines is polluted by trailing comment banners (`#### Model Data`, `-----`), which explains apparent oddities. Examples verified [H]:

| Keyword | Naïve count | Real count | Why |
| :--- | ---: | ---: | :--- |
| `[path]` | 15 (5,360×) | 12 | 3 trailing comment lines (`###`, `Model Data`, `###`) |
| `[light_enh_2]` | 25 (408×) | 24 | trailing banner |
| `[path_2]` | 17 (34×) | 14 | trailing banner |

When only **numeric** lines are counted the fixed layouts of the drafts hold:

| Keyword | Expected | Matches |
| :--- | ---: | :--- |
| `.sco [path]` | 12 | 275,782 of 281,145 naïve (5,360 more explained by comment noise) |
| `.sli [profilepnt]` | 4 | 140,160 of 140,967 |
| `.sli [path]` | 5 | 15,591 of 15,592 |
| `.sli [rail_enh]` | 8 | 420 of 420 |
| `.sli [third_rail]` | 6 | 348 of 348 |
| `.sli [path_2]` | 6 | 26 of 26 |
| `.sli [heightprofile]` | 4 | 26,359 of 27,054 (681 have 5 numeric lines, 14 have 8 — embedded tutorials containing sample values, not data) |

[C]

---

## 📚 Field Reference

### `.sco` keywords not documented in `sco.md`

Parameter lines are shown as observed in the files; "(tutorial)" marks keywords that appear at the start of a line **inside a pasted author tutorial**, i.e. not real directives there — a parser's strict "line begins with `[` and ends with `]`" rule will read them anyway, so unknown keywords must be skipped.

| Keyword | Files | Params | Observed values / use |
| :--- | ---: | :--- | :--- |
| `[mass]` | 476 | 1 | Mass of a parked/static vehicle object (`1.5`, `1`, `2`; units of the neighbouring comment `Masse in t:` = tonnes). Appears with `[momentofintertia]` and `[model]`. |
| `[momentofintertia]` | 380 | 3 | Three values (`1.49 0.40 1.56`, or `300 80 300`); some files add a trailing comment. ❓ Moment of inertia about x, y, z. |
| `[model]` | 348 | 1 | Path to a vehicle-style `model.cfg` (`model\model_parked.cfg`): scenery object that embeds a vehicle model. |
| `[tex_detail_factor]` | 233 | 1 | `0.5` (176×), `0.3` (17×). ❓ Texture-resolution analogue of `[detail_factor]`. |
| `[rail_enh]` | 65 | 8 | Same values as `.sli [rail_enh]` (`200 0 30 0.02 7 30 0.02 7`). See [`sli.md`](sli.md). |
| `[third_rail]` | 51 | 6 | As `.sli [third_rail]` (`0 0.686 3 8100 …`, or `±1.120 0.180 3 750 …`). |
| `[terrainhole]` | 161 | 1 | Mesh file cutting a hole in the terrain (`…_hole.o3d`, `…_loch.o3d`). |
| `[terrainmapping]` | 55 | — | Flag. ❓ Terrain texture mapping on the object's mesh. |
| `[matl_allcolor]` | 66 | 14 | Fourteen floats (`1 1 1 1 1 1 1 0 0 0 0 0 …`). ❓ A full colour override for the current material (diffuse/ambient/specular/emissive/power style). |
| `[matl_bumpmap]` | 19 | 2 | `<texture>`, `<strength>` (`0.05`–`0.5`). |
| `[matl_noXwrite]`, `[matl_noYwrite]` | 22 | — | Flags (UV address-mode / write masks ❓). |
| `[matl_texadress_clamp]` | 4 | — | Flag. ❓ Texture-address mode clamp. |
| `[texcoordtransX]`, `[texcoordtransY]` | 9 / 2 | 1 | A **variable name** (`rolltreppe`, `trans1min`, `transdest`): UV scroll driven by a script variable. |
| `[animparent]` | 9 | 1 | Name of a parent animated mesh (`Arm_A`, `Arm_B`, `AR`): hierarchy for `[newanim]`. |
| `[mesh_ident]` | 9 | 1 | Names the current mesh so `[animparent]` can refer to it. |
| `[blockpath]` | 2 | 2 | Two numbers (`15 0`, `5 0`, `2 0`). ❓ Blocks a `[path]` over a distance. |
| `[triggerbox_new]` | 44 | 6–10 | Six numbers (box position/size) followed by sub-parameters. |
| `[triggerbox_setreverb]` | 44 | 2 | Reverb time (s, max 3) and a second value; used after `[triggerbox_new]` for sound reverb zones. |
| `[illumination_interior]` | 7 | 4 | `-1 -1 -1 -1` in all 7 uses. |
| `[heightprofile]` | 1 | 4 | Same four values as `.sli [heightprofile]`. Appears in `.sco` files that embed a spline. |
| `[newfont]` | 1 | 5 | `<name>`, `<bitmap>`, `<alpha bitmap>`, `<size>`, `<n>`: user-defined bitmap font. |
| `[viewpoint]` | 1 | 1 | `0`. |
| `[nolightmapmapping]` | 60 | — | Flag; inverse of `[LightMapMapping]`. |
| `[petrolstation]`, `[depot]`, `[busstop]`, `[carpark_p]`, `[entrypoint]`, `[signal]` | 58 / 54 / 46 / 3 / 1 / 17 | — | Bare flags that classify the object for the editor/AI (fuel stop, depot, stop, parking, …). Several occurrences are inside tutorial text. ❓ |
| `[moisture]`, `[puddles]` | 16 / 6 | — | Occur inside embedded tutorials (surface-type tables), not as directives in `.sco`. (They are real in texture `.cfg` files — see the name-based files pass.) |

### Misspelled and alias keywords

OMSI evidently tolerates these (they occur in shipped, working assets), so a parser must accept them. Counts are `.sco` occurrences. [C]

| Intended keyword | Spellings seen (count) |
| :--- | :--- |
| `[collision_mesh]` | `collisionsmesh` (3,893), `collisioionsmesh` (345), `collisionmesh` (199), `collsion_mesh` (35), `collisionsbox` (2) — canonical `collision_mesh`: 11,741 |
| `[nocollision]` | `nocolission` (172), `nocollsion` (33), `nocollisioion` (14), `nocollission` (8), `no_collision` (6), `noollision` (1) — canonical: 16,820 |
| `[shadow]` | `isshadow` (16), `schadow` (12) (identity of these two as aliases is ❓: both take no parameter) |
| `[friendlyname]` | `friendylname` (3) |
| `[detail_factor]` | `detailfactor` (3) |
| `[nomaplighting]` | `nomaplightning` (2) |
| `[surface]` | `suface` (1) |

Keyword **case** varies (`[NightMapMode]`, `[nightmapmode]`, `[texcoordtransY]`); matching must be case-insensitive. [C]

The collision aliases are common: 4,474 of 16,215 collision-mesh directives (about 28 %) use a non-canonical spelling, so they cannot be treated as rare typos.

### `.sli` keywords not documented in `sli.md`

| Keyword | Files | Params | Notes |
| :--- | ---: | :--- | :--- |
| `[surface]` | 9 | 1 | Surface type number; tutorial values: `0` asphalt (default), `1` concrete, `2` cobblestone, `3` dirt, `4` grass, `5` gravel, `6` snow, `7` deep snow, `8` slush. [H] |
| `[moisture]` | 9 | — | Appears with an embedded tutorial that defines the `[surface]` values. |
| `[matl_envmap]` | 4 | 2 | `<bitmap>`, `<strength>` (`Rail_envmap.bmp 0.25`): as in `.sco`. |

### `.sli` in texture folders

Files such as `Sceneryobjects/HB_76_crossings_splines/texture/15m_Pflaster8.bmp.sli` contain `[surface]` (and the tutorial for it) rather than a profile. These are **per-texture property files**, the `.sli` counterpart of the `*.bmp.cfg` files; see the name-based files pass for that family. A `.sli` parser that expects `[profile]` blocks must not fail when none are present. [H]

---

## 🧪 Examples

Per-texture `.sli` (property file, excerpt of the real directive; the tutorial is omitted):

```
[surface]
2
```

Surface type `2` = cobblestone, per the in-file legend.

Alias handling (descriptive test inputs):

```
[collisionsmesh]
building_coll.o3d
[nocolission]
```

Expected: one `collision_mesh` entry and the `nocollision` flag.

---

## ❓ Open Questions

| ID | Question | Priority |
| :--- | :--- | :--- |
| Q-AUD-001 | What exactly do `[matl_allcolor]` (14 values) and `[mass]` / `[momentofintertia]` control? Needs a test object in OMSI. | 🟡 Medium |
| Q-AUD-002 | `[tex_detail_factor]`: semantics and range. | 🟢 Low |
| Q-AUD-003 | `[triggerbox_new]`: full parameter list (6, 9 or 10 values occur). | 🟡 Medium |
| Q-AUD-004 | Are `isshadow`/`schadow` really aliases of `[shadow]`/`[noshadow]`? (They may be author typos that OMSI ignores.) Test: toggle in OMSI and check shadow. | 🟢 Low |
| Q-AUD-005 | Do the misspelled collision keywords behave identically to `[collision_mesh]` in OMSI, or does OMSI **ignore** them (assets work without collision)? Test: a copy with the misspelling and a vehicle driven into it. | 🔴 High |
| Q-AUD-006 | `[blockpath]`, `[illumination_interior]`, `[viewpoint]`, `[newfont]`: parameter meaning. | 🟢 Low |

---

## 🔄 Update 2026-10-04 (M-0006): corpus-derived layouts and corrections

Facts marked 📚 come from the OMSI community wiki (`reboot.omsi-webdisk.de`, CC BY-SA 4.0; facts restated, nothing copied).

### `[light_enh_2]` field layout (corpus-derived)

Over 3,283 `.sco` uses the **shape** of the parameter lines (n = number, s = text) is:

| Shape | Uses | Reading |
| :--- | ---: | :--- |
| `n`×17, `s`, `n`×5 | 2,134 | variable **name**, no effect bitmap → 23 lines |
| `n`×17, `s`, `n`×5, `s` | 1,091 | variable name **and** effect-bitmap name → 24 lines |
| `n`×23 | 42 | the variable is a plain number, no bitmap |
| `n`×23, `s` | 6 | numeric variable, with bitmap |

Fields: 1–3 position, 4–6 direction, 7–9 up/axis vector, 10 omni, 11 rotating, 12–14 RGB, 15 size, 16 inner angle, 17 outer angle; **18 variable**; 19 factor, 20 z-offset, 21 parameter bit mask, 22 cone flag, 23 time constant; **24 effect bitmap (optional)**. This reproduces the layout already listed in `sco.md` ✅ and resolves Q-SCO-004. ⚠️ The wiki lists **25** parameters, splitting the variable into an activation and a brightness variable and describing fields 7–9 as a "light-cone vector (0 minimum … 2 = 180°)"; neither appears in the files (no shape has two text fields before the factor, and the 7–9 values are unit-axis vectors like `0 0 1`). Recorded as a conflict, `community-crosscheck.md` Q-CC07.

### Corrections to counts above

- **`[matl_alpha]`**: values over 25,508 uses are `2` ×12,119, `1` ×10,290, `0` ×2,201, `3` ×896, `16` ×2 — see `sco.md`.
- **Path types** in `.sco` `[path]`: `0` 204,423, `1` 74,473, `2` 3,185, no `3`; in `.sli` type `3` (aircraft 📚) occurs 4 times.
- **`[newanim]` sub-keys** by use: `origin_rot_z` 128,168; `origin_rot_y` 126,583; `anim_rot` 119,510; `origin_trans` 103,943; `origin_rot_x` 100,433; `anim_trans` 81,271; `origin_from_mesh` 70,269; **`maxspeed` 22,472; `delay` 6,964** (animation speed and damping, 📚).
- **Comments**: apostrophe-led lines — `.osc` 309,888, `.cfg` 5,552, `.txt` 844, `.sli` 499, `.oft` 448, `.sco` 38; a `[DISABLED]`/`[ENABLED]` block (📚) never occurs.

## ✅ Resolved Questions (this page and the specs it audits)

| ID | Resolution |
| :--- | :--- |
| Q-SCO-004 | Layout: 17 numbers, variable, 5 numbers, optional bitmap. |
| Q-SCO-002 | `[rendertype]` values answered by the wiki (see `sco.md`). |

---

## 📝 Provenance

- **Community cross-check (M-0006, 2026-10-04):** wiki pages `8-objekt`, `17-licht`, `92-animationen`, `18-auskommentieren`; `[light_enh_2]` shape census over all `.sco`.
- **Corpus:** `/…/steamapps/common/OMSI 2/`, entire tree, scanned 2026-10-04. 50,499 `.sco` and 13,363 `.sli` files.
- **Techniques:** a scratch scanner (outside the repository) recognised keyword lines (`[` first, `]` last, no leading whitespace), tallied keyword frequency, files per keyword, and parameter-line counts between keywords; encoding/BOM/line-ending tally; context dumps of the first occurrences of every undocumented keyword; numeric-only line counts for the fixed-layout keywords.
- **Cross-check:** documented keywords were taken from the text of the existing drafts.
- **Embedded documentation:** values for `[surface]` come from the tutorial text pasted into the data files themselves.
- **Not done:** no mutation tests; no `.osc` or map-side verification of how paths connect.
- **External tools consulted:** none.

---

**Clean-room attestation (ADR-0002):**
- No OMSI binaries were inspected.
- No GPL source code was read.
- No OMSI assets were committed.

## 🔗 Related Documents

- [`.sco` — Scenery Objects](sco.md)
- [`.sli` — Spline Profiles](sli.md)
- [`.o3d`](../binary/o3d.md), [`.x`](../binary/x.md) — meshes named by `[mesh]`
