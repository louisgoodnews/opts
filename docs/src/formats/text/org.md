# `.org` — Number and Registration Lists

| Metadata | Value |
| :--- | :--- |
| **Format Family** | Text (plain line list) |
| **File Extensions** | `.org` |
| **Parser Crate** | `opts-core` |
| **Parser Module** | `opts_core::formats::org` |
| **Encoding** | ASCII / UTF-8 (380 of 385), CP1252 (5) |
| **Status** | 🟢 Specified |
| **Last Updated** | 2026-10-06 |

---

## 📖 Overview

An `.org` file is a **pool of identifiers** a vehicle draws from when it is created: fleet numbers or number plates. A vehicle definition ([`.bus`](vehicles/bus.md) / `.ovh`) names the list with a keyword; each spawned vehicle receives one entry.

385 files exist. Typical names: `regs.org` (27×), `Regs_Pol.org`, `Regs_Taxi.org`, `Regs_A3L_67_K.org`, `Fahrzeugnummern_ASEAG.org`, `Kennzeichen_ASEAG.org`, `numbers.org`, `Registrations.org`.

Evidence tags: **[C]** counted over the whole corpus, **[H]** seen in files, **[M]** observed in-game, ❓ unverified.

### Referenced from

| Keyword in `.bus` / `.ovh` | References | Meaning |
| :--- | ---: | :--- |
| `[number]` | 1,326 | The list the **fleet/line number** is picked from (`Regs_A3L_67_K.org`). |
| `[registration_list]` | 121 | The list of **registration plates** (`Regs_Std_77.org`). |

The path is relative to the vehicle's folder. [C]

---

## 🔤 General Structure

One identifier per line, no keywords, no comments, `CRLF`.

```
HH-7800
HH-7801
HH-7802
```

```
187-1
188-1
189-1
```

```
4916
4917
4918
```

Line shapes over all files (digits → `9`, letters → `a`; first 400 lines of each) [C]:

| Shape | Lines |
| :--- | ---: |
| `9999` (four digits) | 9,363 |
| `999` | 2,508 |
| `aa-aa 9999` (`HH-KT 1234`) | 1,489 |
| `aaa-999` | 1,417 |
| `aa-9999` | 1,215 |
| `9999.a` (`4752.a`) | 821 |
| `aa-a 9999` | 630 |
| `aa 99-99` | 466 |
| `aa99 aaa` | 398 |
| `99` | 283 |
| `a-aa 9999` | 253 |
| `99-aaa-9` | 238 |
| `999 999-9` | 225 |

So entries are **free text**: number plates in many national formats, fleet numbers, and numbers with a suffix. Spaces and `-` occur inside entries. A reader must keep the line verbatim (trimmed of `CR`/`LF`).

Observed variants [H]:

