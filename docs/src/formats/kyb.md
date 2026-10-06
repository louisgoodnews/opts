# `.kyb` — Keyboard Key-Name Tables

| Metadata | Value |
| :--- | :--- |
| **Format Family** | Text (tab-separated table) |
| **File Extensions** | `.kyb` (2 files: `Inputs/DEU.kyb`, `Inputs/ENG.kyb`) |
| **Parser Crate** | `opts-core` (not yet planned) |
| **Parser Module** | n/a — stub |
| **Encoding** | CP1252 (DEU), UTF-8/ASCII (ENG) |
| **Status** | 🟡 Stub — written by the P-100 reconnaissance pass; observation only |
| **Last Updated** | 2026-10-04 |

> ⚠️ **Stub.** Everything below is observation from a handful of sample files in the OMSI 2 installation. Nothing was tested in OMSI. Statements marked ❓ are guesses.

---

## 📖 Overview

One file per interface language that maps a **numeric key index to a display name** (`1 → Escape`, `2 → 1`, …). It sits in `Inputs/` with `keyboard.cfg`, `keyboard_reset.cfg` and `gamectrler.cfg`; the key names are what the controls screen shows ❓. The indices follow the order of the classic PC keyboard scan codes (1 = Escape, 2–11 = the digit row) ❓.

## 🔤 General Structure

131 lines each: `<index><TAB><name>`, starting at index 0 (`<no>` in English, `<keine>` in German). Names are localised: the German file shows `ß` and `´` where the English file shows `-` and `=`.

## 🧪 Examples

```
0	<no>
1	Escape
2	1
```

## ❓ Open Questions

| ID | Question | Priority |
| :--- | :--- | :--- |
| Q-01 | ~~Do the indices equal DirectInput key codes?~~ — consistent: `keyboard.cfg` codes 25, 44, 48, 181 map to `P`, `Z`, `B`, `Num /` here (see [engine-config](text/engine-config.md)). Are the gaps keys that cannot be bound? | 🟢 Low |
| Q-02 | Format of `keyboard.cfg` and `gamectrler.cfg` (`.cfg` files in the same folder). | 🟢 Low |

## 📝 Provenance

- `Inputs/ENG.kyb` and `Inputs/DEU.kyb` (both read in full).

**Clean-room attestation (ADR-0002):**
- No OMSI binaries were inspected.
- No GPL source code was read.
- No OMSI assets were committed.

## 🔗 Related Documents

- [Inventory](inventory.md)
