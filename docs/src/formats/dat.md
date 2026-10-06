# `.dat` — Star Catalogue (`helper/stars.dat`)

| Metadata | Value |
| :--- | :--- |
| **Format Family** | Text (tab-separated table) |
| **File Extensions** | `.dat` (1 file in the installation) |
| **Parser Crate** | `opts-core` (not yet planned) |
| **Parser Module** | n/a — stub |
| **Encoding** | ASCII, LF |
| **Status** | 🟡 Stub — written by the P-100 reconnaissance pass; observation only |
| **Last Updated** | 2026-10-04 |

> ⚠️ **Stub.** Everything below is observation from a handful of sample files in the OMSI 2 installation. Nothing was tested in OMSI. Statements marked ❓ are guesses.

---

## 📖 Overview

The only `.dat` file is `helper/stars.dat` (140,576 bytes). It is a plain table of **5,079 stars**, one per line, with four tab-separated columns. It lives in OMSI's `helper` folder next to other engine helper data, and is presumably the source of the **night-sky star field** ❓ (no in-file documentation exists).

## 🔤 General Structure

Four columns on every one of the 5,079 lines:

| Col | Content (observed) | Range |
| ---: | :--- | :--- |
| 1 | a number ❓ right ascension in degrees | 0.27 – 359.98 |
| 2 | a number ❓ declination in degrees | −88.96 – +89.26 |
| 3 | a decimal number ❓ brightness (descending near the top: 3.84, 1.94, 1.04 …) | — |
| 4 | one letter ❓ spectral class | K 1,268 · B 1,056 · A 990 · G 710 · F 669 · M 335 · O 37 · C 9 |

The first rows match the brightest stars of the real sky (e.g. `101.29 −16.72` and `95.99 −52.7`, `213.92 19.18` are the positions of three well-known bright stars), which supports the right-ascension/declination reading; this identification is the author's astronomical knowledge, not file documentation ❓.

## 🧪 Examples

```
101.29	-16.72	3.837072455	A
95.99	-52.7	1.940885878	F
213.92	19.18	1.037528416	K
```

## ❓ Open Questions

| ID | Question | Priority |
| :--- | :--- | :--- |
| Q-01 | Exact meaning and scale of column 3 (linear brightness? flux?). | 🟡 Medium |
| Q-02 | Epoch of the coordinates and how OMSI maps them to sky position and colour. | 🟢 Low |

## 📝 Provenance

- `helper/stars.dat` (only file; fully tallied).

**Clean-room attestation (ADR-0002):**
- No OMSI binaries were inspected.
- No GPL source code was read.
- No OMSI assets were committed.

## 🔗 Related Documents

- [Inventory](inventory.md)
