# Texture Property Files — `<texture>.<ext>.cfg`

| Metadata | Value |
| :--- | :--- |
| **Format Family** | Text (keyword-based) |
| **File Names** | `<texture file name>.cfg`, e.g. `str_asphdrk.bmp.cfg`, `gras1.dds.cfg` |
| **Parser Crate** | `opts-core` |
| **Parser Module** | `opts_core::formats::texture_cfg` |
| **Encoding** | UTF-8 / ASCII (12,000 of 12,000) |
| **Status** | 🟢 Specified |
| **Last Updated** | 2026-10-07 |

---

## 📖 Overview

A texture property file sits **next to a texture** and carries the texture's name plus `.cfg`. It does not describe the image; it attaches **behaviour** to it: what kind of surface it is (for tyre sound, grip, rain), whether it gets wet, and whether the terrain maps it.

There are **12,000** of them: `*.bmp.cfg` 7,071, `*.dds.cfg` 4,696, `*.png.cfg` 75, `*.jpg.cfg` 108, `*.tga.cfg` 50. They live in the `Texture` folder and in the `texture` subfolder of scenery objects and splines (`Sceneryobjects/<Pack>/texture/`, `Splines/<Pack>/texture/`). The same name often appears in many packs (`str_asphdrk.bmp.cfg` 441×). A few property files in texture folders carry the `.sli` extension instead (`15m_Pflaster8.bmp.sli`); they use the same keywords. See [`sco-sli-audit.md`](sco-sli-audit.md).

Evidence tags: **[C]** counted over the whole corpus, **[H]** seen in individual files, ❓ unverified. No mutation test in OMSI has been run.

---

## 🔤 General Structure

Keyword file as in [`.sco`](sco.md): `[keyword]` on its own line, parameters on following lines. Whole-file layout varies only by which keywords are present [C]:

| Keyword set | Files |
| :--- | ---: |
| `[surface]` only | 4,623 |
| `[moisture]` + `[surface]` | 2,574 |
| `[moisture]` + `[puddles]` + `[surface]` | 2,372 |
| `[moisture]` + `[puddles]` | 1,876 |
| `[terrainmapping]` only | 314 |
| `[moisture]` only | 216 |
| `[puddles]` + `[surface]` | 18 |
| no keyword (comment only) | 5 |

Some files embed a tutorial (indented lines) listing the surface types; indented keywords there are comments.

---

## 📚 Field Reference

| Keyword | Lines | Uses | Meaning |
| :--- | :--- | ---: | :--- |
| `[surface]` | 1 | 9,588 | **Surface type** number (table below). |
| `[moisture]` | 0 | 7,040 | Flag: the texture **gets wet** (darker/shinier) in rain. |
| `[puddles]` | 0 | 4,266 | Flag: puddles may form on this texture. |
| `[terrainmapping]` | 0 | 312 | Flag: the texture is used as terrain mapping (ground texture). ❓ |

(The census also counts 10, 11 or 19 "lines" for some `[moisture]`/`[puddles]` entries; those are the pasted tutorial text, not parameters.)

### Surface types

From the legend that authors pasted into the files [H]:

| Value | Surface |
| ---: | :--- |
| 0 | asphalt (default) |
| 1 | concrete |
| 2 | cobblestone |
| 3 | dirt |
| 4 | grass |
| 5 | gravel |
| 6 | snow |
| 7 | deep snow |
| 8 | slush |

Distribution of the values actually used (9,588 `[surface]` lines): `6` 2,900; `8` 2,230; `1` 1,883; `0` 1,140; `2` 642; `7` 569; `4` 101; **`15` 44; `5` 23; `16` 20; `3` 14; `11` 8** ❓ — values **above 8** (`11`, `15`, `16`) are not in the legend. [C]

A texture without a property file defaults to ❓ (probably asphalt / dry).

---

## 🧪 Examples

```
[surface]
6
```

Snow surface (e.g. `WinterSnow/str_gehweg05.bmp.cfg`).

```
[moisture]
[puddles]
```

Wet-capable texture with puddles, surface left at default (e.g. `str_asphdrk.bmp.cfg`).

---

## ❓ Open Questions

