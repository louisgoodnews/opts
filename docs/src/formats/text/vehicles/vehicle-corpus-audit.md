# `.hof` / `.osc` / Varlists / Constfiles — Corpus Audit

| Metadata | Value |
| :--- | :--- |
| **Format Family** | Text |
| **File Extensions** | `.hof`, `.osc`, `*_varlist.txt`, `*_stringvarlist.txt`, `*_constfile.txt` |
| **Parser Crate** | `opts-core` |
| **Parser Module** | `opts_core::formats::{hof, osc, varnamelist, constfile}` |
| **Encoding** | CP1252, UTF-8, UTF-8+BOM, UTF-16LE — detect per file |
| **Status** | 🟡 Draft — addendum to [`hof.md`](hof.md), [`osc.md`](osc.md), [`varnamelist.md`](varnamelist.md), [`const.md`](const.md) |
| **Last Updated** | 2026-10-04 |

---

## 📖 Overview

The four vehicle-side drafts ([`hof`](hof.md), [`osc`](osc.md), [`varnamelist`](varnamelist.md), [`const`](const.md)) were written from one vehicle folder (`MAN_NL_NG`). This page checks them against the **whole install** and records what the larger corpus adds. It does not restate the drafts and does not change them.

| Type | Files in install | Draft sample |
| :--- | ---: | :--- |
| `.hof` | **27,729** | ≈ 170 |
| `.osc` | **5,821** | one vehicle's scripts |
| varlists (`*varlist*.txt`) | **5,577** (4,462 float, 1,115 string) | 11 files |
| constfiles (`*constfile*.txt`) | **3,372** | 3 files |

Evidence tags: **[C]** counted over the whole corpus, **[S]** sample statistics, ❓ unverified. No mutation tests in OMSI have been run.

---

## 🔤 General Structure

### Encodings [C]

The drafts assume Windows-1252 only. The corpus shows:

| Type | CP1252 | UTF-8/ASCII | UTF-8 + BOM | **UTF-16LE** |
| :--- | ---: | ---: | ---: | ---: |
| `.hof` | 19,806 | 5,383 | — | **2,540** (2,528 under `Vehicles/`, 12 under `Addons/`) |
| `.osc` | 3,656 | 2,096 | 4 | **65** |
| varlists | 13 | 5,564 | — | — |
| constfiles | 1,168 | 2,201 | 3 | — |

A reader must detect the encoding per file (BOM `FF FE` → UTF-16LE; `EF BB BF` → UTF-8; otherwise UTF-8 if valid, else CP1252). **UTF-16 `.hof` files are the major gap in the draft**: 9 % of all hofs.

---

## 📚 Field Reference

### `.hof`

All 13 keyword families in the draft occur and no new ones were found [C]:

| Keyword | Uses | Files |
| :--- | ---: | ---: |
| `[addbusstop]` | 1,347,046 | 8,717 |
| `[infosystem_trip]` | 1,088,461 | 23,434 |
| `[infosystem_busstop_list]` | 1,088,297 | 23,276 |
| `[addterminus]` | 778,664 | 12,218 |
| `[addterminus_allexit]` | 80,810 | 9,993 |
| `[name]` | 27,729 | 27,729 |
| `[servicetrip]` | 25,809 | 25,809 |
| `[global_strings]` | 23,902 | 23,902 |
| `[addbusstop_list]` | 15,519 | 15,513 |
| `[addterminus_list]` | 15,507 | 15,507 |
| `[end]` | 31,026 | 15,513 |
| `[HOF_SUITE_DIRECTORIES]` / `…_END` | 4,899 each | 4,899 |
| `[infosystem_busstop]` | 2,212 | 158 |

The draft's counts (8,475 `[addbusstop]`, …) are for its sample folder only; the numbers above are corpus-wide. (`[SP]` and `[DLC]` are **prefixes inside** `[HOF_SUITE_DIRECTORIES]` lines, not keywords.)

Checks of draft claims:

