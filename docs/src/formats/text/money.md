# Money Systems — `Money/<pack>/*.cfg`

| Metadata | Value |
| :--- | :--- |
| **Format Family** | Text (keyword-based) + meshes |
| **File Extensions** | `.cfg` (55), `.o3d` (246), textures |
| **Parser Crate** | `opts-core` |
| **Parser Module** | `opts_core::formats::money` |
| **Encoding** | ASCII / UTF-8 |
| **Status** | 🟢 Specified (grammar complete; run-time use ❓) |
| **Last Updated** | 2026-10-04 |

---

## 📖 Overview

A **money system** defines the currency a map uses: its coins and banknotes, each with a value and a 3D model. Passengers pay with these objects at the cash desk and the driver returns change from them (callback `GiveChangeCoin`, [predefined variables](vehicles/predefined-variables.md)). The grammar is explained in the files themselves; the map selects a system with `[moneysystem]` in [`global.cfg`](../map/global-cfg.md) or a chrono's `Chrono.cfg` ([timetables](../map/timetables.md)). Evidence tags: **[C]** counted over the corpus, **[H]** seen in files, 📖 shipped manual, ❓ inferred.

## 🔤 General Structure

### Layout [C]

24 pack folders under `Money/`, each with one or more `.cfg`, coin and note meshes (`.o3d`, 246), and textures (`.bmp`/`.tga`/`.dds`/`.jpg`). Packs often hold **one `.cfg` per era**: the Euro packs carry `Euro.cfg`, `Euro2007.cfg`, `Euro2007+.cfg`, `Euro2013.cfg`, `Euro2014.cfg` (6 packs show this pattern), the DM pack `DM.cfg`, `DM_91.cfg`, `DM_92.cfg`, `DM_93.cfg`. The manual notes there was **no 20-pfennig coin** in the simulated DM period 📖 ✅ (the DM config lists 5, 10, 50 Pf and 1, 2, 5 DM).

### Grammar [H]

Three keywords, parameters one per line:

| Keyword | Lines | Meaning | Uses |
| :--- | ---: | :--- | ---: |
| `[currency]` | 2 | **currency name** (`Euro`, `DM`, `CHF`, `Hamburg109_Euro`, …) and the **number of decimals** (`2` in all 55 files) | 55 |
| `[coin]` | 2 | **mesh file** of the coin and its **value** | 464 |
| `[bill]` | 2 | **mesh file** of the note and its value | 134 |

(Three `[bill]` blocks and one `[currency]` have extra lines; these are comments.)

- **Mesh paths** are relative to the `.cfg`'s folder: **all 598 referenced meshes exist** ✅.
- **Values** are decimal numbers in currency units, written with varying precision (`0.1` ×69 and `0.10` ×13 both occur) — parse as floats. Coin values in the corpus: `1`, `2` (82 each), `0.1`, `0.5` (69 each), `0.2` (64), `0.05` (54), `0.01`; larger coins `5` (5 uses); banknote values `5` (73), `10` (60), `20` (1).
- A `.cfg` has **6–11 coins and 2 notes** in the Euro files (the number of coins grows between `Euro.cfg` and `Euro2007+.cfg`).
- **Selection:** all 47 `[moneysystem]` references in `global.cfg`/`Chrono.cfg` resolve to an existing file ✅. Currency name labels are free text (`Hamburg109_Euro` is a pack-specific label of an ordinary Euro system).

## 📚 Field Reference

See the tables above. The change-giving flow is script-side: the `cashdesk` scripts and the `GiveChangeCoin` callback ([script sets](vehicles/script-sets.md)).

## 🧪 Examples

```
[currency]
Euro
2

[coin]
<coin mesh>.o3d
0.05

[bill]
<note mesh>.o3d
5
```

## ❓ Open Questions

| ID | Question | Priority |
| :--- | :--- | :--- |
| Q-MN01 | How OMSI composes a payment from the listed denominations and chooses change (largest-first?). | 🟡 Medium |
| Q-MN02 | Are values compared exactly or rounded to the declared decimals (`0.10` vs `0.1`)? | 🟢 Low |
| Q-MN03 | Can a pack mix an `[bill]` of value 1 or 500? (Only 5/10/20 occur.) | 🟢 Low |

## 📝 Provenance

- **Corpus (2026-10-04):** all 55 money `.cfg` (keywords, values, mesh resolution), pack layout of 24 packs, all `[moneysystem]` references in `global.cfg` and `Chrono.cfg` (47/47 resolve); the grammar legend read from the shipped `DM.cfg`.

**Clean-room attestation (ADR-0002):**
- No OMSI binaries were inspected.
- No GPL source code was read.
- No OMSI assets were committed.

## 🔗 Related Documents

- [`global.cfg`](../map/global-cfg.md), [timetables (chrono)](../map/timetables.md), [ticketpacks](ticketpacks.md), [predefined variables](vehicles/predefined-variables.md), [script sets](vehicles/script-sets.md)