| ID | Question | Priority |
| :--- | :--- | :--- |
| Q-TC01 | ~~Meaning of surface values 11, 15 and 16~~ — resolved by the community wiki (see below). | ✅ |
| Q-TC02 | Default surface when no property file exists. | 🟡 Medium |
| Q-TC03 | What `[terrainmapping]` changes. | 🟢 Low |
| Q-TC04 | Are `[moisture]` / `[puddles]` independent? (`[puddles]` without `[moisture]` occurs in only 18 files.) | 🟢 Low |

---

## 🌐 Community Cross-Check (M-0006)

Facts marked 📚 come from the OMSI community wiki (`reboot.omsi-webdisk.de`, CC BY-SA 4.0; facts restated, nothing copied), surface-structure page.

The in-file legend stops at 8; the wiki defines the full range 📚: **0** asphalt, **1** concrete, **2** cobblestone, **3** dirt, **4** grass, **5** gravel, **6** snow, **7** deep snow, **8** slush, **9** ice, **10** water, **11** mud, **12** sand, **13** metal, **14** wood, **15** small cobblestone, **16** interlocking pavers. ✅ Every `[surface]` value in the corpus lies in 0–16, so the previously unexplained values 11 (8 uses), 15 (44) and 16 (20) are mud, small cobblestone and pavers. The type affects vehicle handling, force feedback and sound; a companion `.surf` bitmap (red channel only, ±2 cm) adds fine unevenness (see [community cross-check](../community-crosscheck.md)).

## ✅ Resolved Questions

| ID | Resolution |
| :--- | :--- |
| Q-TC01 | Surface values 9–16 = ice, water, mud, sand, metal, wood, small cobblestone, interlocking pavers (📚). |

---

## 📝 Provenance

- **Community cross-check (M-0006, 2026-10-04):** wiki page `7-oberflaechenstrukturen`; corpus check that all `[surface]` values lie in 0–16.

- **Corpus:** `/…/steamapps/common/OMSI 2/`, entire tree, scanned 2026-10-04. 12,000 files matching `*.{bmp,dds,tga,jpg,png}.cfg`.
- **Techniques:** keyword census (counts, keyword-set tally, `[surface]` value tally) over all of them; hex/text dump of representative files including one with the embedded legend.
- **Not done:** no mutation test (e.g. changing the surface type and driving across it).

---

**Clean-room attestation (ADR-0002):**
- No OMSI binaries were inspected.
- No GPL source code was read.
- No OMSI assets were committed.

## 🔗 Related Documents

- [`.sco`](sco.md), [`.sli`](sli.md), [corpus audit](sco-sli-audit.md)
- [Texture Formats](../textures.md)

## OPTS parse spec for `[moisture]` and `[puddles]` (M-0121 proposal)

Status: spec only, drafted from the format notes above. Nothing in the code parses these keys per texture yet.

**Parse rule.** A texture `.cfg` is a keyword file (see the general structure above). `[moisture]` and `[puddles]` are flags: the keyword line alone sets the flag to true, and the file's absence of the keyword sets it to false. A non-empty line after either keyword is not a parameter: the census found 10, 11 or 19 lines after some of these keywords, which are pasted tutorial text (above), so the parser must ignore them with a warning rather than fail. `[surface]` is read separately (its values are not specified in this spec).

**Output on the texture asset.** `wet_capable: bool` (from `[moisture]`), `puddle_capable: bool` (from `[puddles]`), each independent of the other (open question Q-TC04).

**Default.** No `.cfg` file, or no keyword: both flags false. The material stays dry (matches the milestone's "default to dry when unsure").

**Test fixture.** A small synthetic `.cfg` with each keyword combination (none, moisture only, puddles only, both, both plus junk lines), no OMSI file committed.

**Count method (not run).** The counts in the table above were taken over the whole corpus ([C] tags). No script in `scripts/` counts texture-cfg keyword presence: `scripts/content_issues.py` classifies content problems and does not count flags. A repeatable count would be a scan of every `*.cfg` in the `Texture` folder and the `texture` sub-folders, grouped by keyword set, and it would need an OMSI path. It was not run in this pass, so the existing counts are not re-verified.

**Meaning on non-ground objects** stays open (see M-0121 risk table): the observed meaning of `[moisture]` on vehicles and scenery is not documented.
