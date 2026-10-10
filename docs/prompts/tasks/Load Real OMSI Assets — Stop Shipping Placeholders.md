# 🚨 Load Real OMSI Assets — Eliminate Placeholders

The OPTS application currently renders placeholder geometry: a
yellow box as a vehicle, a plain road strip, and a single pole as
scenery. This is not a simulator. This is a scaffold.

The entire Phase 1 documentation effort — 40+ format specs, two
mutation kits, corpus audits, `.o3d` layout confirmed on 165,000+
files, coordinate frames verified in-engine — exists so that OPTS
loads **real OMSI content**. The application must load actual map
tiles, actual scenery objects, actual splines, and actual vehicles
from the user's OMSI installation.

**This task is not complete until the yellow box is gone and
replaced by an actual OMSI bus mesh loaded from an actual `.o3d`
file, driving on an actual `.map` tile from an actual map folder,
surrounded by actual `.sco` scenery objects.**

You are not to build new subsystems. You are not to redesign
anything. You are to **use the code that already exists** — the
parsers, the catalogue, the coordinate conversion — to make the
simulation load real content.

---

## 1. Configuration

### OMSI installation path
/home/louisgoodnews/nvme2n1/steamapps/common/OMSI 2

If this path does not exist, STOP and report.

### OPTS repository root
The current working directory. Confirm by checking for a `Cargo.toml`
with `[workspace]` and a `docs/src/SUMMARY.md`.

### Output targets
- `crates/opts-game/src/main.rs` (or `app.rs`) — wiring
- `crates/opts-bevy/src/` — asset loaders and scene spawners
- `crates/opts-sim/src/` — simulation state sourced from real files
- `docs/src/architecture/application-startup.md` — updated

---

## 2. What Went Wrong

The current implementation loads:

- A hardcoded yellow cuboid as the "vehicle"
- A hardcoded plane strip as the "road"
- A hardcoded cylinder as "scenery"
- Hardcoded vehicle state (600 RPM, gear D, 0 km/h)

Every one of these is a placeholder. None of them is sourced from
the OMSI installation. Every one of them can be replaced by the
real thing using the code and specs already in the repository.

This is not acceptable. The user did not fund 40+ format specs to
see a yellow box.

---

## 3. What Already Exists (Use It)

Before writing any new code, inventory what's already available.
Do not reinvent. Do not stub. **Use these.**

### Format specifications (read first)

- `docs/src/formats/binary/o3d.md` — 🟢 Specified, layout
  confirmed on 165,695 files
- `docs/src/formats/binary/o3d-chunks.md` — chunk reference
- `docs/src/formats/binary/x.md` — 🟢 Specified
- `docs/src/formats/map/tiles.md` — `.map` tile format
- `docs/src/formats/map/global-cfg.md` — `global.cfg`
- `docs/src/formats/map/lists.md` — AI lists
- `docs/src/formats/text/sco.md` — scenery objects
- `docs/src/formats/text/sli.md` — splines
- `docs/src/formats/text/vehicles/bus.md` — 🟢 Specified
- `docs/src/formats/text/vehicles/model_cfg.md`
- `docs/src/formats/text/vehicles/paths_cfg.md` — 🟢 Specified
- `docs/src/formats/text/vehicles/passengercabin_cfg.md`
- `docs/src/formats/text/vehicles/sound_cfg.md`
- `docs/src/formats/text/org.md` — 🟢 Specified
- `docs/src/architecture/coordinates.md` — OMSI ↔ Bevy conversion

### Code (inventory before implementing)

Report the current state of these crates. For each, list its public
API and any `todo!()`, `unimplemented!()`, or placeholder returns.

| Crate | Has parser for... | Public fn signature | Placeholders? |
| :--- | :--- | :--- | :---: |
| `opts-core::formats::o3d` | `.o3d` binary | `parse(&[u8])` | ? |
| `opts-core::formats::x` | `.x` | `parse(&[u8])` | ? |
| `opts-core::formats::sco` | `.sco` text | ? | ? |
| `opts-core::formats::sli` | `.sli` text | ? | ? |
| `opts-core::formats::map::tile` | `.map` | ? | ? |
| `opts-core::formats::map::global` | `global.cfg` | ? | ? |
| `opts-core::formats::vehicles::bus` | `.bus` | `parse(&str)` | ? |
| `opts-discovery` | filesystem scan | `scan(...)` | ? |
| `opts-bevy` | asset loaders | `add_asset_loader(...)` | ? |

