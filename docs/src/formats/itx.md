# `.itx` — Internet Texture List

| Metadata | Value |
| :--- | :--- |
| **Format Family** | Text (line pairs) |
| **File Extensions** | `.itx` (10 files under `Texture/`, one in `Texture/PAD-Labs/`) |
| **Parser Crate** | `opts-core` (not yet planned) |
| **Parser Module** | n/a — stub |
| **Encoding** | UTF-8 / ASCII |
| **Status** | 🟡 Stub — written by the P-100 reconnaissance pass; observation only |
| **Last Updated** | 2026-10-04 |

> ⚠️ **Stub.** Everything below is observation from a handful of sample files in the OMSI 2 installation. Nothing was tested in OMSI. Statements marked ❓ are guesses.

---

## 📖 Overview

An `.itx` file lists **remote textures and where they go locally**. Each entry is a URL followed by a local relative path (backslash form, from the OMSI root). All 10 files contain advertising textures for specific vehicles or add-ons (`MANCitybus.itx`, `va_wien_1_texturen.itx`, …). Each `.itx` has a companion [`.ipr`](ipr.md) of the same base name. The purpose appears to be an **ad-texture download list** ❓ — no in-file documentation exists.

## 🔤 General Structure

Entries are separated by a blank line; each entry is two lines:

1. `http://…` — the source URL of an image,
2. a path such as `Vehicles\MAN_NL_NG_263\Texture\Werbung\werbung0.jpg` — the destination, relative to the installation.

Across the 10 files there are **126 URL entries** (0 in `Bremen-Nord.itx`, 8–24 in the others).

## 🧪 Examples

```
http://btsimpro.de/Downloads/OMSI/werbung0.jpg
Vehicles\MAN_NL_NG_263\Texture\Werbung\werbung0.jpg
```

## ❓ Open Questions

| ID | Question | Priority |
| :--- | :--- | :--- |
| Q-01 | Who reads `.itx` — OMSI, an add-on installer, or a community tool? | 🟡 Medium |
| Q-02 | Is a missing or unreachable URL harmless? | 🟢 Low |

## 📝 Provenance

- `Texture/MANCitybus.itx` (smallest), `Texture/va_wien_1_texturen.itx` (largest); URL counts for all 10 files. No URL was fetched.

**Clean-room attestation (ADR-0002):**
- No OMSI binaries were inspected.
- No GPL source code was read.
- No OMSI assets were committed.

## 🔗 Related Documents

- [`.ipr`](ipr.md), [`.cti`](text/misc-types.md#cti) (texture-change items that select such adverts)
