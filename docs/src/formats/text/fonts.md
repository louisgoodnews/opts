# Fonts — The `Fonts/` Registry

| Metadata | Value |
| :--- | :--- |
| **Format Family** | Text (`.oft`) + bitmaps; lookup convention |
| **File Extensions** | `.oft` (1,056), font bitmaps `.bmp` |
| **Parser Crate** | `opts-core` |
| **Parser Module** | `opts_core::formats::oft` |
| **Encoding** | CP1252 / UTF-8 / UTF-16LE (per file) |
| **Status** | 🟡 Draft — registry behaviour inferred from statistics |
| **Last Updated** | 2026-10-04 |

---

## 📖 Overview

OMSI renders text on displays, destination signs and bus-stop boards from **bitmap fonts**. A font is a pair of bitmaps plus an `.oft` file that says where each character sits. The `.oft` grammar is in [misc types](misc-types.md#oft); this page adds the **registry**: how fonts are found, named and shared. Evidence tags: **[C]** counted over the corpus, **[H]** seen in files, 📚 community wiki, ❓ inferred.

## 🔤 General Structure

### Where fonts live [C]

`Fonts/` is a flat folder of **2,095 files**: 1,053 `.oft`, about 1,020 bitmaps (`.bmp`, a few `.tga`/`.png`), plus `.bak` leftovers, and two sub-folders (`St-Servan` with 2 `.oft`, `Roboto`). One further `.oft` sits in an add-on folder (`Addons/Thueringer_Wald/Sonstiges/`). Naming is informal: `<Name>.oft` + `<Name>.bmp` + `<Name>_alpha.bmp`, often prefixed by a number (`17_HHAschedule_font.oft`) for the project that made it.

### Definition

One `.oft` normally defines **one font**: `[newfont]` (5 lines: *name*, *colour bitmap*, *alpha bitmap*, *line height in pixels incl. spacing*, *horizontal gap*) followed by `[char]` entries (4 lines: *character*, *x start*, *x end*, *y top*; the cells are inked edge to edge and the *horizontal gap* goes between them, checked 2026-10-26 on `DIN Narrow` and the Krüger fonts). 12 files define several fonts (1,128 `[newfont]` blocks in 1,056 files).

| Property | Observed [C] |
| :--- | :--- |
| characters per font | median 81, maximum 791, minimum 1 |
| line height | `32` (105 fonts), `16` (86), `64` (39), `20` (36), `8` (36), `19` (35) |
| gap | `2` (346), `1` (338), `0` (153), `8` (64) |
| bitmap type | `.bmp` for 1,127 of 1,128 |
| bitmap and alpha present next to the `.oft` | 1,096 of 1,128 (97 %) |
| alpha bitmap has the **same pixel size** as the colour bitmap | 1,047 of 1,096 pairs (95.5 %) — the wiki states it must 📚; 49 pairs differ |

### Lookup by **name**, not by path

Text elements refer to a font by its `[newfont]` **name**: of 32,373 font-name lines in `[texttexture]` / `[texttexture_enh]` blocks (`.sco`, `.cfg`), **31,672 (97.8 %) equal a defined font name**; the rest are template placeholders. Scripts do the same through the script-texture API (`GetFontIndex`, `TextLength`, `STTextOut`: [predefined variables](vehicles/predefined-variables.md)), and string variables such as `Font_7x4/14x8B` carry font names ([variable lists](vehicles/varnamelist.md)). The effect: **the folder is a global registry**: any `.oft` found defines a name usable everywhere.

- **1,094 distinct names** are defined by 1,128 definitions; **21 names are defined twice** (e.g. `DIN Narrow` in `Fonts/` and in the Thuringia add-on, `Myriad Pro gras` in `Fonts/` and `Fonts/St-Servan/`, `StreetHamburg2` in two files). Which definition wins is unknown ❓.
- Names are compared case-insensitively in the corpus (`din narrow` ↔ `DIN Narrow`) ❓.
- Sub-folders (`St-Servan`) are scanned too ✅ (their fonts are referenced by name like the rest).

### Parser status [C]

`opts_core::formats::oft::OftFile` parses all 1,055 `.oft` files (1,127 `[newfont]` blocks, 84,013 glyph cells). Many files start with an **indented tutorial** that quotes `[newfont]`/`[char]`; keywords count only at column 0. The first `[char]` line may be a space character. Text rendering: [text textures](../../architecture/text-textures.md).

## 🧪 Examples

```
Fonts\<Name>.oft:   [newfont]  DIN Narrow / DIN_Narrow_Alpha.bmp / DIN_Narrow_Alpha.bmp / 56 / 4
                    [char] ...  (one per glyph: char, x from, x to, y top)
.sco:               [texttexture] <string #> / DIN Narrow / <width> / <height> / <mono|colour> / R / G / B
```

(`[texttexture]` field order from [sco](sco.md).)

## ❓ Open Questions

| ID | Question | Priority |
| :--- | :--- | :--- |
| Q-FN01 | Which of two same-named fonts is used (first scanned, last scanned)? Test with two `.oft` defining one name with different bitmaps. | 🟡 Medium |
| Q-FN02 | Which folders are scanned: only `Fonts/` and sub-folders, or also the `.oft` in add-on folders (`Addons/…`)? | 🟡 Medium |
| Q-FN03 | Behaviour when alpha and colour bitmap sizes differ (4.5 % of pairs). | 🟢 Low |
| Q-FN04 | Character encoding of the `[char]` first line (the wiki says up to 110 codes on a German keyboard; the corpus has fonts with 791 entries). | 🟢 Low |

## 📝 Provenance

- **Corpus (2026-10-04):** all 1,056 `.oft` (definitions, characters, heights, gaps), file resolution and bitmap-size comparison (1,096 pairs, BMP headers only), duplicate-name census, `[texttexture]` font-name resolution over `.sco`/`.cfg` (32,373 lines).

**Clean-room attestation (ADR-0002):**
- No OMSI binaries were inspected.
- No GPL source code was read.
- No OMSI assets were committed.

## 🔗 Related Documents

- [misc types (`.oft`)](misc-types.md#oft), [`.sco`](sco.md), [predefined variables](vehicles/predefined-variables.md), [gap analysis](../gap-analysis.md)
