# 📊 Format Specifications Status Audit Report

- **Date:** 2026-10-06
- **Pass:** Documentation Freeze Pass (Stage 2)
- **Rule:**
  - 🟢 **Specified** requires zero unresolved 🔴 High questions and verified structural/grammatical definition.
  - 🟡 **Draft** / 🟡 **Stub** applies when open questions or semantic behaviors remain under investigation.
  - 🔴 **Blocked** requires an explicit blocker reason.

---

## Specification Status Matrix

| Spec | Declared Status | High Questions | Total Open | Actual Status | Change Made |
| :--- | :--- | :---: | :---: | :--- | :--- |
| `formats/binary/o3d.md` | 🟢 Specified | 0 | 4 | 🟢 Specified | None (confirmed; high questions resolved in M-0002) |
| `formats/binary/x.md` | 🟢 Specified | 0 | 4 | 🟢 Specified | None (confirmed; high questions resolved in M-0002) |
| `formats/binary/o3d-chunks.md` | 🟡 Draft | 0 | 0 | 🟡 Draft | None (companion chunk reference) |
| `formats/text/vehicles/bus.md` | 🟢 Specified | 0 | 0 | 🟢 Specified | None (confirmed grammar and keywords complete) |
| `formats/text/vehicles/paths_cfg.md` | 🟢 Specified | 0 | 0 | 🟢 Specified | None (confirmed complete) |
| `formats/text/money.md` | 🟢 Specified | 0 | 3 | 🟢 Specified | None (grammar complete, 0 high questions) |
| `formats/text/org.md` | 🟢 Specified | 0 | 3 | 🟢 Specified | None (grammar complete, 0 high questions) |
| `formats/text/cfg.md` | None (0-byte) | 0 | 0 | 🟡 Draft | Populated 0-byte file as hub/index document |
| `formats/text/sco.md` | 🟡 Draft | 0 | 9 | 🟡 Draft | None (confirmed 9 open questions) |
| `formats/text/sli.md` | 🟡 Draft | 0 | 6 | 🟡 Draft | None (confirmed 6 open questions) |
| `formats/text/vehicles/model_cfg.md` | 🟡 Draft | 4 | 13 | 🟡 Draft | None (confirmed 4 high open questions) |
| `formats/text/vehicles/passengercabin_cfg.md` | 🟡 Draft | 1 | 2 | 🟡 Draft | None (confirmed 1 high open question) |
| `formats/text/vehicles/sound_cfg.md` | 🟡 Draft | 0 | 4 | 🟡 Draft | None (confirmed 4 open questions) |
| `formats/text/vehicles/osc.md` | 🟡 Draft | 0 | 5 | 🟡 Draft | None (confirmed 5 open questions) |
| `formats/text/vehicles/varnamelist.md` | 🟡 Draft | 0 | 4 | 🟡 Draft | None (confirmed 4 open questions) |
| `formats/text/vehicles/const.md` | 🟡 Draft | 0 | 3 | 🟡 Draft | None (confirmed 3 open questions) |
| `formats/text/vehicles/hof.md` | 🟡 Draft | 0 | 5 | 🟡 Draft | None (confirmed 5 open questions) |
| `formats/text/vehicles/predefined-variables.md` | 🟡 Draft | 1 | 5 | 🟡 Draft | None (confirmed 1 high open question) |
| `formats/text/vehicles/script-sets.md` | 🟡 Draft | 2 | 5 | 🟡 Draft | None (confirmed 2 high open questions) |
| `formats/text/vehicles/vehicle-corpus-audit.md` | 🟡 Draft | 0 | 5 | 🟡 Draft | None (audit reference document) |
| `formats/text/scenery-scripts.md` | 🟡 Draft | 0 | 4 | 🟡 Draft | None (confirmed 4 open questions) |
| `formats/text/repaints.md` | 🟡 Draft | 1 | 6 | 🟡 Draft | None (confirmed 1 high open question) |
| `formats/text/environment.md` | 🟡 Draft | 0 | 5 | 🟡 Draft | None (confirmed 5 open questions) |
| `formats/text/engine-config.md` | 🟡 Draft | 0 | 3 | 🟡 Draft | None (confirmed 3 open questions) |
| `formats/text/opl.md` | 🟡 Draft | 0 | 3 | 🟡 Draft | None (confirmed 3 open questions) |
| `formats/text/ticketpacks.md` | 🟡 Draft | 0 | 4 | 🟡 Draft | None (confirmed 4 open questions) |
| `formats/text/fonts.md` | 🟡 Draft | 0 | 4 | 🟡 Draft | None (confirmed 4 open questions) |
| `formats/text/misc-types.md` | 🟡 Draft | 0 | 8 | 🟡 Draft | None (confirmed 8 open questions) |
| `formats/text/texture-cfg.md` | 🟡 Draft | 0 | 4 | 🟡 Draft | None (confirmed 4 open questions) |
| `formats/text/sco-sli-audit.md` | 🟡 Draft | 1 | 6 | 🟡 Draft | None (audit reference document) |
| `formats/map/global-cfg.md` | 🟡 Draft | 1 | 7 | 🟡 Draft | None (confirmed 1 high open question) |
| `formats/map/lists.md` | 🟡 Draft | 0 | 7 | 🟡 Draft | None (confirmed 7 open questions) |
| `formats/map/timetables.md` | 🟡 Draft | 2 | 6 | 🟡 Draft | None (confirmed 2 high open questions) |
| `formats/map/chrono.md` | 🟡 Draft | 2 | 5 | 🟡 Draft | None (confirmed 2 high open questions) |
| `formats/map/tiles.md` | 🟡 Draft | 3 | 9 | 🟡 Draft | None (confirmed 3 high open questions) |
| `formats/textures.md` | 🟡 Draft | 3 | 10 | 🟡 Draft | None (confirmed 3 high open questions) |
| `formats/wav.md` | 🟡 Draft | 0 | 6 | 🟡 Draft | None (confirmed 6 open questions) |
| `formats/community-crosscheck.md` | 🟡 Draft | 1 | 6 | 🟡 Draft | None (cross-check analysis document) |
| `formats/gap-analysis.md` | 🟡 Draft | 0 | 4 | 🟡 Draft | None (gap tracking document) |
| `formats/inventory.md` | 🟡 Draft | 0 | 2 | 🟡 Draft | None (inventory dataset document) |
| `formats/dat.md` | 🟡 Stub | 0 | 2 | 🟡 Stub | None (inventory stub) |
| `formats/html.md` | 🟡 Stub | 0 | 2 | 🟡 Stub | None (inventory stub) |
| `formats/ipr.md` | 🟡 Stub | 0 | 1 | 🟡 Stub | None (inventory stub) |
| `formats/itx.md` | 🟡 Stub | 0 | 2 | 🟡 Stub | None (inventory stub) |
| `formats/kennz.md` | 🟡 Stub | 0 | 2 | 🟡 Stub | None (inventory stub) |
| `formats/kyb.md` | 🟡 Stub | 0 | 2 | 🟡 Stub | None (inventory stub) |
| `formats/off.md` | 🟡 Stub | 0 | 1 | 🟡 Stub | None (inventory stub) |
| `formats/sichtbar.md` | 🟡 Stub | 0 | 1 | 🟡 Stub | None (inventory stub) |
| `formats/xsli.md` | 🟡 Stub | 0 | 2 | 🟡 Stub | None (inventory stub) |