- **`[infosystem_trip]` has 4 lines** (route code, route string, terminus code, line string): confirmed in 49,095 of 59,779 sampled records. 10,564 show 5 raw lines because a `................` terminator row follows (an extra comment line, not a parameter); 90 have 3 and 30 have 6. [S]
- **`[infosystem_busstop_list]`: first line is a count N, then N idents**: confirmed in 34,612 of 59,767 records exactly; in 24,818 more the file has additional trailing comment lines (stop-sequence heading, `-----` ruler) after the N idents, so N is still right; in 337 (0.6 %) N is larger than the number of idents ❓ (inconsistent files). [S]
- **Trailing tabs**: spreadsheet exports pad every line with up to 20 tabs (`2301\t\t\t…`); trim cells. [S]

### `.osc`

| Property | Value |
| :--- | :--- |
| Size | 0 B – 303 KB, median ≈ 3.7 KB |
| Macro/block tags (`{…}`) | `{if}` 369,044 · `{endif}` 369,866 · `{else}` 145,155 · `{end}` 82,951 · `{trigger:…}` 51,396 · `{macro:…}` 28,385 · `{frame}` 1,843 · `{init}` 1,325 · `{frame_ai}` 120 |
| Distinct macro names | 1,834 (e.g. `traegheit` ×566, `antrieb_init`, `antrieb_frame`, `door0_calc`) |

`{if}` and `{endif}` counts differ by 822: some `{endif}` occur in comments or scripts nest differently; a reader must not assume exact balance across a file. ❓

**Variable scopes** used in `( … )` expressions, by use count [C]: `L.L` 800,062 · `S.L` 620,095 · `C.L` 110,340 · `M.L` 107,665 · `L.S` 78,218 · `M.V` 66,770 · `T.L` 64,075 · `F.L` 22,634 · `T.F` 724 · `T.S` 6. The draft covers all but `T.S` (6 uses) — see `osc.md` for the meaning of the others.

**String/helper functions** (`$name`) by use count [C]:

| Function | Uses | In draft |
| :--- | ---: | :--- |
| `$d` | 27,861 | ✅ |
| `$SetLengthR` | 19,122 | ✅ |
| **`$cutEnd`** | 13,917 | ❌ |
| `$SetLengthL` | 11,698 | ✅ |
| `$IntToStrEnh` / `$IntToStr` | 9,469 / 9,320 | ✅ |
| `$length` | 6,401 | ✅ |
| `$StrToFloat` | 5,298 | ✅ |
| `$RemoveSpaces` | 3,959 | ✅ |
| **`$cutBegin`** | 3,767 | ❌ |
| **`$msg`** | 331 | ❌ |
| **`$SetLengthC`** | 159 | ❌ |
| **`$SetLengthM`** | 10 | ❌ |
| `$trunc` | 2 | ✅ |
| `$CutEnd` (case variant), `$SetLenghtL` (typo) | 7 / 1 | — |

Observed use of the undocumented functions [H]:

```
(L.S.Year) $IntToStr 2 $cutEnd $StrToFloat 4 % s1     // "1988" → drop last 2 chars → "19"
(L.S.Year) $IntToStr 2 $cutBegin $StrToFloat s1       // "1988" → drop first 2 chars → "88"
l0 1 (M.V.GetTerminusString) $RemoveSpaces 16 $SetLengthC "@" $+     // pad to 16, centred ❓
"Bitte L/S/K eingeben"  30 $SetLengthM                // pad to 30, 'M' mode ❓
"..\..\Anzeigen\Krueger\" (L.$.Matrix_Bitmapfilename) $+ 0 (M.V.STLoadTex) $msg   // ❓
```

- `$cutEnd n` removes the last *n* characters and `$cutBegin n` removes the first *n* (inferred from the year example; ❓ for *n* larger than the string).
- `$SetLengthC` is the centred counterpart of `$SetLengthL` / `$SetLengthR`; `$SetLengthM` ❓.
- `$msg` takes a path/name expression and appears next to texture/announcement loads ❓ semantics; likely a request to the engine.
- Function names are **case-insensitive** (`$CutEnd` occurs) and misspellings occur (`$SetLenghtL`), which OMSI evidently ignores or tolerates ❓.

**Trigger names** (`{trigger:name}`) are plentiful and vehicle-specific; the most common not in the draft: `bus_doorfront0` / `_off` (250 / 246), `bus_doorfront1`, `bus_dooraft`, `kw_m_enginestart` / `_off`, `door0_drag` … `door5_drag`, `bus_doorfront0_external`, `veh_tank`, `cp_microphone` / `_off`, `kw_scheinwerfer_toggle`, `parking_brake_toggle` / `_set` / `_release` / `_mouse_drag` / `_mouse_off`, `cp_batterietrennschalter_toggle`, `horn`. Names are free (an authored string); a parser must not enumerate them.

