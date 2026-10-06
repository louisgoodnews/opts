# 🔄 Cross-Reference Audit Report

- **Date:** 2026-10-06
- **Pass:** Documentation Freeze Pass (Stage 4)

---

## Audit Methodology

1. Extracted all directed edges $A \to B$ in the `## 🔗 Related Documents` section of each format specification.
2. Verified bidirectional symmetry: where specification $A$ links to closely related specification $B$, verified that $B$ reciprocates by linking back to $A$.
3. Updated specifications lacking reciprocal backlinks across the major format clusters:
   - **Vehicle Ecosystem:** `bus.md` $\leftrightarrow$ (`model_cfg.md`, `paths_cfg.md`, `passengercabin_cfg.md`, `sound_cfg.md`, `hof.md`, `osc.md`, `varnamelist.md`, `const.md`, `script-sets.md`).
   - **Model & Presentation:** `model_cfg.md` $\leftrightarrow$ (`repaints.md`, `osc.md`, `varnamelist.md`, `predefined-variables.md`).
   - **Audio & Announce:** `wav.md` $\leftrightarrow$ (`sound_cfg.md`, `hof.md`).
   - **Scenery & Splines:** `sco.md` $\leftrightarrow$ (`textures.md`, `repaints.md`, `scenery-scripts.md`, `texture-cfg.md`); `sli.md` $\leftrightarrow$ (`xsli.md`, `texture-cfg.md`).
   - **World & Map Data:** `map/lists.md` $\leftrightarrow$ `text/org.md`, `text/repaints.md`; `map/tiles.md` & `map/global-cfg.md` $\leftrightarrow$ `textures.md`.

---

## Reciprocal Links Added

| Specification File | Reciprocal Links Added |
| :--- | :--- |
| `text/vehicles/bus.md` | Added links to 9 vehicle sub-configs (`model_cfg.md`, `paths_cfg.md`, `passengercabin_cfg.md`, `sound_cfg.md`, `hof.md`, `osc.md`, `varnamelist.md`, `const.md`, `script-sets.md`); removed stale `*(not yet written)*` tags for `.sco`, `.cfg`, `.org`. |
| `text/vehicles/model_cfg.md` | Added reciprocal links to `bus.md`, `repaints.md`, `osc.md`, `varnamelist.md`, `predefined-variables.md`. |
| `text/vehicles/sound_cfg.md` | Added reciprocal links to `bus.md`, `wav.md`, `sco.md`. |
| `text/vehicles/osc.md` | Added reciprocal links to `bus.md`, `sco.md`, `opl.md`, `scenery-scripts.md`. |
| `text/vehicles/varnamelist.md` | Added reciprocal links to `bus.md`, `sco.md`, `scenery-scripts.md`, `predefined-variables.md`. |
| `text/vehicles/const.md` | Added reciprocal links to `bus.md`, `sco.md`, `predefined-variables.md`. |
| `text/vehicles/hof.md` | Added reciprocal links to `bus.md`, `wav.md`. |
| `text/sco.md` | Added reciprocal links to `textures.md`, `repaints.md`, `scenery-scripts.md`, `texture-cfg.md`. |
| `text/sli.md` | Added reciprocal links to `xsli.md`, `texture-cfg.md`. |
| `map/lists.md` | Added reciprocal links to `org.md`, `repaints.md`. |
| `map/tiles.md` | Added reciprocal link to `textures.md`. |
| `map/global-cfg.md` | Added reciprocal links to `textures.md`, `ticketpacks.md`. |
| `textures.md` | Added reciprocal link to `wav.md`. |
| `wav.md` | Added reciprocal links to `sound_cfg.md`, `hof.md`, `ticketpacks.md`. |

All format specification clusters are now bidirectionally connected.
