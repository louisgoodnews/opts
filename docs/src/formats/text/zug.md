# `.zug` — Train Consist Files

| Metadata | Value |
| :--- | :--- |
| **Format Family** | Text (positional line pairs, no keywords) |
| **File Extensions** | `.zug` |
| **Parser Crate** | `opts-core` |
| **Parser Module** | `opts_core::formats::trains` (`TrainFormation::from_zug`) |
| **Encoding** | UTF-8/ASCII (221 of 223 in `Trains/`), Windows-1252 (2); decoded by `formats::text::decode_bytes`, which also accepts a BOM and UTF-16 |
| **Status** | 🟢 Specified and corpus-checked (2026-10-15) |
| **Last Updated** | 2026-10-15 |

---

## 📖 Overview

A `.zug` ("Zug" = train) lists the cars of one train, front to back. The same file can be named as a vehicle in a map's `ailists.cfg` (53 AI lists do), so OMSI treats it as a vehicle of its own next to `.bus` and `.ovh`. The earlier note in [misc types](misc-types.md#zug) is superseded by this page.

| Where | Files |
| :--- | ---: |
| `Trains/` (flat folder; 16 of them in `Trains/odreid/`) | 223 |
| `Vehicles/` | 14 |
| `Splines/` | 8 |

(2026-10-15 scan; the documented count of an earlier scan was 186 in `Trains/`.)

## 🔤 Syntax

There are **no keywords, no header and no comments**. The file is a sequence of line pairs:

```text
<vehicle>
<orientation>
<vehicle>
<orientation>
```

| Line | Content |
| :--- | :--- |
| vehicle | a vehicle file relative to the OMSI root, backslashes, any case (`vehicles\Bm238\Bpmz291_or.ovh`); or a **group name** without extension and path (`M-Train Cars`), which the map resolves from its `ailists.cfg` |
| orientation | `0` the car runs forward in the train; `1` it is turned around (180° about the vertical axis). Only these two values occur |

- Line endings are CRLF in all 223 files; 153 have no line break after the last line, so a reader must not require one.
- A blank line after the last pair occurs and is harmless.
- Vehicle files are `.ovh` in **100 %** of the file references (1,351 of 1,351); a `.zug` never names a `.bus`.

## 🧪 Corpus facts (`Trains/`, 2026-10-15)

| Measure | Value |
| :--- | ---: |
| Files that are regular (all pairs `.ovh` + `0`/`1`) | 204 |
| Files with group-name entries | 18 (79 entries) |
| Files that are not consists | 1 (`RUHR.zug` is an `.osc` script with the wrong extension) |
| Cars per file | 1 to 21; 2-10 cars in 74 % of the files |
| Entries / distinct vehicle files | 1,430 / 367 (347 resolve in this installation) |
| Turned cars (`1`) | 550 of 1,430 entries; 130 of 222 files have at least one |

The referenced vehicles are almost all self-contained rail cars: 338 of the 347 resolved files have `[type] 2` and `[boogies]`, 19 have `[couple_back]`, none has `[couple_front]`. Each states its own mass and two coupling points (`[coupling_front]`, `[coupling_back]`): the cars of a consist are joined by those points, as `[couple_back]` chains are.

## 🔧 Parser behaviour (`opts-core`)

- Trailing blank lines are removed; an odd last line is dropped with a note (`ZugNote::DroppedLastLine`).
- An orientation other than `0`/`1`, an empty vehicle line, an empty file or more than 256 cars is an error with the line number.
- Text that begins like a script (`{`, `'`, `(M.`, `(L.`) is refused (`ZugError::NotAConsist`), which is how `RUHR.zug` is rejected without looking at the extension of other files.
- A name with a path separator or an `.ovh`/`.bus` extension is `VehicleRef::File`, anything else `VehicleRef::Group`.
- OPTS reads `.zug` files and never writes them into an installation; `TrainFormation::to_zug_text` exists for tests and a future export.

## ❓ Open Questions

| ID | Question |
| :--- | :--- |
| Q-ZUG-01 | How OMSI resolves a group name that exists in no installed map (`U-Bahnwagen`): by a map or add-on that is not installed here, or by another mechanism? |
| Q-ZUG-02 | Whether timetables (`.ttp`) can name a `.zug`. Not searched. |
| Q-ZUG-03 | Orientation of a multi-file vehicle (a flipped chain rather than a flipped car); no `.zug` of the corpus lists one. OPTS mirrors the whole unit. |

## 📝 Provenance

- Corpus scan `/…/OMSI 2/Trains` and the two other folders, 2026-10-15; community cross-check [`122-zugverkehr-in-omsi-2`](../community-crosscheck.md) for the meaning of the orientation value.
- Tests: `crates/opts-core/src/formats/trains/tests.rs` (self-authored fixtures), `crates/opts-core/tests/zug_corpus.rs` (needs `OMSI_PATH`).

**Clean-room attestation (ADR-0002):** no OMSI binary was inspected; no OMSI asset is committed; the page quotes no `.zug` content beyond file-name shapes.

## 🔗 Related Documents

[Train formation design](../../architecture/train-formation-editor.md) · [Research](../../meta/train-formation-research-2026-10-15.md) · [Map lists](../map/lists.md) · [`.bus`](vehicles/bus.md) · [misc types](misc-types.md)
