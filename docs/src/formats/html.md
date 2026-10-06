# `.html` — Tutorial Menu and Documentation Pages

| Metadata | Value |
| :--- | :--- |
| **Format Family** | Text (HTML fragments) |
| **File Extensions** | `.html` (1,317 files) |
| **Parser Crate** | `opts-core` (not yet planned) |
| **Parser Module** | n/a — stub |
| **Encoding** | mostly ISO-8859-1 (869 declare `charset=iso-8859-1`; 448 declare none) |
| **Status** | 🟡 Stub — written by the P-100 reconnaissance pass; observation only |
| **Last Updated** | 2026-10-04 |

> ⚠️ **Stub.** Everything below is observation from a handful of sample files in the OMSI 2 installation. Nothing was tested in OMSI. Statements marked ❓ are guesses.

---

## 📖 Overview

OMSI ships small **HTML pages** as localised text: 1,140 under `Tutorials/<n>/<LANG>/<id>.html` (tutorial instruction texts; `n` = 1–4), 20 `menu_<k>_<LANG>.html` pages in `Tutorials/` (scenario menu texts for DEU/ENG/HUN/POL, next to `menu_<k>.jpg` pictures and the scenario files `FAST.osn`, `LIN.osn`, `SPEZ.osn`, `STRG.osn` described in [`global-cfg`](map/global-cfg.md)), and 157 documentation pages of one add-on (`Addons/Saint-Servan/Documentation`). They are **not** a spec target for rendering; they matter if OPTS wants to show tutorial text.

## 🔤 General Structure

Pages are minimal: a `<head>` with a content-type meta tag and a small inline `style` (background colour, font family, text colour) and a `<body>` with one or two sentences. The tutorial files are named by number (`3600.html`) and exist once per language folder; how a number is chosen at run time is unknown ❓.

## 🧪 Examples

```html
<html><head><meta http-equiv="content-type" content="text/html; charset=ISO-8859-1"> ... </head>
<body>Oderwij bilet "Zwykły".</body></html>
```
(a Polish ticket-tearing instruction from `Tutorials/3/POL/3600.html`)

## ❓ Open Questions

| ID | Question | Priority |
| :--- | :--- | :--- |
| Q-01 | How scenarios reference the numbered pages, and which HTML subset OMSI renders. | 🟢 Low |
| Q-02 | Encoding for the 448 files without a charset declaration. | 🟢 Low |

## 📝 Provenance

- `Tutorials/3/POL/3600.html` (smallest), `Addons/Saint-Servan/Documentation/Lines/Services/2/Samedi/2-2S-800.html` (largest); folder and charset tally over all 1,317 files.

**Clean-room attestation (ADR-0002):**
- No OMSI binaries were inspected.
- No GPL source code was read.
- No OMSI assets were committed.

## 🔗 Related Documents

- [`global-cfg`](map/global-cfg.md) (`.osn` scenarios), [inventory](inventory.md)