**The agent must produce this table before writing any code.** If
a parser doesn't exist, that is a blocker to report, not an
invitation to stub.

---

## 4. The Real Behaviour Required

### 4.1 Load an Actual Map

The application must load a real map from `OMSI 2/maps/`. The
current `--map-path` argument exists — use it to point at a real
map folder.

Loading a map means:

1. Parse `<map>/global.cfg` to identify the map's metadata.
2. Locate the map's tiles (`tile_*.map`).
3. Load each visible tile:
   - Parse the `.map` file to extract `[object]` placements.
   - Parse the `.map` file to extract `[spline]` placements.
   - Load the tile's `.terrain` height grid.
4. For each `[spline]`, load the referenced `.sli` and extrude it.
5. For each `[object]`, load the referenced `.sco` and its `.o3d`.
6. Position everything using the OMSI → Bevy coordinate conversion
   from `docs/src/architecture/coordinates.md`.

For the first pass, load **only the tile the camera is on** and
its 8 neighbours. Do not attempt streaming yet.

### 4.2 Load an Actual Scenery Object

Instead of a hardcoded yellow cylinder:

1. Read `[object]` records from the tile.
2. Look up the referenced `.sco` file in the catalogue.
3. Parse the `.sco` per `docs/src/formats/text/sco.md`.
4. Load the referenced `.o3d` mesh via the `.o3d` parser.
5. Load the referenced `.dds`/`.tga`/`.bmp` textures.
6. Spawn a Bevy entity with the mesh, material, and transform.

### 4.3 Load an Actual Spline

Instead of a hardcoded road strip:

1. Read `[spline]` records from the tile.
2. Look up the referenced `.sli` file in the catalogue.
3. Parse the `.sli` per `docs/src/formats/text/sli.md`.
4. Extrude the spline profile along the segment path.
5. Generate a mesh from the extrusion.
6. Load the referenced textures.
7. Spawn a Bevy entity with the mesh, material, and transform.

### 4.4 Load an Actual Vehicle

Instead of a hardcoded yellow cuboid:

1. Read the vehicle path from a CLI flag (`--vehicle-path`) or
   from the map's `[aigroup]`/`[ailist]` file.
2. Parse the `.bus` file per `docs/src/formats/text/vehicles/bus.md`.
3. Locate the model config (`.cfg`) referenced by `[model]`.
4. Parse the model config per `model_cfg.md`.
5. For each `[mesh]` directive:
   - Load the referenced `.o3d`.
   - Apply the associated materials.
   - Apply the transforms and animations.
6. Assemble the resulting Bevy entities under one vehicle root.
7. Position the vehicle at the map's starting location.

**The player camera follows the vehicle.** Not a hardcoded box.

### 4.5 Replace Hardcoded HUD State

The HUD currently shows `600 RPM`, `0 km/h`, `D`, `RELEASED`.
These numbers must come from `opts-sim`'s real state, which must be
initialized from the vehicle's actual configuration.

If `opts-sim` is not yet wired to receive real vehicle data, that
wiring happens now. Do not display fake numbers.

---

## 5. Guardrails

### ❌ Forbidden

- Do not disassemble, decompile, or inspect `OMSI.exe` or any DLL.
- Do not commit OMSI assets to the repository. Load at runtime.
- Do not hardcode any asset path. All paths come from the map,
  the catalogue, or CLI arguments.
- Do not hardcode any geometry. All geometry comes from `.o3d` files.
- Do not hardcode any position. All positions come from `.map` files
  and coordinate conversion.
- Do not display fake state in the HUD.
- Do not leave placeholder code paths active. If a parser isn't
  ready, log an `ERROR` and skip that asset — do not silently
  substitute a placeholder.
