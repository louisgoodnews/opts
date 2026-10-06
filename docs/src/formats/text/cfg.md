# `.cfg` — Configuration Files

| Metadata | Value |
| :--- | :--- |
| **Format Family** | Text (keyword-based) |
| **File Extensions** | `.cfg` |
| **Parser Crate** | `opts-core` |
| **Parser Module** | Multiple (subsystem-specific) |
| **Encoding** | Windows-1252 / ASCII text, `CRLF` |
| **Status** | 🟡 Draft — Hub / index page for `.cfg` formats |
| **Last Updated** | 2026-10-06 |

---

## 📖 Overview

The `.cfg` extension in OMSI 2 is a generic configuration extension used across several distinct subsystems. Rather than a single monolithic format, each `.cfg` sub-type follows its own grammar and keywords:

1. **Vehicle Configurations**:
   - [Model Configuration (`model.cfg`)](vehicles/model_cfg.md) — 3D mesh placement, animations, materials, and text textures.
   - [Passenger Paths (`paths.cfg`)](vehicles/paths_cfg.md) — Passenger path graph, navigation nodes, and doors.
   - [Passenger Cabin (`passengercabin.cfg`)](vehicles/passengercabin_cfg.md) — Seats, standing positions, driver seat, and ticket validation equipment.
   - [Sound Configuration (`sound.cfg`)](vehicles/sound_cfg.md) — Sound effects, loops, triggers, and curve mappings.
2. **Map & Environment Configurations**:
   - [`global.cfg`](../map/global-cfg.md) — Map manifest, tile list, and entry points.
   - [Map Lists & Calendars](../map/lists.md) (`ailists.cfg`, `unsched_traffic.cfg`).
   - [Timetable Station Links](../map/timetables.md) (`StnLinks.cfg`).
   - [Texture Property Files (`*.bmp.cfg`, `*.dds.cfg`)](texture-cfg.md) — Surface friction, puddles, moisture.
   - [Engine Environment (`envir.cfg`)](engine-config.md) — Environmental parameters.

---

## ❓ Open Questions

None specific to the hub index. Specific questions are tracked in the respective sub-configuration documents linked above.

---

## 📝 Provenance

- **Clean-room confirmation:** No OMSI binaries were inspected. No GPL source code was read. No OMSI assets were committed.

---

## 🔗 Related Documents

- [`.bus` — Vehicle Definitions](vehicles/bus.md)
- [`model.cfg`](vehicles/model_cfg.md)
- [Passenger Paths (`paths.cfg`)](vehicles/paths_cfg.md)
- [Passenger Cabin](vehicles/passengercabin_cfg.md)
- [Sound Configuration](vehicles/sound_cfg.md)
- [`global.cfg`](../map/global-cfg.md)
- [Texture Properties](texture-cfg.md)