### Varlists

Over 5,577 files, 204,409 lines [C]:

| Property | Finding |
| :--- | :--- |
| Blank lines | 18,943 — skip; they do not take an index (as in the draft). |
| Leading/trailing blanks on a name | none found |
| Lines starting with `[` | none |
| **Names that are not plain identifiers** | 2,383 lines (e.g. `Font_7x4/14x8B`, `Font_(8-1)x3/14x6`): names can contain `/`, `(`, `)`. The draft's "one identifier per line" holds, but a parser must not validate against `[A-Za-z0-9_]`. |
| **Duplicate names within one file** | 785 repeated lines in **300 files** (220 float lists, 80 string lists), e.g. `matrix_seitenschild` (27 files), `antrieb_retarderstufe` (25), `haltestelle` (18). The draft says names are unique per vehicle; **they are not**. Open: whether a duplicate gets a second index or aliases the first ([Q-AUD-VL01](#-open-questions)). |

### Constfiles

Over 3,372 files [C]:

| Keyword | Uses | Files | Data lines |
| :--- | ---: | ---: | :--- |
| `[const]` | 49,258 | 3,114 | 2 (36,389); 3, 4, 5 occur when a comment follows (9,991 / 1,384 / 948) |
| `[newcurve]` | 15,997 | 2,205 | 1 (15,983) |
| `[pnt]` | 70,144 | 2,205 | 2 (68,282); 3, 5 with trailing comments |

This confirms the draft structure (`[const]` name+value, `[newcurve]` name + `[pnt]` x,y pairs). Draft totals (445/110/496) were for three files.

---

## 🧪 Examples

String functions (self-written, same grammar as real scripts):

```
"1988" 2 $cutBegin          // result "88"
"1988" 2 $cutEnd            // result "19"
"AB" 6 $SetLengthC          // centred in 6 characters ❓ exact padding
```

Varlist with a duplicate (valid in the corpus):

```
cp_dachluke_1
cp_dachluke_2
cp_dachluke_1
```

---

## ❓ Open Questions

| ID | Question | Priority |
| :--- | :--- | :--- |
| Q-AUD-VL01 | Duplicate varlist names: second index or alias? Test: a script that writes to the duplicate name, read it from both. | 🟡 Medium |
| Q-AUD-OS01 | Exact semantics of `$msg`, `$SetLengthM`, `$SetLengthC` padding, and `$cutEnd`/`$cutBegin` bounds. | 🟡 Medium |
| Q-AUD-OS02 | How OMSI treats misspelled functions (`$SetLenghtL`) and unbalanced `{if}`/`{endif}`. | 🟢 Low |
| Q-AUD-HF01 | 337 `[infosystem_busstop_list]` records where N exceeds the idents present: truncate or error? | 🟢 Low |
| Q-AUD-HF02 | Does OMSI read UTF-16 hofs identically to CP1252 ones (same strings, characters outside CP1252)? | 🟡 Medium |

---

## 📝 Provenance

- **Corpus:** `/…/steamapps/common/OMSI 2/`, entire tree, scanned 2026-10-04.
- **Techniques:** per-file encoding detection; keyword census (counts, files, parameter-line counts) over all 27,729 `.hof`, 3,372 constfiles and 5,577 varlists; macro/function/scope census over all 5,821 `.osc`; structural checks of `[infosystem_trip]` and `[infosystem_busstop_list]` over a 1,500-file random sample; duplicate-name and identifier-shape check over all varlists; context dumps of the undocumented functions.
- **Not done:** no run-time behaviour tests; no parse of the script grammar beyond token census.

---

**Clean-room attestation (ADR-0002):**
- No OMSI binaries were inspected.
- No GPL source code was read.
- No OMSI assets were committed.

## 🔗 Related Documents

- [`.hof`](hof.md), [`.osc`](osc.md), [variable lists](varnamelist.md), [constant files](const.md)
- [`.bus`](bus.md), [Misc. types (`.ovh`, `.cti`, …)](../misc-types.md)