- Do not create new stubs. Existing stubs are a failure to fix,
  not a pattern to extend.
- Do not write new specs. Use the ones that exist.
- Do not modify the format specs unless a parser reveals a factual
  error in them.

### ✅ Permitted

- Read every file in the repository.
- Read every file in the OMSI installation (except executables).
- Load OMSI assets at runtime.
- Add `opts-game` and `opts-bevy` dependencies.
- Add asset loaders to `opts-bevy`.
- Add tile streaming logic (bounded, single-tile first pass).
- Add debugging visualization (bounding boxes, wireframes) if it
  helps verify correct loading.
- Log extensively via `opts-logging`.

---

## 6. Part A — Diagnostic (Do This First)

Before writing any code, produce a diagnostic report.

### Stage A1 — Trace the Placeholder

Open the current `crates/opts-game/src/` code. Identify **every**
line that spawns placeholder geometry, sets fake state, or loads a
default asset. Produce a table:

| File:line | What it does | What it should do instead |
| :--- | :--- | :--- |

This is the list of things to replace.

### Stage A2 — Inventory Existing Parsers

Produce the parser inventory table from Section 3. For each parser:

- Does it exist?
- Does it compile?
- Does it have tests?
- Does it have a doctest example?
- Is it marked `todo!()` / `unimplemented!()`?

If a parser is missing, that is a blocker. Report it. Do not stub.

### Stage A3 — Inventory Existing Catalogue

If `opts-discovery` exists, report:

- Is the catalogue schema implemented?
- Can it be scanned against the OMSI installation?
- Does it expose a lookup by relative path?
- Does `opts-game` currently use it?

If the catalogue is not implemented, that is a blocker. Report it.

### Stage A4 — Test the O3D Parser in Isolation

Pick 3 `.o3d` files from the OMSI installation (a small one, a
medium one, a large one). Run them through the parser. Report:

| File | Size | Vertices parsed | Indices parsed | Materials parsed | Errors |
| :--- | ---: | ---: | ---: | ---: | :--- |

If the parser fails on any of them, that is a blocker. Report it.

### Stage A5 — Checkpoint

Report the diagnostic.

**Wait for approval before Phase B.** If any parser is missing or
broken, the user must decide whether to fix it first or to proceed
with degraded loading.

---

## 7. Part B — Implementation

Only proceed after Phase A is approved.

### Stage B1 — Asset Loaders

For each asset type, wire a Bevy `AssetLoader`:

- **`.o3d`** → `Mesh` + `Material` (custom `O3dAsset`)
- **`.sli`** → extrusion pipeline producing `Mesh`
- **`.sco`** → metadata + reference to `.o3d`
- **`.map`** → tile metadata (not a Bevy asset — a resource)
- **`.bus`** → vehicle definition (not a Bevy asset — a resource)

Each loader is a plugin under `opts-bevy`.

### Stage B2 — Catalogue Integration

If `opts-discovery` is implemented:

1. On startup, scan the OMSI installation.
2. Expose a `CatalogueResource` to the app.
3. Provide a lookup function: `find_file(rel_path) -> Option<PathBuf>`.

If not implemented, fall back to direct filesystem paths derived
from `--map-path`. Do not skip the catalogue silently — log a
`WARN` that `opts-discovery` is unavailable.

### Stage B3 — Map Loading

Implement the map loading pipeline:

1. Parse `global.cfg`.
2. Identify the starting tile.
3. Load the 3×3 tile neighbourhood.
4. For each tile:
   - Parse `[object]` records.
   - Parse `[spline]` records.
   - Parse the terrain grid.
5. Spawn:
   - Terrain as a `Mesh`.
   - Splines as extruded `Mesh` entities.
   - Scenery objects as `Mesh` + `Material` entities.
6. Apply the OMSI → Bevy coordinate conversion.

### Stage B4 — Vehicle Loading

Implement vehicle loading:

1. Parse the `.bus` file (from `--vehicle-path` or map AI list).
2. Parse the model `.cfg`.
3. For each `[mesh]` in the config:
   - Load the `.o3d`.
   - Apply materials from `[matl]` and `[matl_*]`.
   - Apply transforms from `[newanim]` (static transforms only for
     v1; animated transforms are a follow-up).
