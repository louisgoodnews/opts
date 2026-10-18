# `.hof` — Depot (Hof) Files

| Metadata | Value |
| :--- | :--- |
| **Format Family** | Text (keyword-based, tab-separated tables) |
| **File Extensions** | `.hof` |
| **Parser Crate** | `opts-core` |
| **Parser Module** | `opts_core::formats::hof` |
| **Encoding** | Windows-1252 (CP1252) |
| **Status** | 🟢 Specified |
| **Last Updated** | 2026-10-07 |

---

## 📖 Overview

A **hof** (German for *depot* or *yard*) file holds the **operating data** for a bus: its destination list, bus-stop names, route and trip definitions. Because this is separated from the vehicle, **one bus model can serve many cities** by swapping the hof.

It supplies the strings shown on:

- the **destination displays** (front and side matrix signs),
- the **roller blinds** (texture file names),
- the **IBIS** driver terminal and passenger info displays.

### Relationship to Other Files

| File | Relationship |
| :--- | :--- |
| `.bus` | Does **not** reference hofs. Hofs live in the `Vehicles` folder, usually a **shared** folder, and declare which vehicles they apply to. |
| [`varnamelist`](varnamelist.md) | Hof strings land in string variables (`Matrix_Terminus`, `IBIS_busstop_name`, …). |
| [`.osc`](osc.md) | IBIS and matrix scripts read the hof tables and write the string variables. |
| `Texture/Linienlisten/`, `Texture/Werbung_*` | Roller-blind textures named in terminus rows. |

### Sample Corpus

`MAN_NL_NG` ships around **170** `.hof` files (many are symbolic links into a shared `Vehicles/.hof Dateien` folder). Largest sample studied: `Spandau 92.hof` (**3,136 lines**).

Keyword totals across all hofs in the folder:

| Keyword | Count |
| :--- | :--- |
| `[addbusstop]` | 8475 |
| `[infosystem_trip]` | 6650 |
| `[infosystem_busstop_list]` | 6649 |
| `[addterminus]` | 4908 |
| `[addterminus_allexit]` | 510 |
| `[end]` | 164 |
| `[name]` | 156 |
| `[servicetrip]` | 147 |
| `[global_strings]` | 135 |
| `[addterminus_list]` / `[addbusstop_list]` | 82 each |
| `[HOF_SUITE_DIRECTORIES]` / `…_END` | 31 each |
| `[SP]` | 14 |
| `[infosystem_busstop]` | 14 |
| `[DLC]` | 13 |

---

## 🔤 General Syntax

| Rule | Description |
| :--- | :--- |
| **Keywords begin a line** | `[` first, `]` last. Trailing **tabs** after `]` are common (spreadsheet export) and must be ignored. |
| **Parameters follow directly** | One per line, unless the keyword takes a **table**. |
| **Everything else is a comment** | Banners, tutorials, `................` rulers, template lines such as `{routecode} (z.B. '540001')`. |
| **Tab-separated tables** | Files are authored in Excel and saved as tab-delimited text. Rows can have many empty trailing cells. |
| **Windows-1252** | Umlauts appear in names (`Altstädter Ring`, `GÜLTIG AB`). |
| **Comments contain keywords** | Documentation inside the file shows examples such as `[infosystem_busstop]` indented with a tab. Because keywords must begin the line, an **indented** keyword is a comment. |

> 🎯 **Parser rule:** a keyword is only recognized when `[` is the **first** character of the line. This is what separates real data from the embedded documentation.

---

## 🏗️ Document Structure

Order conventionally follows this outline; the parser does not enforce it.

1. `[HOF_SUITE_DIRECTORIES]` header (optional)
2. `[name]`, `[servicetrip]`, `[global_strings]`
3. `stringcount_terminus` and the **terminus** table
4. `stringcount_busstop` and the **bus stop** table
5. `[infosystem_trip]` / `[infosystem_busstop_list]` pairs (one per route)

---

## 📚 Keyword Reference