- **Repeated entries.** `E1_1.org` has the single value `4752` on all 88 lines (fixed number, one per vehicle in a train set). Repetition is allowed and acts as a weight (see [Selection Semantics](#-selection-semantics)).
- **Descending or arbitrary order** (`883, 881, 879, …`); the order is not required to be sorted.
- **Single-line files** (`regs.org` containing `1`), so a list may offer exactly one value.
- Files up to 786 KB exist (very long plate lists); one `*.ttl.org` file (`maps/HamburgLi20/TTData/2 - Kopie.ttl.org`) is a **backup copy of a timetable** `.ttl` (see [timetables](../map/timetables.md)) that merely carries the same extension; it is not a registration list.

### Error handling

| Condition | Behaviour |
| :--- | :--- |
| Blank line | Skip. |
| Trailing whitespace / `CR` | Trim. |
| Non-ASCII byte | Decode as UTF-8, fall back to CP1252. |
| File missing | Warn; vehicle gets no number. |

---

## 🎲 Selection Semantics

> ✅ **Resolved (2026-10-06):** The behavioral semantics of `.org` files are as follows.

### Pick Order

Entries are **picked at random**, not sequentially. The order in the file has no effect on selection order.

### Weighting by Repetition

Repeating an entry in the file **raises the statistical probability** of that entry being picked. `E1_1.org` with 88 identical lines (`4752`) therefore guarantees that value every time — not because the entry is special, but because it occupies the entire pool.

This is the idiomatic way to express weighting: list an entry more often to make it more likely.

### Consumption

Entries are **not consumed**. A value already used by one vehicle can be picked again by another. Uniqueness is not enforced by the `.org` mechanism.

### Interaction with In-Game UI

When a player selects a vehicle in OMSI's UI, the contents of the `.org` file are **proposed as default registrations** and **pre-filled** into the vehicle's registration field. The player can then accept the proposal or override it.

This means `.org` acts as an **advisory list of sensible defaults**, not a hard constraint. In free-play mode the player can type any value; the file simply provides a starting point.

### Interaction with Registration Keywords

| Keyword | Effect on `.org` behaviour |
| :--- | :--- |
| `[number]` | Selects the fleet-number list. Values from this list become the vehicle's internal number. |
| `[registration_list]` | Selects the plate list. Values from this list are proposed as default plates. |
| `[registration_automatic]` | Uses the prefix/postfix system with numbers drawn from the list. |
| `[registration_free]` | In free-play mode, the player may choose any plate; the `.org` value is only a default. |

The exact precedence between these keywords is documented in [`.bus` — Vehicle Definitions](vehicles/bus.md).

> ⚠️ **Not yet documented:** the `S.L.kennzeichengroesse` script variable. It may control the plate-size rendering or an auto-formatting behaviour. Recorded here as a potential follow-up.

---

## 🧪 Examples

A list of three plates, expected to produce three candidate values. Each spawn draws one of them at random with equal probability (since none is repeated):

```
B-KL 7093
B-PU 8018
B-KH 3463
```

A weighted list — `A` is three times more likely than `B` because it appears three times:

```
A
A
A
B
```

---

## ✅ Resolved Questions

| ID | Question | Resolution |
| :--- | :--- | :--- |
| Q-ORG01 | Are entries picked sequentially, randomly, or does repetition weight the draw? Is a used entry consumed? | **Randomly.** Repetition weights the draw. Entries are **not consumed** — duplicates across vehicles are possible. Contents are proposed in-game as defaults and pre-filled when the vehicle is selected. |
| Q-ORG02 | Maximum line length / number of lines OMSI accepts. | **None known.** No observed upper bound. |
| Q-ORG03 | Interaction with `[registration_automatic]`, `[registration_free]` and `S.L.kennzeichengroesse`. | **Covered by Q-ORG01.** The `.org` contents are the proposed defaults; registration keywords select which list to use and whether the player may override. `S.L.kennzeichengroesse` remains undocumented — see [Open Questions](#-open-questions). |

---

## ❓ Open Questions

| ID | Question | Priority |
| :--- | :--- | :--- |
| Q-ORG04 | What does the `S.L.kennzeichengroesse` script variable control? | 🟢 Low |
| Q-ORG05 | Does OMSI enforce any cross-vehicle uniqueness for fleet numbers in AI mode, or does it allow collisions? | 🟢 Low |

---

## 📝 Provenance

- **Corpus:** `/…/steamapps/common/OMSI 2/`, entire tree, scanned 2026-10-04: 385 `.org` files.
- **Files inspected:** `Vehicles/Halycon_AI_Pack_7/regs.org`, `Vehicles/VA_E1_c3/E1_1.org`, `Vehicles/HC_HHA_DT4/Regs_DT4.5_1.org`, `Vehicles/Muenchen_MAN_LC/Regs_Muenchen.org`, `Vehicles/HC_KIPKW5/Regs_Pol.org`, `Vehicles/A3/Regs_A3L_67_K.org`.
- **Techniques:** line-shape census; references found by scanning `.bus`, `.ovh`, `.cfg`, `.sco`, `.dsc` files for lines ending in `.org` and recording the preceding keyword.
- **Project-lead clarification (2026-10-06):** Selection semantics, weighting by repetition, non-consumption, and in-game default proposal behaviour confirmed by the OPTS project lead based on modding experience.

---

**Clean-room attestation (ADR-0002):**
- No OMSI binaries were inspected.
- No GPL source code was read.
- No OMSI assets were committed.

## 🔗 Related Documents

- [`.bus` — Vehicle Definitions](vehicles/bus.md)
- [Map Lists & Calendars](../map/lists.md) — `registrations.txt` (a similar plain list at map level)
- [Misc. Types](misc-types.md) — `.ovh`