# `.xsli` — Spline Project (XML)

| Metadata | Value |
| :--- | :--- |
| **Format Family** | Text (XML) |
| **File Extensions** | `.xsli` (5 files) |
| **Parser Crate** | `opts-core` (not yet planned) |
| **Parser Module** | n/a — stub |
| **Encoding** | UTF-8 |
| **Status** | 🟡 Stub — written by the P-100 reconnaissance pass; observation only |
| **Last Updated** | 2026-10-04 |

> ⚠️ **Stub.** Everything below is observation from a handful of sample files in the OMSI 2 installation. Nothing was tested in OMSI. Statements marked ❓ are guesses.

---

## 📖 Overview

`.xsli` files are **XML project files of a spline editor**: a `SplineProject` whose content mirrors a [`.sli`](text/sli.md) (height profiles, textures, texture wraps, paths) plus editor bookkeeping (`History`, `FileName`, `UnknownLines`, `DefaultLength`). They sit beside the `.sli` they describe (`texture/Zaunsockel.xsli`, `…/AS114_1219/mur.xsli`). OMSI itself is not known to read them ❓; they matter because they carry names and flags that a plain `.sli` lacks (profile names, `HasTransparency`, `Puddles`, `Moisture`).

## 🔤 General Structure

Root element `SplineProject` (XML Schema instance namespaces). Children observed in both the smallest (829 lines) and largest (7,841 lines) file:

| Element | Contents (child element names) |
| :--- | :--- |
| `IndexId` | integer |
| `HeightProfiles` / `HeightProfile` | `IndexId`, `Name`, `StartPoint`, `EndPoint`, `StartHeight`, `EndHeight` |
| `Textures` / `Texture` | `IndexId`, `Filename`, `HasTransparency`, `HasDamageTexture`, `Puddles`, `Moisture` |
| `TextureWraps` / `TextureWrap` | `IndexId`, `Texture`, `StartPoint`, `EndPoint`, `StartHeight`, `EndHeight`, `TextureStartPt`, `TextureEndPt` |
| `Pathes` | list of paths (not expanded here) |
| `FileName`, `UnknownLines`, `DefaultLength` | export file name, lines the editor could not interpret, default segment length |
| `History` | undo history (`IndexId`, `HeightProfiles`, `Textures`) |

The `Puddles` and `Moisture` fields line up with the `[puddles]` / `[moisture]` flags of [texture property files](text/texture-cfg.md) ❓.

## 🧪 Examples

```xml
<SplineProject ...>
  <IndexId>0</IndexId>
  <HeightProfiles>
    <HeightProfile><IndexId>0</IndexId><Name>Gehweg</Name><StartPoint>-0.2</StartPoint>
      <EndPoint>-0.2</EndPoint><StartHeight>0</StartHeight><EndHeight>0.4</EndHeight></HeightProfile>
```
(line breaks adjusted; names are German: *Gehweg* = sidewalk, *Mur* = wall.)

## ❓ Open Questions

| ID | Question | Priority |
| :--- | :--- | :--- |
| Q-01 | Which tool writes `.xsli`, and does OMSI ignore it? | 🟢 Low |
| Q-02 | Mapping of every `.xsli` field to `.sli` keywords (`Pathes`, `TextureWrap`). | 🟢 Low |

## 📝 Provenance

- `Sceneryobjects/HB_76_crossings_splines/texture/Zaunsockel.xsli` (smallest), `Splines/St-Servan_Splines-Additionnel/GPM_Splines/AS114_1219/mur.xsli` (largest); element census of all 5 files.

**Clean-room attestation (ADR-0002):**
- No OMSI binaries were inspected.
- No GPL source code was read.
- No OMSI assets were committed.

## 🔗 Related Documents

- [`.sli`](text/sli.md), [texture property files](text/texture-cfg.md)