### 🧭 Suite Directories

```
[HOF_SUITE_DIRECTORIES]
Bluebus12
Bollore_Bluebus
[DLC] Studio Polygon Lite
[SP] Studio Polygon 400MMC
[HOF_SUITE_DIRECTORIES_END]
```

Lists **vehicle folders** this hof is valid for. One folder name per line. Entries may carry a **tag prefix** (`[DLC]`, `[SP]`) that marks the source of the vehicle pack. `[DLC]` and `[SP]` entries are **plain list entries** inside the suite block, not blocks of their own, even though they look like keywords.

> ⚠️ Because `[DLC]` / `[SP]` lines start with `[` and end with a non-`]` suffix, they are **not** keywords under the "ends with `]`" rule. They are only recognized inside the suite block.

Outside a suite block, a hof applies to the vehicle folder it was found in.

### 🪪 `[name]`

One line: the display name of the depot, shown in the hof selection list.

```
[name]
Spandau 1992
```

### 🔧 `[servicetrip]`

One line: the destination label for a **service trip** (shown when the driver selects "no service" / "Betriebsfahrt"). Value is a plain string (`Betriebsfahrt`, `Empty`, `www.hochbahn.de`).

### 🌐 `[global_strings]`

| Line | Meaning |
| :--- | :--- |
| 1 | Count N of strings that follow. |
| 2 … N+1 | The strings. |

String slots are fixed in meaning by the file's own header comment:

| Index | Meaning |
| :--- | :--- |
| 0 | Announcements subdirectory |
| 1 | Roller-blind subdirectory |
| 2 | Side-sign subdirectory |
| 3 | IBIS special code for "900" |
| 4 | IBIS special code for "800" |
| 5 | IBIS special code for "500" |

Example (`Spandau 92.hof`): `5`, `Spandau`, `Spandau`, `Spandau_94`, `35`, `36`.