4. Assemble under a vehicle root entity.
5. Position at the map's spawn point.

### Stage B5 — Simulation Binding

Replace hardcoded HUD values with real state from `opts-sim`:

1. `opts-sim` reads vehicle mass, wheel radius, gear ratios from
   the `.bus` file.
2. `opts-sim` computes engine RPM from vehicle speed and gear.
3. HUD reads from `opts-sim`, not from constants.

If `opts-sim` cannot yet source this data, the HUD must show
`--` or `N/A` rather than fake numbers.

### Stage B6 — Diagnostic Overlay

Add a debug overlay (toggleable) that shows:

- Current map name
- Current tile coordinates
- Number of tiles loaded
- Number of scenery objects loaded
- Number of splines loaded
- Vehicle mesh name
- Total vertices in scene

This makes it obvious whether real content is loading.

### Stage B7 — Checkpoint

Report the implementation. Include screenshots.

**Wait for approval before Phase C.**

---

## 8. Part C — Verification

### Stage C1 — Run Against a Real Map

Launch the app with a real map:

```bash
cargo run -p opts-game -- --map-path "/home/louisgoodnews/nvme2n1/steamapps/common/OMSI 2/maps/Grundorf"
```

(Or whichever map the user specifies.)

Verify:
- [ ] The terrain is not a flat plane
- [ ] Roads are extruded from real `.sli` files, not planes
- [ ] Scenery objects are real `.o3d` meshes
- [ ] Textures are visible (not untextured grey)
- [ ] The vehicle is a real bus mesh, not a box
- [ ] The coordinate conversion is correct (roads align with
      terrain, objects sit on the ground)

### Stage C2 — Log Audit

Run the app with `--log-level debug`. Confirm the log shows:

- Map `global.cfg` parsed
- N tiles discovered
- Each tile: N objects, M splines
- Each `.sco` loaded from a real path
- Each `.o3d` loaded with vertex count
- Vehicle `.bus` parsed
- Vehicle model config parsed
- N vehicle meshes loaded

If any of these are missing, the pipeline is not loading real
content. Fix and re-verify.

### Stage C3 — The Yellow Box Test

**The yellow box must be gone.**

If any placeholder geometry remains visible, the task is not
complete. There are no exceptions.

### Stage C4 — Final Report

Produce `docs/src/meta/real-asset-loading-YYYY-MM-DD.md`:

- Which map was loaded
- Number of tiles
- Number of scenery objects
- Number of splines
- Vehicle mesh name and vertex count
- Screenshots
- Any parsers that failed
- Any assets that couldn't be loaded (with reason)
- Any remaining placeholders (should be zero)

---

## 9. Reporting Cadence

1. Part A (diagnostic) — approval before Part B
2. Part B (implementation) — approval before Part C
3. Part C (verification) — end of task

Format:

```
## Real Asset Loading — Checkpoint N

**Phase:** <name>
**Real assets loaded:** <counts>
**Placeholders remaining:** <count>
**Blockers:** <if any>

Ready to proceed? (yes / adjust)
```

---

## 10. Exit Criteria

- ✅ No hardcoded geometry in the render path.
- ✅ No hardcoded vehicle state in the HUD.
- ✅ A real map's `global.cfg` is parsed at startup.
- ✅ Real `.map` tiles are parsed.
- ✅ Real `.sco` scenery objects are loaded and spawned.
- ✅ Real `.sli` splines are extruded and rendered.
- ✅ A real `.bus` vehicle is parsed and rendered.
- ✅ Real `.o3d` meshes and `.dds`/`.tga`/`.bmp` textures are loaded.
- ✅ The OMSI → Bevy coordinate conversion is applied.
- ✅ The log shows real file paths for every loaded asset.
- ✅ The yellow box is gone.
- ✅ `cargo clippy -- -D warnings` passes.
- ✅ `cargo test --workspace` passes.

When finished, respond with the report from Stage C4.

Then respond with the exact string:
"REAL ASSETS LOADED — ready for user review"