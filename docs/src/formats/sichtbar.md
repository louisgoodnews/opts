# `.sichtbar` / `.unsichtbar` — Visible/Invisible Spline Variants

| Metadata | Value |
| :--- | :--- |
| **Format Family** | Text (`.sli` content) |
| **File Extensions** | `.sichtbar`, `.unsichtbar` (3 + 3 files) |
| **Parser Crate** | `opts-core` (not yet planned) |
| **Parser Module** | n/a — stub |
| **Encoding** | UTF-8 |
| **Status** | 🟡 Stub — written by the P-100 reconnaissance pass; observation only |
| **Last Updated** | 2026-10-04 |

> ⚠️ **Stub.** Everything below is observation from a handful of sample files in the OMSI 2 installation. Nothing was tested in OMSI. Statements marked ❓ are guesses.

---

## 📖 Overview

Three pairs in `Sceneryobjects/PAD-Labs/Streets/` (`Colide_K`, `Human_Duo`, `Human_Solo`): `<name>.sli.sichtbar` ("visible") and `<name>.sli.unsichtbar` ("invisible") next to a plain `<name>.sli`. They are full [`.sli`](text/sli.md) files starting with a creator-credit line in `°°…°°` marks. The two variants of a pair **differ in exactly one line**: the value of `[matl_alpha]` is `0` in the visible file and `1` in the invisible one, with the same texture `Collide_Alpha.dds`. The pair is a ready-made switch between a **visible and an invisible collision/helper spline**; the user presumably copies the wanted one over the `.sli` ❓.

## 🔤 General Structure

Standard `.sli` keywords: `[heightprofile]`, `[texture]`, `[matl]`, `[matl_alpha]`, `[profile]`, `[profilepnt]`, `[path]`. Nothing else is special about the files.

**Evidence for alpha modes ✅:** with `Collide_Alpha.dds` (an alpha texture), mode `0` is the *visible* variant and mode `1` the *invisible* one. This suggests mode `0` ignores the texture's alpha channel and mode `1` honours it (transparent texels drop out) ❓ — see [`community-crosscheck`](community-crosscheck.md) Q-CC02.

## 🧪 Examples

```
[texture]
Collide_Alpha.dds
[matl]
Collide_Alpha.dds
0
[matl_alpha]
0          <- 1 in the .unsichtbar file
```

## ❓ Open Questions

| ID | Question | Priority |
| :--- | :--- | :--- |
| Q-01 | Does OMSI treat `[matl_alpha] 0` as "ignore alpha" and `1` as "alpha test"? (Test in the mutation kit.) | 🟡 Medium |

## 📝 Provenance

- `Sceneryobjects/PAD-Labs/Streets/Colide_K.sli.sichtbar` and `.unsichtbar` (line-by-line comparison: one differing line); `Human_Duo` and `Human_Solo` pairs listed.

**Clean-room attestation (ADR-0002):**
- No OMSI binaries were inspected.
- No GPL source code was read.
- No OMSI assets were committed.

## 🔗 Related Documents

- [`.sli`](text/sli.md), [`[matl_alpha]` in `.sco`](text/sco.md), [textures](textures.md)