> A short list is legal. Missing trailing slots are empty. Slot meanings 3–5 are Berlin-specific IBIS coding and may be reused differently by other vehicle packs. See [Q-HOF-002](#-open-questions).

### 🏁 Termini

#### `stringcount_terminus`

A **bare keyword with no brackets**. Next line is an integer **N**: the number of string columns per terminus row. Observed values: `4`, `5`, `6`, `7`, `8`, `9`, `26`.

#### Column meaning

Declared by the file (for N = 7):

| Column | Meaning |
| :--- | :--- |
| string0 | IBIS display text **and** roller-blind texture selector |
| string1 | Front sign, line 1 (centered automatically) |
| string2 | Front sign, line 2 |
| string3 | Side sign |
| string4 | Roller-blind texture file |
| string5 | IBIS-2 display (mixed case, max 20 characters) |
| string6 | Additional plate |

Vehicles using fewer columns ignore the rest. Larger N (such as `26`) is used by packs with many display types.

#### `[addterminus_list]`

A **table** of termini, terminated by `[end]`.

Row layout (tab separated):

```
<Code> <Ident> <string0> <string1> … <stringN-1>
```

| Column | Type | Description |
| :--- | :--- | :--- |
| Code | int | Numeric destination code, as entered at the IBIS. |
| Ident | string | Internal identifier / human-readable name. |
| string0..N−1 | string | See above. Empty cells are allowed. |

```
[addterminus_list]
	206	Schulbus	SCHULBUS		SCHULBUS	SCHULBUS	Schulbus.tga	Schulbus
	270	Kaiserdamm	U KAISERDAMM	U-BAHNHOF	KAISERDAMM	U KAISERDAMM	Bln_Kaiserdamm.tga	U Kaiserdamm
[end]
```

> ⚠️ **Row prefixes:** rows may begin with a marker such as `{ALLEX}` before the code (`{ALLEX}	0	Empty`). It flags the row as "all exits" (compare `[addterminus_allexit]`). A row may also **start with a tab** (blank first cell), which is not a marker.

A cell with the single character `#` (used for stations outside the core network: `Bhf Falkensee	#`) means "reuse the roller-blind/side text from elsewhere / not displayed" and is stored literally.

#### `[addterminus]` (legacy, single entry)

Older format. One terminus per block, with an explicit count.

| Line | Description |
| :--- | :--- |
| 1 | Code. |
| 2…N+1 | N strings (N from `stringcount_terminus`). |

Blank lines in the block are **meaningful** (an empty string). A `.....` ruler line ends the entry and is a comment.

#### `[addterminus_allexit]`

Same layout as `[addterminus]`, but marks a destination where **all doors release passengers** (terminus stop). Used for end-of-line stops.

### 🚏 Bus Stops

#### `stringcount_busstop`

Bare keyword followed by an integer: the number of string columns per bus stop. Observed: `4`, `5`, `6`, `9`, `26`.

Column meaning for N = 4–5:

| Column | Meaning |
| :--- | :--- |
| Ident | Lookup key used by routes (see below). |
| string0 | IBIS-1 text |
| string1 | Interior display line 1 |
| string2 | Interior display line 2 |
| string3 | IBIS-2 text |

#### `[addbusstop_list]`

Tab-separated table terminated by `[end]`. First cell is the **Ident**, then N string cells.

```
[addbusstop_list]
Aalemannufer	AALEMANNUFER	Aalemannufer		Aalemannufer
Altstaedter Ring	ALTSTÄDTER RING	Altstädter Ring		Altstädter Ring
[end]
```

#### `[addbusstop]` (legacy, single entry)

| Line | Description |
| :--- | :--- |
| 1 … N | The strings, in column order. |

The first line doubles as the ident. A `.....` line ends the entry.

### 🚌 Routes (Info System)

The info system declares **one block pair per route**.

#### `[infosystem_trip]`

| Line | Type | Description |
| :--- | :--- | :--- |
| 1 | int | Route code (e.g. `13001`). |
| 2 | string | Route string, for the IBIS device. |
| 3 | int | Terminus code, referencing an `[addterminus*]` entry. |
| 4 | string | Line string, as shown on the line display (e.g. `130`). |

```
[infosystem_trip]
13001
URUH-NERV
231
130
```

#### `[infosystem_busstop_list]`

| Line | Description |
| :--- | :--- |
| 1 | Count N of stops. |
| 2…N+1 | **Idents** of stops in travel order. They must exist in the bus-stop table. |

```
[infosystem_busstop_list]
29
U Ruhleben
Krematorium Ruhleben
Polizeidirektion 2
…
```

Each `[infosystem_trip]` is followed (after comments) by exactly one `[infosystem_busstop_list]`.

#### `[infosystem_busstop]`

Appears in the file's **documentation text** as an indented template (`{string0: IBIS-Name}` … `{string4: LCD 2. Zeile rechts}`); it describes the information a stop provides to the info system. In active use it is rare (14 occurrences) and carries the stop's strings in order:

| Line | Meaning |
| :--- | :--- |
| 1 | IBIS name |
| 2 | LCD line 1, left |
| 3 | LCD line 1, right |
| 4 | LCD line 2, left |
| 5 | LCD line 2, right |

---

## 🔗 Related Documents

| From | To | Resolved by |
| :--- | :--- | :--- |
| `[infosystem_trip]` line 3 | Terminus row | Code (integer) |
| `[infosystem_busstop_list]` entries | Bus-stop row | Ident (case-sensitive, trimmed) |
| `[addterminus_list]` column | Texture | File name in the roller-blind folder named by `[global_strings]` slot 1 |

Missing references are non-fatal: show the raw ident or an empty display.

---

## 🧪 Minimal Example

```
[name]
Demo

[servicetrip]
Betriebsfahrt

[global_strings]
1
Demo

stringcount_terminus
4

[addterminus_list]
1	Depot	DEPOT		DEPOT	DEPOT
[end]

stringcount_busstop
4

[addbusstop_list]
Hbf	HBF	Hauptbahnhof		Hauptbahnhof
[end]

[infosystem_trip]
1001
D-H
1
11

[infosystem_busstop_list]
1
Hbf
```

---

## 🛡️ Error Handling

| Condition | Behavior |
| :--- | :--- |
| Table not closed by `[end]` | Parse to EOF or the next keyword; `WARN`. |
| Row has fewer columns than `stringcount_*` | Pad with empty strings. |
| Row has more columns | Ignore the extra cells. |
| Bus-stop ident not found for a route | `WARN`; keep the stop with an empty display. |
| Terminus code not found | `WARN`; show the trip's route string only. |
| Duplicate terminus code | Last definition wins; `WARN`. |
| Non-numeric count | `WARN`; use the number of rows actually found. |
| Encoding errors | Decode as CP1252; never fail on bytes. |

---

## 🧪 Corpus (M-0099)

`opts-core/tests/hof_corpus.rs` (local gate, `OMSI_PATH`): the installation holds 27,779 `.hof` files below `Vehicles/`; they are 296 distinct files (the same depot is shipped into dozens of vehicle folders), all of which parse; 15,891 routes, 85 of them name a terminus code the depot's table does not have, and 3,387 of 272,142 route stops name no row of the stop table (Q-HOF-007).

## 🧪 Test Cases

| ID | Input | Expected |
| :--- | :--- | :--- |
| T-HOF-001 | `[name]\r\nSpandau 1992` | `name = "Spandau 1992"` |
| T-HOF-002 | `[name]\t\t\t\r\nX` (trailing tabs) | Recognized. |
| T-HOF-003 | Indented `[infosystem_trip]` inside a comment | Ignored. |
| T-HOF-004 | `[addbusstop_list]` row with trailing empty cells | Trimmed to N columns. |
| T-HOF-005 | `[HOF_SUITE_DIRECTORIES]` with `[DLC] …` entry | Entry stored as plain text. |
| T-HOF-006 | `[infosystem_busstop_list]` count > rows | Truncated list + warning. |
| T-HOF-007 | Terminus row prefixed `{ALLEX}` | Flag set on row. |

---

## ❓ Open Questions

| ID | Question | Status |
| :--- | :--- | :--- |
| Q-HOF-001 | ~~Exact semantics of `{ALLEX}` prefix and the `#` cell~~ — resolved: `[addterminus_allexit]` / `{ALLEX}` marks drop-off only stops (passengers alight, none board; 📚 M-0006). | ✅ |
| Q-HOF-002 | ~~Are `[global_strings]` slots 3–5 vendor-specific?~~ — resolved: universal OMSI 2 layout maps up to 26 string slots for terminus rows, 9 for stop rows (📚 M-0006). | ✅ |
| Q-HOF-003 | ~~Is `Ident` matching case-sensitive?~~ — resolved: stop names match editor cube names; passengers react to names and IBIS codes (📚 M-0006). | ✅ |
| Q-HOF-004 | Do `[addterminus]` / `[addbusstop]` (single entry) share one terminator rule? | Open; `.....` assumed. |
| Q-HOF-005 | How `[SP]` / `[DLC]` tags affect loading. | Open. |
| Q-HOF-006 | The stop lines of `[infosystem_busstop_list]` may carry empty trailing columns as tabs (a spreadsheet export; 14,000 stops in `Lemmental_Neuenbreid.hof`): the ident is the first column. | ✅ resolved 2026-10-19 in the parser (before: 84,614 of 272,142 route stops did not resolve against the stop table, afterwards 3,387, 1.2 %). |
| Q-HOF-007 | The remaining 3,387 route stops that name no row of the stop table: some carry a suffix after `*` (`Teltow Bahnhof*N37`, `Teltow Bahnhof*S` in `BRT Berlin.hof`, whose table has `Teltow Bahnhof`), some name stops that only the vehicle's own lists define (`Bahnhof Terminus`). The meaning of the `*` suffix (platform or direction marker?) is not known; the corpus test (`opts-core/tests/hof_corpus.rs`) holds the share under 2 %. | Open. |

---

## 🌐 Community Cross-Check (M-0006)

Facts marked 📚 come from the OMSI community wiki (`reboot.omsi-webdisk.de`, CC BY-SA 4.0; facts restated, nothing copied) (hof page, destination-display page, announcements page); ✅ = confirmed in the corpus.

- **Slot assignment for OMSI 2 ("universal" layout)** 📚 — terminus row, up to 26 string slots: 0 IBIS line 1 (16 characters, upper case); 1–3 ANNAX matrix; 4 roller-blind texture file; 5 IBIS line 2 (20 characters); 6 plug-in sign texture; 7–9 Krüger / rear display; 11–12 full matrix; 13–16 LAWO matrix; 17–25 system specific. Stop row, 9 slots: 0 IBIS line 1; 1–2 interior displays; 3 IBIS line 2; 4–8 special. This explains the slot variation noted under [Q-HOF-002](#-open-questions): the drafted slots 3–5 are **not** Berlin-specific but follow this scheme in universal hofs ❓ partly.
- **Rules** 📚: passengers react only to stop **names** and IBIS **codes**, never to strings; terminus codes are 3 digits for IBIS, 4+ digits for non-IBIS devices and ≥ 1000 for plug-in signs; several codes may map to one destination; the stop name should equal the editor's bus-stop cube name or timetable entry; the actual route comes from the editor, the hof only drives displays; strings are tab-separated.
- **Announcements** 📚: audio lives in `Vehicles\Announcements\<folder>`, the folder is registered in the hof, file names equal the stop name exactly (case-sensitive, no `.wav`), and a `_#terminus` suffix marks the final-stop version. ✅ 7,563 WAV files under `Announcements`, 956 with `_#terminus`; formats are mostly 16-bit PCM (22.05–48 kHz, mono or stereo) but 271 are 32-bit float, so "16-bit PCM" is a recommendation, not a limit.
- **Displays** 📚: roller blinds show a texture band (texture needs a black pixel in a corner); ANNAX = 2 × 16 characters + 3-character line display; Busfanat matrix limits lines to 25 characters. Scripts read hof data with `M.V.GetTerminusString` (by index) and `M.V.GetTerminusIndex` (IBIS input → index) ✅ (622 / 342 files); line number and course go through `IBIS_LinieKurs`, `IBIS_Linie_suffix`, `IBIS_TerminusIndex` ✅ (282 / 452 / 503 files).
- ⚠️ **Encoding:** the wiki says hof files must be ANSI (for umlauts). The corpus has **2,540 UTF-16LE** hofs (9 %). See `community-crosscheck.md` Q-CC04.

---

## 📖 Manual Cross-Check (M-0008)

The shipped manual (paraphrased) adds the **player-side view** of the same data: passengers board only if the displayed destination is one they expect; each terminus has its own **roller number and matrix code** (and a different matrix code after 29 May 1994), and the IBIS "Line/Course" input is a five-digit value (three-digit line + two-digit course). See [`community-crosscheck.md`](../../community-crosscheck.md#shipped-manuals-m-0008-review-2026-10-04).

---

## 📝 Provenance

- **Shipped manual (M-0008, 2026-10-04):** destination-code and IBIS line/course facts (player view), paraphrased.
- **Community cross-check (M-0006, 2026-10-04):** wiki pages `14-hofdatei`, `19-fahrtzielanzeiger-an-bussen`, `93-haltestellenansagen`; corpus checks of announcement WAVs (7,563) and macro/variable names in `.osc`/varlists.

---

**Clean-room attestation (ADR-0002):**
- No OMSI binaries were inspected.
- No GPL source code was read.
- No OMSI assets were committed.

## 🔗 Related Documents

- [`.bus` — Vehicle Definition](bus.md)
- [`.osc` — Scripts](osc.md)
- [Variable Lists](varnamelist.md)
- [.wav — Audio Files (Announcements)](../../wav.md)
