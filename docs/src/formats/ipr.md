# `.ipr` — Internet Texture Dates

| Metadata | Value |
| :--- | :--- |
| **Format Family** | Text |
| **File Extensions** | `.ipr` (10 files under `Texture/`, each next to an `.itx`) |
| **Parser Crate** | `opts-core` (not yet planned) |
| **Parser Module** | n/a — stub |
| **Encoding** | UTF-8 / ASCII |
| **Status** | 🟡 Stub — written by the P-100 reconnaissance pass; observation only |
| **Last Updated** | 2026-10-04 |

> ⚠️ **Stub.** Everything below is observation from a handful of sample files in the OMSI 2 installation. Nothing was tested in OMSI. Statements marked ❓ are guesses.

---

## 📖 Overview

An `.ipr` file accompanies an [`.itx`](itx.md) of the same base name (10 of 10 pairs). It contains **HTTP-style dates** (`Mon, 23 Jul 2018 17:49:43 GMT`) separated by blank lines. The number of dates equals the number of URL entries in the `.itx` for 7 of the 9 pairs that have both (e.g. 24 and 24, 11 and 11), so each date most likely belongs to one URL entry — a **"last modified" record** used to decide whether a download is up to date ❓.

## 🔤 General Structure

Blank-line-separated records; each non-blank line is a date in the RFC 1123 form used by HTTP `Last-Modified` headers. Exceptions: `Palma.ipr` and `Bremen-Nord.ipr` contain no dates although `Palma.itx` lists 11 URLs; `RUHR_BUS.ipr` has 10 dates for 11 URLs.

## 🧪 Examples

```
Mon, 23 Jul 2018 17:49:43 GMT
```

## ❓ Open Questions

| ID | Question | Priority |
| :--- | :--- | :--- |
| Q-01 | Meaning of each date and what happens when it is missing (Palma, Bremen-Nord). | 🟢 Low |

## 📝 Provenance

- `Texture/standard.ipr` (smallest), `Texture/va_wien_1_texturen.ipr` (largest); date counts compared with `.itx` URL counts for all pairs.

**Clean-room attestation (ADR-0002):**
- No OMSI binaries were inspected.
- No GPL source code was read.
- No OMSI assets were committed.

## 🔗 Related Documents

- [`.itx`](itx.md)
