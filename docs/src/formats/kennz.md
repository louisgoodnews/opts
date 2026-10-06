# `.kennz` — Number-Plate Pattern List

| Metadata | Value |
| :--- | :--- |
| **Format Family** | Text with binary prefix per line |
| **File Extensions** | `.kennz` (2 files: `Vehicles/VA_Motorrad_Wien_1/Honda.kennz`, `Vehicles/VA_Motorrad_Wien_2/Honda.kennz`) |
| **Parser Crate** | `opts-core` (not yet planned) |
| **Parser Module** | n/a — stub |
| **Encoding** | CP1252-like bytes |
| **Status** | 🟡 Stub — written by the P-100 reconnaissance pass; observation only |
| **Last Updated** | 2026-10-04 |

> ⚠️ **Stub.** Everything below is observation from a handful of sample files in the OMSI 2 installation. Nothing was tested in OMSI. Statements marked ❓ are guesses.

---

## 📖 Overview

`.kennz` ("Kennzeichen", licence plate) holds **13 short records** for the motorcycle packs `VA_Motorrad_Wien_1` and `_2` (identical size). Each line starts with **four non-text bytes** and continues with a number and a letter code (`213 E`, `25 DE`, `26 FR`, `54 PL`). It is probably the plate **layout/colour prefix plus the number and country/district text** the vehicle shows ❓. Only these two packs use it, and no in-file documentation exists.

## 🔤 General Structure

Line length is constant; every line ends with `CR LF`. Example bytes of the first record: `80 20 57 7B` then ASCII `   213 E  `. Other records start with `80 47 46 7D`, `80 57 55 7D`, `80 4B 4F 7D` — the second and third bytes are letters (`GF`, `WU`, `KO`), the first byte is `0x80` and the fourth is `0x7B` or `0x7D` ❓ (looks like a short code padded with the high-bit character and braces).

## 🧪 Examples

```
80 20 57 7b 20 20 20 32 31 33 20 45 20 20 0d 0a      ". W{   213 E  \r\n"
```

## ❓ Open Questions

| ID | Question | Priority |
| :--- | :--- | :--- |
| Q-01 | Meaning of the four-byte prefix (style, colour, font?). | 🟢 Low |
| Q-02 | How a vehicle references a `.kennz` (keyword in the `.ovh`). | 🟢 Low |

## 📝 Provenance

- `Vehicles/VA_Motorrad_Wien_1/Honda.kennz` (206 bytes, 13 lines); the `Wien_2` copy has the same size.

**Clean-room attestation (ADR-0002):**
- No OMSI binaries were inspected.
- No GPL source code was read.
- No OMSI assets were committed.

## 🔗 Related Documents

- [`.org`](text/org.md) (plate lists referenced by `[registration_list]`)
