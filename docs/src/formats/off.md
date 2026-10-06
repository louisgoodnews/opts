# `.off` — Disabled Texture-Change Lists

| Metadata | Value |
| :--- | :--- |
| **Format Family** | Text |
| **File Extensions** | `.off` (5 files) |
| **Parser Crate** | `opts-core` (not yet planned) |
| **Parser Module** | n/a — stub |
| **Encoding** | UTF-8 / CP1252 |
| **Status** | 🟡 Stub — written by the P-100 reconnaissance pass; observation only |
| **Last Updated** | 2026-10-04 |

> ⚠️ **Stub.** Everything below is observation from a handful of sample files in the OMSI 2 installation. Nothing was tested in OMSI. Statements marked ❓ are guesses.

---

## 📖 Overview

`.off` files are **`.cti` files switched off by renaming**. The in-file text (German, paraphrased) says that OMSI searches the chosen folder for all `*.cti` files and merges every `[item]` command it finds; any number of `.cti` files may exist, each with one or many items. A file ending in `.off` is therefore skipped by that search ❓ (inferred from the quoted rule, not tested). Examples: `Vehicles/MAN_NL_NG/Texture/Werbung_EN92/testanstrich.cti.off`, `Sceneryobjects/Ruede/texture/werbungen/werbungen_originale.off`.

## 🔤 General Structure

Content is exactly the [`.cti`](text/misc-types.md#cti) grammar: `[item]` followed by three lines — item name, the `[CTCTexture]` name it belongs to, and the texture file name — optionally preceded by a banner of asterisks and a German explanation of the syntax.

## 🧪 Examples

```
[item]
Testanstrich
farbschema_tex1
EN92_1_test.dds
```

## ❓ Open Questions

| ID | Question | Priority |
| :--- | :--- | :--- |
| Q-01 | Confirm that only the `.cti` extension is scanned. | 🟢 Low |

## 📝 Provenance

- `Vehicles/MAN_NL_NG/Texture/Werbung_EN92/testanstrich.cti.off` (smallest), `Sceneryobjects/Ruede/texture/werbungen/werbungen_originale.off` (largest).

**Clean-room attestation (ADR-0002):**
- No OMSI binaries were inspected.
- No GPL source code was read.
- No OMSI assets were committed.

## 🔗 Related Documents

- [`.cti` in misc types](text/misc-types.md#cti)
