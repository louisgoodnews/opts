# 🗄️ Path Catalogue Schema

| Metadata | Value |
| :--- | :--- |
| **Status** | 🟢 Implemented (`opts-catalogue`, schema v1) |
| **Schema Version** | 1 (proposed) |
| **Storage** | SQLite (rusqlite, WAL mode) |
| **Last Updated** | 2026-10-09 |

---

## 📖 Overview

The path catalogue caches a discovered inventory of an OMSI 2 installation so that the OPTS application can query "what's installed?" without recursively traversing 600,000+ files on every launch. It provides an efficient, relational view of vehicles, maps, scenery objects, splines, and shared asset pools, supporting fast selection menus, dependency resolution, and incremental staleness invalidation.

---

## 🔍 Investigation Findings

### Top-Level Structure (Stage 1 Survey)

Conducted on installation root `/home/louisgoodnews/nvme2n1/steamapps/common/OMSI 2`:
- **Total Direct Children:** 77 entries (35 directories, 42 files, 0 symlinks).
- **Total Files in Scope:** 622,305 files (622,263 in subdirectories + 42 root files).
- **Total Disk Footprint:** 214.5 GiB (230,335,011,102 bytes).
- **Total Directories:** 23,318 directories across the tree.

| Directory | Immediate Children | Recursive Files | Recursive Dirs | Total Size | Primary Content |
| :--- | ---: | ---: | ---: | ---: | :--- |
| `Vehicles` | 516 | 246,465 | 9,466 | 133.9 GiB (143.8 GB) | Drivable and AI buses, trams, ships, cars, textures, sounds |
| `Sceneryobjects` | 156 | 221,313 | 9,639 | 52.7 GiB (56.6 GB) | Static & animated objects, buildings, street furniture, crossing meshes |
| `maps` | 44 | 102,882 | 1,167 | 11.8 GiB (12.7 GB) | World tiles, elevation rasters, timetable graphs, AI list configs |
| `Splines` | 82 | 30,059 | 1,856 | 4.5 GiB (4.8 GB) | Extruded road and rail profiles, terrain texture bindings |
| `TicketPacks` | 40 | 12,465 | 392 | 4.0 GiB (4.3 GB) | Passenger ticketing profiles, currency sounds, fare textures |
| `Addons` | 65 | 2,129 | 398 | 5.1 GiB (5.5 GB) | Addon documentation, PDF timetables, repaint templates |
| `Fonts` | 2,072 | 2,095 | 2 | 776.4 MiB (814 MB) | Bitmap and vector matrix fonts (`.oft`, `.bmp`) — flat folder |
| `Tutorials` | 45 | 1,466 | 20 | 27.7 MiB (29 MB) | Tutorial scenario HTML guides and mission markers |
| `Texture` | 106 | 1,038 | 56 | 391.5 MiB (411 MB) | Shared environmental sky, road, and vegetation bitmaps |
| `Humans` | 12 | 463 | 74 | 217.7 MiB (228 MB) | Passenger & driver skins, animations, `.hum` manifests |
| `OmniNavigation` | 21 | 423 | 96 | 227.7 MiB (239 MB) | Third-party navigation plugin files and assets |
| `Money` | 24 | 334 | 24 | 13.9 MiB (14.6 MB) | Physical cash payment tokens and coin specifications |
| `Sounds` | 5 | 272 | 9 | 42.3 MiB (44.3 MB) | Shared environmental sound effects (rain, wind, thunder) |
| `SDK` | 1 | 202 | 7 | 565.2 MiB (593 MB) | Development tools, exporter plugins, sample 3D source files |
| `Trains` | 186 | 186 | 0 | 43.9 KiB (45 KB) | Train composition definitions (`.zug`) — flat folder |
| `Situations` | 91 | 91 | 0 | 11.8 MiB (12.4 MB) | Saved game situations and scenario state files (`.osn`) |
| `option_presets` | 75 | 75 | 0 | 115.0 KiB (118 KB) | Graphical and simulator performance presets (`.oop`) |
| `Weather` | 56 | 56 | 0 | 15.4 KiB (15 KB) | Weather profile definition files |
| `Languages` | 42 | 42 | 0 | 376.8 KiB (386 KB) | Localization dictionary tables (`.olf`) |
| `OMSI-Tools` | 12 | 41 | 11 | 115.5 MiB (121 MB) | Utility executables and conversion tools |
| `Scripts` | 1 | 32 | 1 | 36.2 KiB (37 KB) | Shared base scripts (`.osc`) |
| `template` | 2 | 26 | 5 | 1.3 MiB (1.4 MB) | Template assets |
| `docs` | 20 | 20 | 0 | 18.5 MiB (19.4 MB) | PDF manuals and community guides |
| `helper` | 19 | 19 | 0 | 634.5 KiB (650 KB) | Helper mesh objects |
| `RegAddons` | 14 | 14 | 0 | 1.4 KiB (1.5 KB) | Registry metadata markers |
| `Repaints` | 5 | 14 | 1 | 84.8 MiB (88.9 MB) | Global vehicle repaint packs |
| `DEMImport` | 1 | 12 | 3 | 5.9 MiB (6.1 MB) | Digital Elevation Model import utility |
| `program` | 10 | 10 | 0 | 5.8 KiB (5.9 KB) | Core variable definitions and system scripts |
| `GUI` | 6 | 6 | 0 | 4.6 MiB (4.8 MB) | Menu skin textures |
| `Inputs` | 5 | 5 | 0 | 12.6 KiB (13 KB) | Keyboard and controller mapping definitions |
| `Drivers` | 2 | 2 | 0 | 56.9 KiB (58 KB) | Driver roster files |
| `_Straßenbahn` | 2 | 2 | 0 | 16.5 MiB (17.3 MB) | Tram addon staging folder |
| `plugins` | 2 | 2 | 0 | 841.5 KiB (862 KB) | DLL plugin wrappers |
| `Temp_City_RegionBus_S200-series` | 2 | 2 | 0 | 26.0 MiB (27.2 MB) | Addon staging / scratch folder |
| `Screenshots` | 0 | 0 | 0 | 0.0 B (0 B) | Empty user screenshot directory |

---

### Category Mode Classification (Stage 2 Amendment)

| Directory | Mode | Recursive Files | Rationale |
| :--- | :---: | ---: | :--- |
| `Vehicles` | **Folder-entity** | 246,465 | Nested entity folders at depth 1–2 (`Vehicles/<Name>/` or `Vehicles/<Category>/<Name>/`); marked by `*.bus` and `*.ovh`. |
| `maps` | **Folder-entity** | 102,882 | Discrete map project folders (`maps/<MapName>/`); marked by `global.cfg`. |
| `TicketPacks` | **Folder-entity** | 12,465 | Discrete fare pack folders (`TicketPacks/<PackName>/`); marked by `*.otp`. |
| `Money` | **Folder-entity** | 334 | Discrete currency pack folders (`Money/<Currency>/`); marked by denomination `*.cfg`. |
| `Humans` | **Folder-entity** | 463 | Character model folders (`Humans/<Pack>/`); marked by `*.hum`. |
| `Sceneryobjects` | **File-entity** | 221,313 | Author/pack folders holding multiple independent objects; each object marked by its `.sco` file. |
| `Splines` | **File-entity** | 30,059 | Author folders holding multiple road/rail profiles; each profile marked by its `.sli` file. |
| `Fonts` | **File-entity** | 2,095 | Single-level font pool (`.oft` and associated bitmaps). |
| `Trains` | **File-entity** | 186 | Single-level pool of train composition files (`.zug`). |
| `Situations` | **File-entity** | 91 | Single-level pool of saved simulation states (`.osn`, `.owt`). |
| `Weather` | **File-entity** | 56 | Single-level pool of weather profile files (`.cfg`, `.txt`, `.dsc`). |
| `option_presets` | **File-entity** | 75 | Single-level pool of performance preset files (`.oop`). |
| `Languages` | **File-entity** | 42 | Single-level pool of localization dictionary tables (`.olf`). |
| `Inputs` | **File-entity** | 5 | Single-level pool of controller mapping definitions (`.cfg`, `.kyb`). |
| `GUI` | **File-entity** | 6 | Single-level pool of interface skin textures (`.bmp`). |
| `Drivers` | **File-entity** | 2 | Single-level pool of driver profiles (`.odr`, `.bmp`). |
| `Texture` | **Pool** | 1,038 | Shared environmental asset pool (seasonal ground, sky, asphalt). |
| `Sounds` | **Pool** | 272 | Shared audio asset pool (`AI_Cars`, `Passengers`). |
| `helper` | **Pool** | 19 | Editor visualization gizmos and skybox helper meshes (`.o3d`, `.x`, `.sli`). |
| `program` | **Pool** | 10 | Core system script variable lists referenced by the engine. |
| `Scripts` | **Pool** | 32 | Global shared AI vehicle scripts. |
| `Addons` | **Excluded** | 2,129 | Third-party PDF documentation, timetables, and templates. |
| `Tutorials` | **Excluded** | 1,466 | HTML scenario walk-throughs and guides. |
| `SDK` | **Excluded** | 202 | Exporter plugins, 3D source files, and development documentation. |
| `OmniNavigation` | **Excluded** | 423 | Third-party navigation plugin runtime assets. |
| `OMSI-Tools` | **Excluded** | 41 | External utilities, converters, and batch tools. |
| `docs` | **Excluded** | 20 | User guides and PDF documentation. |
| `template` | **Excluded** | 26 | Shipped map editor starter templates (`map/global.cfg`, `NewMap/global.cfg`). |
| `RegAddons` | **Excluded** | 14 | Installer registry `.ini` markers. |
| `Repaints` | **Excluded** | 14 | Work-in-progress Paint.NET `.pdn` source files. |
| `DEMImport` | **Excluded** | 12 | Digital Elevation Model import utility for map editing. |
| `plugins` | **Excluded** | 2 | Native DLL plugin wrappers (`.dll`, `.opl`). |
| `_Straßenbahn` | **Excluded** | 2 | Mod backup containing only executable copies (`Omsi_current.exe`, `Omsi_older.exe`). |
| `Temp_City_RegionBus_S200-series` | **Excluded** | 2 | Modder scratch folder containing only Paint.NET `.pdn` files. |
| `Screenshots` | **Excluded** | 0 | Empty user screenshot directory. |

---

## 🔍 Stage 2 — Deep-Dive Findings

### Investigation A — `.ovh` (Other Vehicle Header)

#### 1. Specification Reference
From [`docs/src/formats/text/misc-types.md`](../formats/text/misc-types.md#ovh):
> "An `.ovh` ('other vehicle') is used for AI traffic, trailers, parked vehicles, trams and rail vehicles. It is a **`.bus`-style** vehicle definition: all of its keywords are the `.bus` vocabulary... Keywords such as `[model]`, `[sound]`, `[paths]`, `[passengercabin]`, `[varnamelist]`, `[script]`, `[constfile]` point to files documented in `model_cfg`, `sound_cfg`, `paths_cfg`, `passengercabin_cfg`, `varnamelist`, `osc`, `const`... Per-keyword semantics follow `bus.md`."

#### 2. Sampling Across Diverse Vehicle Types
Inspected 5 representative `.ovh` files across distinct vehicular categories:

| Relative Path | Vehicle Category | Key Header Content (Lines 1–10) | Keyword Syntax vs `.bus` |
| :--- | :--- | :--- | :--- |
| `Vehicles/A3/A3L_67_K.ovh` | Subway Train | `[friendlyname]` U-Bahn / A3 64/66 K-Wagen / BVG, `[type]` 2, `[number]` | Exact match (identical keyword syntax) |
| `Vehicles/B727/b727.ovh` | Aircraft / Scenery | `[friendlyname]` Boeing / B727 / All white, `[type]` 3, `[registration_free]` | Exact match |
| `Vehicles/Citr_BX/BX.ovh` | AI Passenger Car | `[registration_free]`, `[friendlyname]` Citroen / BX / Silber-Metallic | Exact match |
| `Vehicles/BR132/br132.ovh` | Train Locomotive | `[friendlyname]` Lokomotivfabrik... / BR 132 / DR, `[type]` 2, `[registration_free]` | Exact match |
| `Vehicles/Gueterwagen/Eaos_empty.ovh` | Freight Wagon | `[type]` 2, `[registration_free]`, `[model]` | Exact match |

#### 3. Side-by-Side Analysis (`.bus` and `.ovh` in Same Folder)
- **Census across `Vehicles/`:** Out of 550 folders with vehicle markers, exactly **24 folders** contain both `.bus` and `.ovh` files.
- **Twin Pairs:** In all 24 folders (e.g., `Vehicles/HH109_KIPKW1`–`KIPKW7`, `Vehicles/Gladbeck_KIPKW1`–`KIPKW7`, `Vehicles/Luzern_*`), every `.bus` file has an identically named `.ovh` twin (`ai_hh109kipkw1.bus` ↔ `ai_hh109kipkw1.ovh`, `Luzern_Truck.bus` ↔ `Luzern_Truck.ovh`).
- **Diff Comparison:**
  - `Vehicles/Luzern_Truck/`: The `.bus` and `.ovh` files are **100% byte-for-byte identical**.
  - `Vehicles/HH109_KIPKW1/`: The `.ovh` strips the horn collision script and falls back from Hamburg-specific sound config (`HH2_sound.cfg`) to default `sound.cfg`.
- **Purpose:** These twins serve as compatibility aliases so that maps referencing `.bus` or `.ovh` in their `ailists.cfg` or train consists resolve seamlessly.

#### 4. Dedicated `.ovh`-Only Folders
- **Census:** **393 folders** contain **only `.ovh`** files (compared to 133 folders containing only `.bus` files).
- **Domain:** Dedicated `.ovh` folders represent non-player vehicles: AI passenger traffic, rail/train consists (`BR132`, `DGBgqe`), subway trainsets (`A3`), trams (`VA_TypU_Wien_1`), airplanes (`B727`), and freight cars (`Gueterwagen`).
- **Structure:** These folders share the exact component hierarchy as player buses (`model/`, `sound/`, `script/`, `texture/`).

#### 5. Conclusion & Schema Implications
- **Primary Entity Marker:** `.ovh` is not a secondary companion; it is a **co-equal primary marker file** for vehicles. Over 71% of vehicle folders rely entirely on `.ovh`.
- **Playable vs AI Flag (`is_playable` boolean switch):** In OMSI, determining whether a vehicle is user-drivable requires multi-signal evaluation beyond the file extension alone (detailed in [`playability-rules.md`](playability-rules.md)):
  - `.ovh` files are strictly AI/consist vehicles that never appear in the player vehicle selection UI (`is_playable = 0`).
  - `.bus` trailing articulated units (having `[couple_front]` or trailer suffixes without a cockpit) are flagged `is_playable = 0`.
  - Lead tractor units and solo `.bus` files possessing a `[driverspot]` and player viewpoint are flagged `is_playable = 1`.
  This allows the player UI to query `WHERE is_playable = 1` via partial index, providing instant menu population without scanning file bodies or parsing script configs at runtime.
- **Resolution Ergonomics:** Because map AI lists and train consists (`.zug`) reference vehicles by exact file path (e.g., `Vehicles\Citr_BX\BX.ovh`), the catalogue must index every individual marker file and resolve it directly to its enclosing entity folder.

---

### Investigation B — Maps with Multiple `global.cfg`

#### 1. Identification of Map Folders
Across the entire OMSI installation, 48 files are named `global.cfg`:
- **41 Map Folders** have exactly one `global.cfg` directly at `maps/<MapName>/global.cfg`.
- **1 Map Folder** (`maps/Ahlheim 5/`) contains 5 files named `global.cfg`:
  1. `maps/Ahlheim 5/global.cfg` (primary active map definition)
  2. `maps/Ahlheim 5/#Editor-Fahrgastzahlen-Verkehrsaufkommen/Globaldateien/Fr/global.cfg`
  3. `maps/Ahlheim 5/#Editor-Fahrgastzahlen-Verkehrsaufkommen/Globaldateien/MoDo/global.cfg`
  4. `maps/Ahlheim 5/#Editor-Fahrgastzahlen-Verkehrsaufkommen/Globaldateien/Sa/global.cfg`
  5. `maps/Ahlheim 5/#Editor-Fahrgastzahlen-Verkehrsaufkommen/Globaldateien/So/global.cfg`
- **3 Map Folders** in Bremen (`HB_2017_Bremen-Nord_FIKTIV`, `HB_2026_Bremen-Nord`, `HB_76_Bremen-Nord`) contain variant global configs inside `Fahrgastaufkommen/` (`globalMoFR.cfg`, `globalSa.cfg`, `globalSo.cfg`, `globalFeiertag.cfg`).
- **1 Map Folder** (`TH_Wald`) contains language documentation backups (`global_DEU.cfg`, `global_ENG.cfg`).
- **2 Excluded Template Folders** (`template/map/global.cfg`, `template/NewMap/global.cfg`).
- **1 Incomplete Map Folder** (`maps/Ahlheim_Laurenzbach Updated/`) lacks `global.cfg` entirely (contains only an orphan `ailists.cfg`).

#### 2. Variant Analysis: Chrono vs Manual Swapping
- **Chrono System:** Chrono events (455 events across 18 maps documented in [`docs/src/formats/map/chrono.md`](../formats/map/chrono.md)) **never contain `global.cfg`**. Chrono overlays are defined by `Chrono.cfg`, TTData, and delta tiles (`tile_X_Y.map`).
- **Manual Batch Swapping:** In `HB_*_Bremen-Nord`, the `Fahrgastaufkommen/` directory contains companion batch scripts (`BremenMoFR.cmd`, `BremenSa.cmd`, `BremenSo.cmd`) executing:
  ```cmd
  xcopy "globalMoFR.cfg" "..\global.cfg" /Y
  ```
  These are manual, player-switched day-of-week passenger density presets, not distinct game maps or runtime engine variants.
- **Author Backups:** In `Ahlheim 5`, the `#Editor-...` directory contains author reference backups for Friday, Monday–Thursday, Saturday, and Sunday timetable passenger numbers.

#### 3. Conclusion & Schema Implications
- **Folder-Entity Integrity:** `entities.kind = 'folder'` **strictly holds** for `maps`. Every valid map is uniquely defined by a single folder: `maps/<MapName>/`.
- **Anchor Marker Rule:** A map is recognized **only** by the presence of `global.cfg` directly at depth 1 relative to `maps/` (`maps/<MapName>/global.cfg`). Subfolder instances (`maps/<MapName>/*/.../global*.cfg`) are inert scenario backups and must **not** spawn additional entity rows.
- **Chrono Representation:** Chrono events are sub-features of the parent map entity. They are discovered via `maps/<MapName>/Chrono/<Event>/Chrono.cfg` and can be modeled either as child entities (`parent_entity_id`) or in an `entity_chrono_events` table.

---

## 🔗 Stage 3 — Relationship Findings

### Investigation A1 — `[type]` Enumeration

#### 1. Frequency Table
Census of all 2,537 vehicle definition files (1,485 `.bus` and 1,052 `.ovh`) across `Vehicles/`:

| `[type]` Value | `.bus` Count | `.ovh` Count | Total Files | Sample Folders |
| :---: | ---: | ---: | ---: | :--- |
| *(missing)* | 1,433 | 397 | 1,830 | `Vehicles/MAN_NL_NG`, `Vehicles/MAN_SD200`, `Vehicles/Citr_BX`, `Vehicles/VW_T3` |
| **`2`** | 52 | 648 | 700 | `Vehicles/Tram_NF6D`, `Vehicles/A3`, `Vehicles/BR132`, `Vehicles/DUS_ICE3`, `Vehicles/DGBgqe` |
| **`3`** | 0 | 7 | 7 | `Vehicles/B727`, `Vehicles/HB76_EC_135`, `Vehicles/Mi8MT`, `Vehicles/HB76_Moewe` |

No other values (e.g. `0` or `1`) exist in the corpus.

#### 2. Category Correlation
Sampling across distinct vehicle types reveals strict domain boundaries:

| `[type]` Value | Primary Domain | Physical / Simulation Behavior | Sample Folders |
| :---: | :--- | :--- | :--- |
| *(missing)* | **Road Vehicle** (Bus, Car, Truck, Van) | Steered road vehicle subject to asphalt road friction and traffic rules. | `MAN_SD200`, `HH_Stadtbus_2017`, `Citr_BX`, `Gladbeck_KIPKW1` |
| **`2`** | **Rail / Track Vehicle** (Train, Tram, Subway, Freight wagon) | Track-bound vehicle constrained to rail splines, utilizing bogie kinematics (`[boogies]`), pantographs (`[contact_shoe]`), and train consist coupling (`.zug`). (Also applied to bikes/scooters on narrow dedicated path tracks). | `Tram_NF6D`, `Muenchen_M5_Wagen`, `A3`, `BR132`, `DUS_ICE3`, `Gueterwagen` |
| **`3`** | **Aircraft / Ambient Airborne** (Airplane, Helicopter, Bird) | 3D airspace path trajectory without ground wheel friction. | `B727`, `HB76_EC_135` (Helicopter), `Mi8MT`, `HB76_Moewe` (Seagull) |

#### 3. Conclusion & Schema Implications
- **Semantic Classification:** `[type]` serves as OMSI's top-level vehicle domain selector: default/omitted = Road, `2` = Rail, `3` = Aircraft.
- **Dedicated Column (`vehicle_class`):** Rather than generic free-text tags, the schema should store an explicit enumerated column:
  ```sql
  vehicle_class TEXT NOT NULL CHECK(vehicle_class IN ('road', 'rail', 'aircraft'))
  ```
  Mapped as: `2` $\rightarrow$ `'rail'`, `3` $\rightarrow$ `'aircraft'`, omitted/default $\rightarrow$ `'road'`.
- **Query Ergonomics:** Combined with `is_playable`, this provides instant, indexed filtering for UI and engine systems:
  - Drivable buses: `WHERE is_playable = 1 AND vehicle_class = 'road'`
  - Drivable trams: `WHERE is_playable = 1 AND vehicle_class = 'rail'`
  - Train consists: `WHERE vehicle_class = 'rail'`
  - Ambient road traffic: `WHERE is_playable = 0 AND vehicle_class = 'road'`

---

### Investigation A2 — Alias Hypothesis Verification

#### 1. Reference Census across AI Lists and Train Consists
Scanned all 92 AI lists across `maps/` and all 208 `.zug` consist files across `Trains/`, `Vehicles/`, and `Splines/`:

| File Type / Context | `.bus` References | `.ovh` References | Dominant Vehicle Types |
| :--- | ---: | ---: | :--- |
| **Map AI Lists** (`ailists.cfg`, `ailists_#upd.cfg`) | 1,507 | 1,004 | `.bus` for scheduled lines / player buses; `.ovh` for ambient traffic, police, trucks, trains |
| **Train Consists** (`*.zug`) | 0 | 1,166 | 100% `.ovh` (locomotives, railcars, freight wagons, subway units) |

#### 2. Cross-Reference of the 24 Twin Folders
Inspected the 24 folders containing identically named `.bus` and `.ovh` twins (e.g. `HH109_KIPKW1`–`7`, `Gladbeck_KIPKW1`–`7`, `Luzern_*`):
- In shipped map AI lists, these vehicles are referenced via their `.bus` paths (e.g. `Vehicles\HH109_KIPKW1\ai_hh109kipkw1.bus` across 11 maps).
- In addon AI lists (e.g. `Addons/Halycon_AI_Pack_2`, `Halycon_AI_Pack_4`, `Halycon_AI_Pack_7` Krumenaab ailists), the exact same vehicles are referenced via their `.ovh` paths (`ai_hh109kipkw1.ovh`, `ai_hh109kipkw5_pol.ovh`).
- Neither format is redundant; both are actively referenced across independent content packs.

#### 3. Single-Extension References
- **`.bus`-Only References in AI Lists:** Heavily present (e.g. `Vehicles\MAN_NL_NG_263\MAN_NL263.bus`, `Vehicles\Muenchen_MAN_LC\MAN_LC_A21_AI.bus`, `Vehicles\MB_C2_EN_BVG\MB_KI_C2_E6_Gn_Hybrid_main.bus`). Player bus models running on scheduled timetables are defined as `.bus`.
- **`.ovh`-Only References in AI Lists:** Dedicated background vehicles (e.g. `Vehicles\Citr_BX\BX.ovh`, `Vehicles\A3\A3L_71b_S.ovh`, `Vehicles\HB76_Feuerwehr\Feuerwehr_active.ovh`, `Vehicles\HH20_S-Bahn BR474\BR474.ovh`).

#### 4. Conclusion & Schema Implications
- **Alias Hypothesis Confirmed:** The `.bus`/`.ovh` twins are genuine compatibility aliases created by addon developers so that maps referencing either convention resolve cleanly.
- **Index Every Marker File:** The catalogue must not discard or hide either file. Every marker file (`.bus` and `.ovh`) must be indexed as a resolvable vehicle entry point.
- **Entity Resolution Model:** Each marker file represents an addressable vehicle definition. To avoid duplicate logical entities while preserving 100% path resolution:
  - Each marker file has a record (either as an individual `entity` row or in an `entity_markers` table).
  - An optional `alias_of_id` (or `parent_entity_id`) can link the alias `.ovh` to its `.bus` sibling when they share identical geometry/folder.
  - Path lookup queries (resolving `Vehicles\foo\bar.bus` or `Vehicles\foo\bar.ovh`) resolve directly in $O(1)$ time via the indexed file path.

---

### Investigation A3 — Trailer Composition

#### 1. Coupling Census & Target Extensions
Inspected all 2,537 vehicle definitions for kinematic coupling keywords:
- **`[couple_back]` (Trailing vehicle link):** **846 occurrences**
  - Targets `.bus`: **641** (75.8%) — standard bus trailers (`GN92_trail.bus`, `C2G_trail.bus`).
  - Targets `.ovh`: **191** (22.6%) — train/subway coaches and AI trailers (`A3L_67_S.ovh`, freight cars).
  - Empty / unassigned: **14** (1.6%).
- **`[couple_front]` (Leading vehicle link):** **21 occurrences** (20 `.ovh`, 1 `.bus`) used for bi-directional train cabs and push-pull consists.
*(Note: `[coupling_front]` and `[coupling_back]` are numeric physical damping parameters, distinct from the file-linking `[couple_*]` keywords).*

#### 2. Folder Topology: Same-Folder vs Cross-Folder
- **Same Folder (520 instances / 61.5%):** Trailing segments defined as bare filenames living in the same folder as the tractor (e.g., `MAN_GN92_main.bus` $\rightarrow$ `MAN_GN92_trail.bus`).
- **Cross-Folder (326 instances / 38.5%):** Segments referenced via relative path into a different vehicle folder (e.g., `Vehicles/BR132/br132.ovh` $\rightarrow$ `..\Bm238\Bm238.bus`). Common in modular railway traction.

#### 3. Double-Articulated & Multi-Segment Vehicles
Double-articulated vehicles **definitely exist** in the corpus (177 chained instances found):
- `Vehicles/HH_Doppelgelenkbus/`:
  - `AGG300_87_main.bus` (Front tractor) `[couple_back]` $\rightarrow$ `AGG300_87_mid.bus`
  - `AGG300_87_mid.bus` (Middle section) `[couple_back]` $\rightarrow$ `AGG300_87_trail.bus` (Rear trailer).
- `Vehicles/Tram_NF6D/`: Multi-section articulated tram segments coupled in series.

#### 4. Conclusion & Schema Implications
- **Entity Model:** Multi-segment vehicles are composed of independent vehicle definition files chained together via coupling references.
- **Reference-Based Composition:** A dedicated table is unnecessary. The general-purpose `entity_references` table handles this cleanly:
  ```sql
  entity_references (source_id, target_file_id, role)
  ```
  with `role = 'trailer_rear'` (from `[couple_back]`) and `role = 'trailer_front'` (from `[couple_front]`).
- **Hierarchy Representation:** This naturally represents arbitrary topologies: single trailers (1:1), double-articulated chains (A $\rightarrow$ B $\rightarrow$ C), reversible railcars, and cross-folder consist references.
- **Menu Visibility (`is_trailer` / `is_playable`):** Dedicated trailer files (lacking driver positions) should be flagged `is_playable = 0` so only the front tractor unit appears in the user vehicle selection interface.

---

### Investigation A4 — Spec Reconciliation

#### 1. Discrepancy Table

| Spec Claim (`misc-types.md`) | Investigation Finding | Decision | Rationale |
| :--- | :--- | :--- | :--- |
| **Secondary format framing:** Treated as a miscellaneous minor type relegated to `misc-types.md`. | **Co-equal primary vehicle marker:** 71.5% of vehicle folders (393 folders) contain *only* `.ovh`. All 208 train consists reference `.ovh` exclusively. | **Update the spec** | Evidence is conclusive: `.ovh` is the foundational format for OMSI rail, ambient aircraft, and AI traffic. |
| **Trailer usage:** States `.ovh` is used for trailers. | **Trailers use both extensions:** Road bus trailers use `.bus` (75.8%), while rail and AI trailers use `.ovh` (22.6%). | **Update the spec** | Ground truth shows road trailers (`MAN_GN92_trail.bus`) are `.bus` files, not `.ovh`. |
| **`[type]` keyword:** Counted in census (645 files) but semantics undocumented. | **Simulation domain selector:** `[type]` strictly defines domain: omitted = road, `2` = rail, `3` = aircraft. | **Update the spec** | 100% correlation across 2,537 vehicle files confirms exact enumerated domain meaning. |
| **Q-MS01 Open Question:** "Does OMSI treat `.ovh` differently from `.bus`?" | **Engine behavior resolved:** OMSI hides `.ovh` from player vehicle selection menus (`is_playable = 0`); both share identical keyword parsers. | **Update the spec** | Closed by AI list, consist, and UI menu analysis. |
| **Corpus file count:** 1,061 files. | **1,061 total:** 1,052 in `Vehicles/`, 9 static vehicle objects in `Sceneryobjects/`. | **Leave as-is** | Spec count is accurate across the full installation. |

#### 2. Proposed Specification Patch
Proposed update to [`docs/src/formats/text/misc-types.md`](../formats/text/misc-types.md) (not applied, awaiting user authorization):

```diff
--- a/docs/src/formats/text/misc-types.md
+++ b/docs/src/formats/text/misc-types.md
@@ -51,4 +51,11 @@
-An `.ovh` ("other vehicle") is used for AI traffic, trailers, parked vehicles, trams and rail vehicles. It is a **`.bus`-style** vehicle definition: all of its keywords are the `.bus` vocabulary. Census over 1,061 files [C] (count of files, typical parameter lines):
+An `.ovh` ("other vehicle") is a co-equal vehicle definition file alongside [`.bus`](vehicles/bus.md), used for non-player vehicles: AI road traffic, rail vehicles (trains, trams, subways), aircraft, trailers, and parked vehicles. In OMSI, `.ovh` files never appear in the player vehicle selection UI (`is_playable = false`), but are referenced extensively by map AI lists (`ailists.cfg`, 1,004 references) and train consists (`.zug`, 1,166 references, 100% of consist entries). It shares the identical keyword grammar and parser as `.bus`. Census over 1,061 files [C] (1,052 in `Vehicles/`, 9 static scenery objects):
+
+The `[type]` keyword acts as the top-level vehicle simulation domain selector:
+- `[type]` omitted: Road vehicle (steered road physics; buses, cars, trucks)
+- `[type] 2`: Rail / track vehicle (constrained to rail splines, bogies, train consists; trains, subways, trams)
+- `[type] 3`: Aircraft / ambient airborne (3D trajectory without ground friction; airplanes, helicopters, birds)
@@ -223,1 +230,1 @@
-| Q-MS01 | Does OMSI treat `.ovh` differently from `.bus` (e.g. which keywords are ignored for AI vehicles)? | 🟡 Medium |
+| Q-MS01 | ~~Does OMSI treat `.ovh` differently from `.bus`~~ — resolved: `.ovh` is hidden from the player vehicle selection UI (AI-only); physics and keywords follow `.bus`, with `[type]` selecting road/rail/air. | ✅ |
```

---

## 🔗 Part B — Cross-Mode Relationship Mapping

### 1. Cross-Mode Reference Verification
Ground-truth inspection of configurations across OMSI content types confirmed 16 concrete cross-mode relationship edges:

| Source Mode | Source Category | Target Mode | Target Category | Reference Syntax / Keyword | Cardinality | Optional? | Sample File & Real Corpus Target |
| :--- | :--- | :--- | :--- | :--- | :---: | :---: | :--- |
| **Folder-entity** | `vehicle` | **Pool** | `sound` | `[sound]` path / relative WAV | $1 \to N$ | Yes | `Vehicles/HH109_KIPKW1/ai_hh109kipkw1.ovh` $\to$ `..\..\Sounds\AI_Cars\sound.cfg` |
| **Folder-entity** | `vehicle` | **Pool** | `texture` | `[matl]` / path in `model.cfg` | $1 \to N$ | Yes | `Vehicles/MAN_NL_NG/Model/model_EN92.cfg` $\to$ `..\..\Texture\envmap.bmp` |
| **Folder-entity** | `vehicle` | **File-entity** | `font` | `[texttexture]` line 6 | $1 \to N$ | Yes | `Vehicles/MAN_SD200/Model/model_SD77.cfg` $\to$ `Fonts/DIN_Narrow.oft` |
| **Folder-entity** | `vehicle` | **Folder-entity** | `vehicle` | `[couple_back]` / `[couple_front]` | $1 \to N$ | Yes | `Vehicles/MAN_NL_NG/MAN_GN92_main.bus` $\to$ `MAN_GN92_trail.bus` |
| **Folder-entity** | `map` | **Folder-entity** | `vehicle` | Path line in `ailists.cfg` | $1 \to N$ | No | `maps/Berlin-Spandau/ailists.cfg` $\to$ `Vehicles\MAN_SD200\MAN_SD77.bus` |
| **Folder-entity** | `map` | **Folder-entity** | `money` | `[moneysystem]` block in `global.cfg` | $1 \to 1$ | Yes | `maps/Berlin-Spandau/global.cfg` $\to$ `Money\DM\DM.cfg` |
| **Folder-entity** | `map` | **Folder-entity** | `ticketpack` | `[ticketpack]` block in `global.cfg` | $1 \to 1$ | Yes | `maps/Berlin-Spandau/global.cfg` $\to$ `TicketPacks\Berlin_1\Berlin_1.otp` |
| **Folder-entity** | `map` | **Pool** | `texture` | `[groundtex]` in `global.cfg` | $1 \to N$ | No | `maps/Berlin-Spandau/global.cfg` $\to$ `texture\gras.bmp` |
| **Folder-entity** | `map` | **File-entity** | `spline` | `[spline]` block in `tile_*.map` | $1 \to N$ | No | `maps/Berlin-Spandau/tile_2379_11288.map` $\to$ `Splines\Ruede\rail_concrete_01.sli` |
| **Folder-entity** | `map` | **File-entity** | `sceneryobject` | `[object]` block in `tile_*.map` | $1 \to N$ | No | `maps/Berlin-Spandau/tile_2379_11288.map` $\to$ `Sceneryobjects\Buildings_MC\bk_melanchthon.sco` |
| **File-entity** | `sceneryobject`| **Pool** | `texture` | `[matl]` or `[texture]` in `.sco` | $1 \to N$ | Yes | `Sceneryobjects/Generic/fireworks.sco` $\to$ `texture\fireworks.bmp` |
| **File-entity** | `sceneryobject`| **Pool** | `sound` | `[sound]` in `.sco` $\to$ `.cfg` $\to$ WAV | $1 \to N$ | Yes | `Sceneryobjects/Buildings_MC/bk_melanchthon.sco` $\to$ `Sound/church_ev_sound.cfg` |
| **File-entity** | `spline` | **Pool** | `texture` | `[texture]` block in `.sli` | $1 \to N$ | No | `Splines/invis_sidewalk.sli` $\to$ `texture\sidewalk.bmp` |
| **Folder-entity** | `human` | **Pool** | `texture` | `[model]` $\to$ `.cfg` $\to$ skin texture | $1 \to N$ | No | `Humans/aXYZ/man01.hum` $\to$ `texture\man01_tex.bmp` |
| **File-entity** | `train_consist`| **Folder-entity** | `vehicle` | Path line in `*.zug` | $1 \to N$ | No | `Trains/BR112_Dosto.zug` $\to$ `Vehicles\BR112_X10\BR112.ovh` |
| **File-entity** | `situation` | **Folder-entity** | `map` | `[map]` line in `*.osn` | $1 \to 1$ | No | `Situations/23A 2005.osn` $\to$ `maps\Wien_2_2005_Linie_23A\global.cfg` |

### 2. Path Syntax & Normalization Rules
1. **Windows Backslash Standard:** Configs predominantly use Windows backslashes (`\`): e.g. `Splines\Ruede\rail_concrete_01.sli`, `..\..\Sounds\AI_Cars\sound.cfg`.
2. **Relative Resolution Anchors:**
   - **Root-relative paths:** Strings starting with known root directories (`Vehicles\`, `Sceneryobjects\`, `Splines\`, `Texture\`, `Money\`, `TicketPacks\`, `Fonts\`) resolve directly against the OMSI root directory.
   - **Parent-relative traversal (`..\`):** Paths like `..\..\Sounds\AI_Cars\` traverse upward from the declaring config file's directory.
   - **Bare filenames:** Files like `MAN_GN92_trail.bus` in `[couple_back]` or `sound_GN92.cfg` in `[sound]` resolve inside the immediate folder of the calling definition.
   - **Case-Insensitive Match:** Windows filesystems are case-insensitive. On Linux hosts (or when resolving cross-platform in OPTS), references must be resolved using lower-cased / case-folded lookups.

### 3. Cardinality & Cycle Characteristics
- **Fan-Out (1 $\to$ N):** Maps fan out to thousands of splines and scenery objects; vehicles fan out to dozens of scripts, sounds, and textures.
- **Cycles in Trailers / Consists:** Bidirectional coupling (`A3L_67_K.ovh` $\leftrightarrow$ `A3L_67_S.ovh`) creates reference cycles. Resolvers and walkers must track visited entity IDs to prevent infinite loops.
- **Dangling References (Missing Assets):** Community addons frequently reference missing textures, splines, or sound files. The relationship model must treat foreign references as nullable / unresolved without aborting transactions.

### 4. Schema Implications for Reference Graph
- **Unified `entity_references` Table:** A single relational table connects source entities to target files/entities:
  ```sql
  CREATE TABLE entity_references (
      reference_id     INTEGER PRIMARY KEY AUTOINCREMENT,
      source_entity_id INTEGER NOT NULL,
      target_file_id   INTEGER,            -- NULL when the referenced asset is missing
      raw_reference    TEXT NOT NULL,      -- original verbatim string
      role             TEXT NOT NULL,      -- 'ai_vehicle', 'spline', 'sceneryobject', 'sound', 'texture', 'font', 'moneysystem', 'ticketpack', 'trailer_rear', 'trailer_front'
      is_resolved      INTEGER NOT NULL DEFAULT 0,
      FOREIGN KEY (source_entity_id) REFERENCES entities(entity_id) ON DELETE CASCADE,
      FOREIGN KEY (target_file_id) REFERENCES files(file_id) ON DELETE SET NULL
  );
  ```
- **Bidirectional Indexing:**
  - `CREATE INDEX idx_ref_source ON entity_references(source_entity_id, role);` supports: *"What does map X depend on?"*
  - `CREATE INDEX idx_ref_target ON entity_references(target_file_id, role);` supports: *"Which maps/vehicles use spline or texture Y?"*
  - `CREATE INDEX idx_ref_unresolved ON entity_references(is_resolved) WHERE is_resolved = 0;` supports instant health-checking: *"List all broken / missing dependencies in this installation."*

---

## 🏛️ Part C — Schema Implications & Requirements Summary

### 1. Entity Identity & Category Taxonomy
The catalogue models exactly **16 active entity categories** across two operational modes:
- **Folder-Entity Mode (5 categories):** `vehicle` (vehicles & railcars), `map` (game worlds), `ticketpack` (ticketing profiles), `money` (currencies), `human` (pedestrians & drivers).
- **File-Entity Mode (11 categories):** `sceneryobject` (`.sco`), `spline` (`.sli`), `font` (`.oft`), `train_consist` (`.zug`), `situation` (`.osn`), `weather` (`.owt`), `option_preset` (`.oop`), `language` (`.olf`), `input_map` (`.kyb`/`.cfg`), `gui_skin` (`.bmp`), `driver` (`.odr`).
- **Resource Pools (0 entities):** `texture`, `sound`, `helper`, `program`, `script` (5 categories). Files within pools are raw resources recorded exclusively in the `files` table; they never instantiate rows in `entities`.

### 2. Multi-Marker Support
- While most categories bind to a single marker (`.sco`, `.sli`, `.otp`, `.zug`), `vehicle` requires **co-equal marker support** (`.bus` and `.ovh`).
- The schema accommodates this by storing the actual marker filename/extension on the entity row (`marker_file_id` pointing to `files`) rather than enforcing a rigid 1:1 database constraint.

### 3. Kinematic & Consist Composition
- Articulated buses, double-articulated buses, trams, and train consists are composed via directed reference links (`[couple_back]`, `[couple_front]`).
- Rather than maintaining an auxiliary `entity_composition` table, composition is modeled directly within `entity_references` using roles `trailer_rear` and `trailer_front`.
- Trailer entities are flagged `is_playable = 0` to prevent unsteerable trailing segments from appearing in the user vehicle selection menu.

### 4. Strongly-Typed Relational Reference Storage
The `entity_references` table uses a fixed relational schema:
- `source_entity_id INTEGER NOT NULL`: Enclosing entity owner.
- `target_file_id INTEGER`: Foreign key to `files(file_id)`, set to `NULL` if the target is missing on disk.
- `raw_reference TEXT NOT NULL`: Verbatim string from configuration to preserve diagnostic provenance.
- `role TEXT NOT NULL`: Discriminator enum (`'ai_vehicle'`, `'spline'`, `'sceneryobject'`, `'sound'`, `'texture'`, `'font'`, `'moneysystem'`, `'ticketpack'`, `'trailer_rear'`, `'trailer_front'`).
- `is_resolved INTEGER NOT NULL DEFAULT 0`: Explicit status flag for fast indexing and health checking.

### 5. Pool Resource Indexing
- Because pool files lack entity wrappers, cross-mode queries ("Find textures for vehicle X") join `entities` $\to$ `entity_references` $\to$ `files`.
- The covering index `idx_ref_source(source_entity_id, role)` ensures $O(1)$ lookup for all assets needed by an entity at runtime.

### 6. Reverse Lookup & Staleness Invalidation
- **Reverse Dependency Queries:** Supported by index `idx_ref_target(target_file_id, role)`.
- **Mod Management & Safe Deletion:** Enables instant validation before deleting scenery packs: *"Are any installed maps referencing splines in this folder?"*
- **Fine-Grained Staleness Invalidation:** When a modified timestamp changes on a shared texture or sound pool file, the catalogue instantly identifies the exact set of dependent vehicle models or maps requiring cache invalidation without scanning the filesystem.

---

## ⚠️ Stage 4 — Edge Cases & Anomalies

Grounded in full-installation scans of `/home/louisgoodnews/nvme2n1/steamapps/common/OMSI 2`:

### 1. Backup Folders & Backup Files
- **Backup Folders (5 found):**
  - `maps/Grundorf/texture/map/Backup`
  - `Sceneryobjects/NoNameProdution/NNP_Crossing/Backups`
  - `Vehicles/HH109_Stadtgelenkbus1992_HHA/Sound/backup`
  - `Vehicles/GPM_C2/Sound/tueren/bak`
- **Backup Files by Extension:**
  - `*.bu`: **1,031 files** (created automatically by the OMSI Map Editor when saving `.sco` objects).
  - `*.bak`: **44 files** (manual editor/modder backups).
  - `*.aod`: **24 files** (OMSI tile autosave backups).
  - `*.old` / `*.orig`: **10 files**.
- **Discovery Rule:** Never register backup files or folders as entities. The scanner indexes them as `file_kind = 'backup'` in `files` so physical disk space is tracked, but filters them out during entity marker scanning (`WHERE is_backup = 0`).

### 2. Duplicate Author Folders & Pack Splitting
- **Examples in `Splines/`:** `Splines/ADDON_Gladbeck` and `Splines/Gladbeck` coexist as separate directories created by different installer versions.
- **Examples in `Sceneryobjects/`:** The Gladbeck/Ruhr publisher distributed scenery across 12 distinct top-level directories: `ADDON_Gladbeck`, `ADDON_Gladbeck_Kreuz`, `ADDON_Gladbeck_MAK`, `ADDON_Gladbeck_NF6D`, `ADDON_Gladbeck_Yufa`, `ADDON_RUHR`, `ADDON_RUHR_MAK`, `ADDON_RUHR_NF6D`, `ADDON_RUHR_Yufa`, `ADDON_Ruhr_Kreuz`, etc.
- **Discovery Rule:** In File-Entity mode, each `.sco` and `.sli` is an autonomous entity identified by its full relative path (`Sceneryobjects/ADDON_Gladbeck_Kreuz/kreuzung.sco`). Distinct author directory prefixes do not cause collisions.

### 3. Deeply Nested Mods
- **Scenery Objects (`.sco`):** Span depths 1 through 6 relative to `Sceneryobjects/`:
  - Depth 1: 21,509 files (`Sceneryobjects/Generic/lamp.sco`)
  - Depth 4: 1,742 files (`Sceneryobjects/Express 91.06/Abribus - TCSP/Abribus TCSP 2016/Bande TCSP/Bande TCSP.sco`)
  - Depth 5: 255 files (`Sceneryobjects/Express 91.06/.../Destinations/06ABCD pour Massy/Etiquette.sco`)
  - Depth 6: 2 files
- **Splines (`.sli`):** Span depths 0 through 4 (e.g. `Splines/invis_street.sli` at depth 0, depth 4 in `Express 91.06`).
- **Vehicles (`.bus`/`.ovh`):** Span depths 1 through 3:
  - Depth 1: 2,421 files (`Vehicles/MAN_NL_NG/MAN_EN92_main.bus`)
  - Depth 2: 95 files (`Vehicles/Muenchen_KI/ap1/KI_Audi_A3.ovh`)
  - Depth 3: 21 files (`Vehicles/Muenchen_KI/ap4/A4_Avant/A4_Avant.ovh`)
- **Discovery Rule:** The scanner must recursively traverse all subdirectories. For File-Entities, any matching marker extension is an entity regardless of depth. For Folder-Entities, the entity folder is defined as the immediate enclosing directory of the marker file.

### 4. Non-Entity Folders & Grouping Folders
- **Intermediate Pack Groups:** `Vehicles/Muenchen_KI` is a pack directory containing subdirectories (`ap1`, `ap4`), but contains no marker file directly at its root.
- **Shared Vehicle Pools:**
  - `Vehicles/Announcements/`: Shared bus audio announcements (`IBIS` wav files).
  - `Vehicles/Anzeigen/`: Matrix display bitmap textures.
- **Abandoned / Broken Mod Folders:** `Vehicles/MB_O530_Modded/`, `Vehicles/MB_O530_G_Modded/` (contain texture scraps but zero `.bus`/`.ovh`).
- **Discovery Rule:** A directory under `Vehicles/` is **never** an entity unless it contains an anchor marker (`.bus` or `.ovh`). Marker-less folders are treated as structural groupings or pools.

### 5. Non-Standard Extensions (Authoring & Tool Remnants)
Scanners encountered extensive development and authoring tool files left behind in addon distributions:
- `*.pdn` (Paint.NET layered repaint projects): **695 files** (e.g. in `Repaints/` and vehicle texture folders).
- `*.blend` (Blender 3D mesh project files): **381 files**.
- `*.psd` (Photoshop project files): **174 files**.
- `*.zip` / `*.rar` / `*.7z` (Unpacked archive leftovers): **63 files**.
- `*.cmd` / `*.bat` (Windows batch scripts): **14 files**.
- `*.lnk` (Windows desktop shortcuts): **4 files**.
- **Discovery Rule:** Index these files in `files` with appropriate metadata (`file_kind = 'authoring' | 'archive'`), but exclude them from entity definition parsing.

### 6. Empty Folders
- **Truly Empty Directories (0 files, 0 subdirs):** 28 folders found (e.g. `Vehicles/-hof Dateien/`, `Screenshots/`).
- **Marker-less Directories:** 31 directories in `Vehicles/` lack markers.
- **Discovery Rule:** Empty folders are recorded in `directories` with `file_count = 0` and skipped during entity generation.

### 7. Symlinks (The 27,211 `.hof` Symlink Fleet)
- **Census:** Exactly **27,211 symlinks** exist across the installation.
- **Broken Links:** **0 broken symlinks** (all 27,211 resolve to valid files).
- **Extension Breakdown:** **100% of symlinks are `.hof` files** (`000_S-Bahn-Berlin.hof`, `Spandau 92.hof`, etc.) symlinked from a centralized `.hof Dateien/` depot into dozens of vehicle folders.
- **Discovery Rule & Schema Support:**
  - The `files` table must store `is_symlink INTEGER NOT NULL DEFAULT 0` and `symlink_target TEXT`.
  - The Rust walker must configure symlink handling to prevent recursive loops (`follow_links(false)` during initial pass) and record canonical target IDs.

---

## 📋 Stage 5 — Metadata Selection & Justification

Every field in the schema is strictly justified by the application's query requirements, integrity constraints, and fine-grained staleness invalidation. Speculative columns have been rejected.

### 1. Catalogue Metadata (`meta` table)
Key-value configuration store for scanner runtime and migration provenance:
- `key TEXT PRIMARY KEY`: Attribute name.
- `value TEXT NOT NULL`: Attribute value.
- **Justification:** Tracks `schema_version` (for SQLite migration safety), `omsi_root` (to detect if the database was moved to a different folder/machine), `last_scan_completed_at`, `total_files`, and `total_entities`.

### 2. Directories Table (`directories`)
Represents the filesystem folder tree structure:
- `dir_id INTEGER PRIMARY KEY AUTOINCREMENT`: Unique internal identifier.
- `parent_dir_id INTEGER REFERENCES directories(dir_id)`: Self-referencing FK for hierarchical tree traversal.
- `rel_path TEXT NOT NULL UNIQUE`: Canonical relative directory path (e.g. `Vehicles/MAN_NL_NG`).
- `dir_name TEXT NOT NULL`: Directory basename (e.g. `MAN_NL_NG`).
- `depth INTEGER NOT NULL`: Nesting level from OMSI root (0 = root, 1 = category, 2 = pack/vehicle).
- `mtime_sec INTEGER NOT NULL`: Directory modification timestamp.
- `child_dir_count INTEGER NOT NULL DEFAULT 0`: Direct child directory count.
- `child_file_count INTEGER NOT NULL DEFAULT 0`: Direct child file count.
- **Justification:** Enables shallow subtree filtering, structural grouping queries, and fast directory-level staleness detection without scanning individual files.

### 3. Files Table (`files`)
Tracks all physical and symlinked files within the scope of the installation:
- `file_id INTEGER PRIMARY KEY AUTOINCREMENT`: Unique internal file identifier.
- `dir_id INTEGER NOT NULL REFERENCES directories(dir_id) ON DELETE CASCADE`: Enclosing parent folder.
- `rel_path TEXT NOT NULL UNIQUE`: Canonical relative file path (e.g. `Vehicles/MAN_NL_NG/MAN_EN92_main.bus`).
- `file_name TEXT NOT NULL`: File basename (e.g. `MAN_EN92_main.bus`).
- `extension TEXT NOT NULL`: Normalized lowercase extension without leading dot (e.g. `bus`, `ovh`, `sco`, `sli`, `o3d`, `dds`, `wav`).
- `size_bytes INTEGER NOT NULL`: Physical file size on disk.
- `mtime_sec INTEGER NOT NULL`: File modification timestamp in seconds since Unix epoch.
- `is_symlink INTEGER NOT NULL DEFAULT 0`: Boolean flag (1 for symlink, 0 for regular file).
- `symlink_target TEXT`: Raw target path if `is_symlink = 1`.
- `is_backup INTEGER NOT NULL DEFAULT 0`: Boolean flag for backup file types (`.bu`, `.bak`, `.aod`, `.old`).
- **Justification:** Answers path lookup queries, extension filtering, storage footprint calculation, fine-grained staleness invalidation, and accurately tracks the 27,211 symlinked `.hof` files.

### 4. Entities Table (`entities`)
First-class gameplay and simulation objects (vehicles, maps, scenery objects, splines):
- `entity_id INTEGER PRIMARY KEY AUTOINCREMENT`: Unique entity identifier.
- `category TEXT NOT NULL`: One of the 16 verified categories (`'vehicle'`, `'map'`, `'sceneryobject'`, `'spline'`, `'font'`, `'ticketpack'`, `'money'`, `'human'`, `'train_consist'`, `'situation'`, `'weather'`, `'option_preset'`, `'language'`, `'input_map'`, `'gui_skin'`, `'driver'`).
- `entity_mode TEXT NOT NULL CHECK(entity_mode IN ('folder', 'file'))`: Structural mode (`'folder'` or `'file'`).
- `name TEXT NOT NULL`: Display name (extracted from `[friendlyname]`, `[name]`, or derived from filename).
- `marker_file_id INTEGER NOT NULL REFERENCES files(file_id) ON DELETE CASCADE`: Foreign key to the defining marker file.
- `entity_dir_id INTEGER NOT NULL REFERENCES directories(dir_id) ON DELETE CASCADE`: Enclosing directory.
- `is_playable INTEGER NOT NULL DEFAULT 1`: Boolean switch for user-selectable vehicles (1 for player buses, 0 for AI traffic, trains, background vehicles, and trailers).
- `vehicle_class TEXT CHECK(vehicle_class IN ('road', 'rail', 'aircraft'))`: Vehicle domain selector from `[type]` (`NULL` for non-vehicles).
- `alias_of_id INTEGER REFERENCES entities(entity_id) ON DELETE SET NULL`: Points to the primary entity when the entity is an alias twin.
- `created_at_mtime INTEGER NOT NULL`: Marker file modification time when scanned.
- **Justification:** Satisfies core UI selection menus ("List drivable buses", "List trains", "List maps") without reading file contents at runtime.

### 5. Entity References Table (`entity_references`)
Directed dependency graph and kinematic consist links:
- `reference_id INTEGER PRIMARY KEY AUTOINCREMENT`: Unique reference identifier.
- `source_entity_id INTEGER NOT NULL REFERENCES entities(entity_id) ON DELETE CASCADE`: Entity declaring the dependency.
- `target_file_id INTEGER REFERENCES files(file_id) ON DELETE SET NULL`: Referenced target file (`NULL` if asset is missing).
- `raw_reference TEXT NOT NULL`: Verbatim reference string as declared in the source config.
- `role TEXT NOT NULL`: Semantic role (`'trailer_rear'`, `'trailer_front'`, `'ai_vehicle'`, `'spline'`, `'sceneryobject'`, `'sound'`, `'texture'`, `'font'`, `'moneysystem'`, `'ticketpack'`, `'model'`, `'script'`, `'constfile'`).
- `is_resolved INTEGER NOT NULL DEFAULT 0`: Resolution flag (1 if target file exists, 0 if dangling/broken).
- **Justification:** Powers component listing, missing-asset health checks, reverse dependency lookup, and safe uninstallation.

---

## 🔍 Stage 6 — Query Survey & SQL Mapping

The path catalogue must efficiently satisfy the OPTS engine, user interface, and maintenance workflows. Below is the comprehensive survey of required queries with concrete SQL sketches and index / view justifications.

### Query 1: List all drivable (player) vehicles
- **Application Context:** Populates the "Select Bus / Vehicle" dialog when a player starts or changes buses in OMSI. Must be instantaneous.
- **SQL Sketch:**
  ```sql
  SELECT 
      e.entity_id,
      e.name,
      e.vehicle_class,
      f.rel_path AS marker_path,
      d.rel_path AS folder_path
  FROM entities e
  JOIN files f ON e.marker_file_id = f.file_id
  JOIN directories d ON e.entity_dir_id = d.dir_id
  WHERE e.category = 'vehicle' 
    AND e.is_playable = 1
  ORDER BY e.name COLLATE NOCASE;
  ```
- **Performance Strategy:** Covered by partial index `idx_entities_playable(category, is_playable)`. Warranted for a dedicated ergonomic view `v_drivable_vehicles`.

### Query 2: List all AI-only vehicles
- **Application Context:** Used by map editors and traffic simulation engines to select ambient traffic cars, trucks, and background railcars.
- **SQL Sketch:**
  ```sql
  SELECT 
      e.entity_id,
      e.name,
      e.vehicle_class,
      f.rel_path AS marker_path
  FROM entities e
  JOIN files f ON e.marker_file_id = f.file_id
  WHERE e.category = 'vehicle' 
    AND e.is_playable = 0
  ORDER BY e.vehicle_class, e.name COLLATE NOCASE;
  ```
- **Performance Strategy:** Warranted for view `v_ai_vehicles`.

### Query 3: Given a vehicle, list its component files by role
- **Application Context:** Used when loading a vehicle into the simulation engine to bind 3D models, audio configs, scripts, and trailers.
- **SQL Sketch:**
  ```sql
  SELECT 
      r.role,
      r.raw_reference,
      f.rel_path AS resolved_path,
      f.size_bytes,
      r.is_resolved
  FROM entity_references r
  LEFT JOIN files f ON r.target_file_id = f.file_id
  WHERE r.source_entity_id = :entity_id
  ORDER BY r.role, r.raw_reference;
  ```
- **Performance Strategy:** Backed by covering index `idx_ref_source(source_entity_id, role)`.

### Query 4: Find a file by relative path
- **Application Context:** Fundamental $O(1)$ path resolver for engine asset loaders (e.g. resolving `Splines\Ruede\rail_concrete_01.sli` or `Texture\envmap.bmp`).
- **SQL Sketch:**
  ```sql
  SELECT 
      f.file_id,
      f.rel_path,
      f.size_bytes,
      f.mtime_sec,
      f.is_symlink,
      f.symlink_target
  FROM files f
  WHERE f.rel_path = :normalized_path COLLATE NOCASE;
  ```
- **Performance Strategy:** Backed by unique index `idx_files_rel_path_nocase ON files(rel_path COLLATE NOCASE)`.

### Query 5: Find all files with a given extension under a given category
- **Application Context:** Find all `.sli` profiles under `Splines/` or all `.cti` repaints under `Vehicles/MAN_NL_NG/`.
- **SQL Sketch:**
  ```sql
  SELECT 
      f.file_id,
      f.rel_path,
      f.size_bytes
  FROM files f
  JOIN directories d ON f.dir_id = d.dir_id
  WHERE f.extension = :ext
    AND d.rel_path LIKE :category_prefix || '%'
  ORDER BY f.rel_path;
  ```
- **Performance Strategy:** Backed by composite index `idx_files_ext_dir(extension, dir_id)`.

### Query 6: Determine if the catalogue is stale relative to the installation
- **Application Context:** Executed on OPTS startup. Must determine in under 100 milliseconds whether files were added, modified, or removed on disk without scanning all 622,000 files.
- **SQL Sketch:**
  ```sql
  -- Step A: Check root directory mtime and child count
  SELECT value FROM meta WHERE key = 'last_scan_completed_at';

  -- Step B: For top-level category directories, compare mtimes
  SELECT rel_path, mtime_sec 
  FROM directories 
  WHERE depth = 1;
  ```
- **Performance Strategy:** Hierarchical directory tree allows checking the top 35 directories first. If top-level mtimes match, the installation is pristine.

### Query 7: List all entities in a given category
- **Application Context:** General entity enumeration for UI selection lists (e.g. list all maps, list all ticket packs, list all fonts).
- **SQL Sketch:**
  ```sql
  SELECT 
      e.entity_id,
      e.name,
      e.entity_mode,
      f.rel_path AS marker_path,
      d.rel_path AS folder_path
  FROM entities e
  JOIN files f ON e.marker_file_id = f.file_id
  JOIN directories d ON e.entity_dir_id = d.dir_id
  WHERE e.category = :category
  ORDER BY e.name COLLATE NOCASE;
  ```
- **Performance Strategy:** Backed by index `idx_entities_category(category, name COLLATE NOCASE)`.

### Query 8: Reverse lookup — Find which maps or vehicles reference asset X
- **Application Context:** Mod manager safe uninstallation / health check: "Can I safely delete `Sceneryobjects/ADDON_Gladbeck/`?"
- **SQL Sketch:**
  ```sql
  SELECT DISTINCT
      e.entity_id,
      e.category,
      e.name,
      r.role
  FROM entity_references r
  JOIN entities e ON r.source_entity_id = e.entity_id
  WHERE r.target_file_id = :file_id;
  ```
- **Performance Strategy:** Backed by reverse index `idx_ref_target(target_file_id, role)`.

### Query 9: List all broken dependencies / missing assets across the installation
- **Application Context:** OPTS Addon Integrity Doctor / Diagnostic Tool.
- **SQL Sketch:**
  ```sql
  SELECT 
      e.category,
      e.name AS entity_name,
      r.role,
      r.raw_reference
  FROM entity_references r
  JOIN entities e ON r.source_entity_id = e.entity_id
  WHERE r.is_resolved = 0
  ORDER BY e.category, e.name;
  ```
- **Performance Strategy:** Backed by partial index `idx_ref_unresolved ON entity_references(is_resolved) WHERE is_resolved = 0`. Warranted for view `v_missing_references`.

---

## 🗄️ Stage 7 — Schema Proposal

### 7.1 Table Definitions (SQLite DDL)

```sql
-- Path Catalogue Schema v1 (Proposed)
-- Storage: SQLite 3.38+ with WAL mode enabled

PRAGMA foreign_keys = ON;

-- 1. Configuration & Scan Provenance
CREATE TABLE meta (
    key   TEXT PRIMARY KEY NOT NULL,  -- Config parameter (e.g. 'schema_version', 'omsi_root')
    value TEXT NOT NULL               -- Config string value
);

-- 2. Filesystem Directory Tree
CREATE TABLE directories (
    dir_id           INTEGER PRIMARY KEY AUTOINCREMENT,
    parent_dir_id    INTEGER REFERENCES directories(dir_id) ON DELETE CASCADE,
    rel_path         TEXT NOT NULL UNIQUE,                       -- Normalized path relative to OMSI root, forward slashes
    dir_name         TEXT NOT NULL,                              -- Basename of the directory
    depth            INTEGER NOT NULL,                           -- Directory nesting level (0 = root, 1 = category, etc.)
    mtime_sec        INTEGER NOT NULL,                           -- Last modified timestamp in seconds since Unix epoch
    child_dir_count  INTEGER NOT NULL DEFAULT 0,                 -- Number of immediate child subdirectories
    child_file_count INTEGER NOT NULL DEFAULT 0                  -- Number of immediate child files
);

-- 3. File Inventory
CREATE TABLE files (
    file_id        INTEGER PRIMARY KEY AUTOINCREMENT,
    dir_id         INTEGER NOT NULL REFERENCES directories(dir_id) ON DELETE CASCADE,
    rel_path       TEXT NOT NULL UNIQUE,                       -- Normalized path relative to OMSI root (e.g. 'Vehicles/MAN_NL_NG/model/model.cfg')
    file_name      TEXT NOT NULL,                              -- Basename of the file (e.g. 'model.cfg')
    extension      TEXT NOT NULL,                              -- Lowercase extension without leading dot (e.g. 'bus', 'ovh', 'sco', 'dds')
    size_bytes     INTEGER NOT NULL,                           -- Physical file size in bytes
    mtime_sec      INTEGER NOT NULL,                           -- Modification timestamp in seconds since Unix epoch
    is_symlink     INTEGER NOT NULL DEFAULT 0,                 -- 1 if file is a symbolic link, 0 if regular file
    symlink_target TEXT,                                       -- Verbatim target path if is_symlink = 1
    is_backup      INTEGER NOT NULL DEFAULT 0                  -- 1 if backup file extension (.bu, .bak, .aod, .old)
);

-- 4. Discovered Game Content Entities
CREATE TABLE entities (
    entity_id        INTEGER PRIMARY KEY AUTOINCREMENT,
    category         TEXT NOT NULL,                            -- One of 16 categories ('vehicle', 'map', 'sceneryobject', 'spline', etc.)
    entity_mode      TEXT NOT NULL CHECK(entity_mode IN ('folder', 'file')), -- Mode: 'folder' (Folder-entity) or 'file' (File-entity)
    name             TEXT NOT NULL,                            -- User-facing name (from [friendlyname], [name], or filename)
    marker_file_id   INTEGER NOT NULL REFERENCES files(file_id) ON DELETE CASCADE, -- Defining anchor marker file
    entity_dir_id    INTEGER NOT NULL REFERENCES directories(dir_id) ON DELETE CASCADE, -- Primary folder
    is_playable      INTEGER NOT NULL DEFAULT 1,               -- 1 = player-selectable drivable vehicle, 0 = AI-only / background traffic
    vehicle_class    TEXT CHECK(vehicle_class IN ('road', 'rail', 'aircraft')), -- Vehicle domain selector (NULL for non-vehicles)
    alias_of_id      INTEGER REFERENCES entities(entity_id) ON DELETE SET NULL, -- Pointer to primary entity for compatibility twins
    created_at_mtime INTEGER NOT NULL                          -- Modification time of marker file when entity was registered
);

-- 5. Cross-Mode Dependency and Kinematic Graph
CREATE TABLE entity_references (
    reference_id     INTEGER PRIMARY KEY AUTOINCREMENT,
    source_entity_id INTEGER NOT NULL REFERENCES entities(entity_id) ON DELETE CASCADE,
    target_file_id   INTEGER REFERENCES files(file_id) ON DELETE SET NULL, -- NULL if referenced file is missing
    raw_reference    TEXT NOT NULL,                            -- Verbatim reference string as declared in config
    role             TEXT NOT NULL,                            -- 'trailer_rear', 'trailer_front', 'ai_vehicle', 'spline', 'sceneryobject', 'sound', 'texture', 'font', etc.
    is_resolved      INTEGER NOT NULL DEFAULT 0                -- 1 if target file exists on disk, 0 if dangling/broken link
);
```

### 7.2 Index Definitions

```sql
-- Directories Indexes
CREATE INDEX idx_directories_parent ON directories(parent_dir_id);
CREATE INDEX idx_directories_depth_mtime ON directories(depth, mtime_sec);
CREATE INDEX idx_directories_rel_path_nocase ON directories(rel_path COLLATE NOCASE);

-- Files Indexes
CREATE INDEX idx_files_dir ON files(dir_id);
CREATE INDEX idx_files_rel_path_nocase ON files(rel_path COLLATE NOCASE);
CREATE INDEX idx_files_ext_dir ON files(extension, dir_id);
CREATE INDEX idx_files_symlink ON files(is_symlink) WHERE is_symlink = 1;

-- Entities Indexes
CREATE INDEX idx_entities_category_name ON entities(category, name COLLATE NOCASE);
CREATE INDEX idx_entities_playable ON entities(category, is_playable, name COLLATE NOCASE) WHERE is_playable = 1;
CREATE INDEX idx_entities_vehicle_class ON entities(vehicle_class) WHERE vehicle_class IS NOT NULL;
CREATE INDEX idx_entities_dir ON entities(entity_dir_id);
CREATE INDEX idx_entities_marker ON entities(marker_file_id);

-- References Indexes
CREATE INDEX idx_ref_source ON entity_references(source_entity_id, role);
CREATE INDEX idx_ref_target ON entity_references(target_file_id, role);
CREATE INDEX idx_ref_unresolved ON entity_references(is_resolved) WHERE is_resolved = 0;
```

### 7.3 Ergonomic View Definitions

```sql
-- 1. Player-Drivable Vehicles View (Instant UI picker)
CREATE VIEW v_drivable_vehicles AS
SELECT 
    e.entity_id,
    e.name,
    e.vehicle_class,
    f.rel_path AS marker_file,
    d.rel_path AS vehicle_folder,
    f.size_bytes AS marker_size,
    e.created_at_mtime
FROM entities e
JOIN files f ON e.marker_file_id = f.file_id
JOIN directories d ON e.entity_dir_id = d.dir_id
WHERE e.category = 'vehicle'
  AND e.is_playable = 1;

-- 2. AI-Only Vehicles View (AI traffic & railcars)
CREATE VIEW v_ai_vehicles AS
SELECT 
    e.entity_id,
    e.name,
    e.vehicle_class,
    f.rel_path AS marker_file,
    d.rel_path AS vehicle_folder
FROM entities e
JOIN files f ON e.marker_file_id = f.file_id
JOIN directories d ON e.entity_dir_id = d.dir_id
WHERE e.category = 'vehicle'
  AND e.is_playable = 0;

-- 3. Game Maps View
CREATE VIEW v_maps AS
SELECT 
    e.entity_id,
    e.name,
    d.rel_path AS map_folder,
    f.rel_path AS global_cfg_path,
    d.child_dir_count AS tile_count,
    e.created_at_mtime
FROM entities e
JOIN files f ON e.marker_file_id = f.file_id
JOIN directories d ON e.entity_dir_id = d.dir_id
WHERE e.category = 'map';

-- 4. Missing References / Asset Diagnostic View (Addon Doctor)
CREATE VIEW v_missing_references AS
SELECT 
    r.reference_id,
    e.category AS source_category,
    e.name AS source_entity_name,
    d.rel_path AS source_folder,
    r.role,
    r.raw_reference
FROM entity_references r
JOIN entities e ON r.source_entity_id = e.entity_id
JOIN directories d ON e.entity_dir_id = d.dir_id
WHERE r.is_resolved = 0;
```

### 7.4 Staleness Algorithm (Pseudocode)

```python
def check_catalogue_staleness(db_conn, omsi_root_path) -> StalenessResult:
    """
    Sub-100ms multi-tier staleness verification algorithm.
    Checks top-level directory mtimes before walking subtrees.
    """
    # Step 1: Verify root directory existence and configuration
    stored_root = db_conn.query_one("SELECT value FROM meta WHERE key = 'omsi_root'")
    if not os.path.exists(omsi_root_path) or stored_root != omsi_root_path:
        return StalenessResult.InvalidRoot

    # Step 2: Tier 1 - Fast Category Directory Scan (35 directories)
    # Check if any top-level category folder mtime has changed
    stored_categories = db_conn.query_all("SELECT rel_path, mtime_sec FROM directories WHERE depth = 1")
    changed_categories = []
    
    for cat_rel, stored_mtime in stored_categories:
        cat_full_path = os.path.join(omsi_root_path, cat_rel)
        if not os.path.exists(cat_full_path):
            changed_categories.append((cat_rel, "deleted"))
            continue
        current_mtime = int(os.path.getmtime(cat_full_path))
        if current_mtime != stored_mtime:
            changed_categories.append((cat_rel, "modified"))

    # If no top-level directories were modified or deleted, catalogue is 100% pristine!
    if not changed_categories:
        return StalenessResult.Pristine

    # Step 3: Tier 2 - Targeted Invalidation of Changed Subtrees
    # Only re-scan the specific category directories that were modified
    for cat_rel, change_type in changed_categories:
        reconcile_category_subtree(db_conn, omsi_root_path, cat_rel)
        
    return StalenessResult.Updated
```

### 7.5 Versioning Strategy
- **Pragma User Version / Meta Table:** The database tracks versioning via `meta(key = 'schema_version')` (initial value: `1`).
- **Forward-Only SQL Migrations:** Version migrations are managed through sequential migration scripts (`migrations/0001_initial.sql`, `0002_add_field.sql`).
- **Engine Handshake:** At connection time, the application verifies `schema_version`. If the catalogue version exceeds the application's supported version, it halts with a clear downgrade warning. If older, it executes pending migration scripts in a single transaction.

### 7.6 Multi-Root Plan (v2 Compatibility)
- **Zero Table Restructuring:** In v2, supporting multiple installation roots (e.g. secondary Steam libraries or standalone addon directories) requires no changes to existing column schemas.
- **Root Table Addition:**
  ```sql
  CREATE TABLE roots (
      root_id   INTEGER PRIMARY KEY AUTOINCREMENT,
      root_name TEXT NOT NULL UNIQUE,       -- e.g. 'Main OMSI', 'Development Sandbox'
      base_path TEXT NOT NULL UNIQUE        -- Absolute filesystem root
  );
  ```
- **Foreign Key Binding:** An optional column `root_id INTEGER DEFAULT 1 REFERENCES roots(root_id)` is added to `directories`. Because `rel_path` in `directories` and `files` is already stored relative to its enclosing root, existing queries and indexes continue to function without modification.

---

## 🧪 Stage 8 — Sample Queries

Below are 7 realistic, high-impact queries exercising the schema, covering primary gameplay, asset loading, reverse dependencies, and diagnostic health checks.

### Query 1: Populate Player Vehicle Selection Menu
- **Purpose:** Fast population of user-selectable buses and trams in the UI launcher.
- **SQL:**
  ```sql
  SELECT entity_id, name, vehicle_class, marker_file, vehicle_folder
  FROM v_drivable_vehicles
  ORDER BY name COLLATE NOCASE;
  ```
- **Result Shape:** `(entity_id: INTEGER, name: TEXT, vehicle_class: TEXT, marker_file: TEXT, vehicle_folder: TEXT)`
- **Accelerating Index:** Partial index `idx_entities_playable(category, is_playable, name COLLATE NOCASE)`. Executes in $< 1\text{ ms}$ across 2,500+ vehicle files.

### Query 2: Resolve Component Assets for Vehicle Startup
- **Purpose:** When loading a vehicle into the 3D simulation engine, retrieves all declared models, sound configs, scripts, and trailers.
- **SQL:**
  ```sql
  SELECT 
      r.role,
      r.raw_reference,
      f.rel_path AS resolved_path,
      f.size_bytes,
      r.is_resolved
  FROM entity_references r
  LEFT JOIN files f ON r.target_file_id = f.file_id
  WHERE r.source_entity_id = :vehicle_entity_id
  ORDER BY r.role, r.raw_reference;
  ```
- **Result Shape:** `(role: TEXT, raw_reference: TEXT, resolved_path: TEXT | NULL, size_bytes: INTEGER | NULL, is_resolved: INTEGER)`
- **Accelerating Index:** Covering index `idx_ref_source(source_entity_id, role)`.

### Query 3: $O(1)$ Direct File Path Resolver for Engine Loaders
- **Purpose:** High-throughput asset resolver called by map tile parsers (e.g. resolving `Splines\Ruede\rail_concrete_01.sli` or `Texture\envmap.bmp`).
- **SQL:**
  ```sql
  SELECT file_id, rel_path, size_bytes, mtime_sec, is_symlink
  FROM files
  WHERE rel_path = :normalized_path COLLATE NOCASE;
  ```
- **Result Shape:** `(file_id: INTEGER, rel_path: TEXT, size_bytes: INTEGER, mtime_sec: INTEGER, is_symlink: INTEGER)`
- **Accelerating Index:** Unique index `idx_files_rel_path_nocase(rel_path COLLATE NOCASE)`. Single B-tree lookup without table scan.

### Query 4: Enumerate Kinematic Trailers for Articulated Vehicles
- **Purpose:** Assembles multi-segment buses (e.g. `MAN_GN92_main.bus` $\to$ `MAN_GN92_trail.bus`) and double-articulated chains (`AGG300`).
- **SQL:**
  ```sql
  SELECT 
      r.role,
      r.raw_reference AS trailer_file,
      f.rel_path AS trailer_path,
      e_trail.name AS trailer_name
  FROM entity_references r
  JOIN files f ON r.target_file_id = f.file_id
  LEFT JOIN entities e_trail ON e_trail.marker_file_id = f.file_id
  WHERE r.source_entity_id = :tractor_entity_id
    AND r.role IN ('trailer_rear', 'trailer_front');
  ```
- **Result Shape:** `(role: TEXT, trailer_file: TEXT, trailer_path: TEXT, trailer_name: TEXT | NULL)`
- **Accelerating Index:** Index `idx_ref_source(source_entity_id, role)`.

### Query 5: Discover All Repaints (`.cti`) for a Vehicle
- **Purpose:** Enumerates repaint skin configuration files in the vehicle's texture subdirectories.
- **SQL:**
  ```sql
  SELECT f.file_id, f.file_name, f.rel_path, f.size_bytes
  FROM files f
  JOIN directories d ON f.dir_id = d.dir_id
  WHERE f.extension = 'cti'
    AND d.rel_path LIKE :vehicle_folder_path || '/%'
  ORDER BY f.file_name COLLATE NOCASE;
  ```
- **Result Shape:** `(file_id: INTEGER, file_name: TEXT, rel_path: TEXT, size_bytes: INTEGER)`
- **Accelerating Index:** Composite index `idx_files_ext_dir(extension, dir_id)`.

### Query 6: Diagnose Broken Dependencies on Map Load (Addon Doctor)
- **Purpose:** Pre-flight diagnostic tool checking for missing splines, objects, or vehicles before launching a simulation session.
- **SQL:**
  ```sql
  SELECT 
      source_category,
      source_entity_name,
      source_folder,
      role,
      raw_reference
  FROM v_missing_references
  WHERE source_category = 'map'
    AND source_folder = :map_folder;
  ```
- **Result Shape:** `(source_category: TEXT, source_entity_name: TEXT, source_folder: TEXT, role: TEXT, raw_reference: TEXT)`
- **Accelerating Index:** Partial index `idx_ref_unresolved(is_resolved) WHERE is_resolved = 0`. Filters directly over broken links without inspecting valid assets.

### Query 7: Reverse Dependency Check for Safe Addon Uninstallation
- **Purpose:** Mod manager validation: "Will deleting scenery object X break any existing maps or vehicles?"
- **SQL:**
  ```sql
  SELECT DISTINCT
      e.entity_id,
      e.category,
      e.name AS dependent_entity,
      r.role
  FROM entity_references r
  JOIN entities e ON r.source_entity_id = e.entity_id
  WHERE r.target_file_id = :target_asset_file_id
  ORDER BY e.category, e.name COLLATE NOCASE;
  ```
- **Result Shape:** `(entity_id: INTEGER, category: TEXT, dependent_entity: TEXT, role: TEXT)`
- **Accelerating Index:** Reverse index `idx_ref_target(target_file_id, role)`.

---

---

## ✅ Implementation Notes (2026-10-09)

Crate: `crates/opts-catalogue`. Game wiring: `crates/opts-game/src/catalogue_service.rs`. Database: `<config dir>/catalogue.db` (WAL), separate from `settings.db`.

**Measured on the local installation (release build, NVMe):** 622,298 files, 23,263 directories, 66,309 entities (2,537 vehicles, 42 maps, 49,802 scenery objects, 12,382 splines, …), 244,711 references (5,299 unresolved) — full scan **≈ 5.3 s** (walk 0.14 s, content detection 0.5 s, SQLite write ≈ 4 s). The database is ≈ 213 MB.

**Deviations from the proposal**

| Proposal | Implemented | Why |
| :--- | :--- | :--- |
| Staleness by directory mtimes (Tier 1/2, §7.4) | Staleness by **age** of `last_scan_completed_at` (default 24 h = `discovery_interval_days × 24 h`, the existing user setting), plus root change | A full scan takes ~4.5 s, so the incremental tiers are unnecessary; the age rule is what the product requires. Directory mtimes are still recorded for a future incremental mode. |
| Incremental reconcile | Whole snapshot rebuilt in **one transaction** | Readers keep the previous snapshot until commit; WAL is truncated afterwards. |
| `human` is a folder entity | `human` is a **file** entity (one per `.hum`) | A `Humans/<Pack>/` folder holds several independent characters. |
| (no display columns) | Tables `map_info` and `vehicle_info` | The New Game menus need name, description, tile count, depot, years, entry points and manufacturer/model/livery without parsing configs at runtime. |
| `weather` markers `.cfg/.txt/.dsc/.owt` | `.owt` only | The others are descriptions/locale files, not selectable weather profiles. |
| 16 categories incl. `input_map` `.kyb`/`.cfg` | `.kyb` only | `.cfg` files in `Inputs/` are device presets. |

**References implemented:** vehicles (`model`, `sound`, `script`, `constfile`, `trailer_rear`, `trailer_front`), maps (`ai_vehicle` from `ailists.cfg`), scenery objects (`model` for meshes and collision meshes, `texture` for `[matl]`/envmap/lightmap/nightmap/tree textures, `spline` for `[splinehelper]`, `script`, `constfile`, `sound`) and splines (`texture`). Textures resolve against the object's `texture/` folder, the object folder, then global `Texture/`, trying the other image extensions when the exact name is absent. On the local install this yields 244,711 references (5,299 unresolved, 2.2 %) and the full scan stays at ≈ 5.3 s.

**Query surface (`Catalogue`):** `maps`, `playable_vehicles`, `vehicle_marker`, `find_file` / `find_dir` (case-insensitive, `\` or `/`), `staleness`, `totals`, `last_scan`, `category_counts`. `opts_bevy::OmsiInstall::with_lookup` consults `find_file`/`find_dir` before touching the filesystem.

**Application behaviour:** on startup the menus are filled from the existing database immediately; if the catalogue is missing, built for another root, or older than the interval, discovery runs on a background thread and the lists are reloaded when it commits. A first-run session waits for the first scan. Settings → "Run Discovery Now" starts a rescan.

---

## ❓ Open Questions

| ID | Topic | Question | Impact | Recommendation |
| :--- | :--- | :--- | :--- | :--- |
| **OQ-01** | **Symlink Staleness** | On Linux host systems with 27,211 symlinked `.hof` files, changes to the central target depot do not update the symlink's own `lstat` mtime. How should cache invalidation treat target modifications? | Low | Staleness checking follows target timestamps during category/vehicle subtree re-scans rather than checking symlink metadata alone. |
| **OQ-02** | **Chrono Event Table** | Should map Chrono events (`maps/<Map>/Chrono/<Event>/Chrono.cfg`) be registered in `entities` (with `category = 'chrono_event'` and `alias_of_id` / parent ID) or in a dedicated `map_chrono_events` table? | Low | Modeling Chrono events as child entities in `entities` preserves the fixed 5-table design while enabling standard entity queries. |
| **OQ-03** | **Format Spec Reconciliation** | ~~When should the proposed patch to [`docs/src/formats/text/misc-types.md`](../formats/text/misc-types.md) (Investigation A4) be committed to reflect `.ovh` co-equality in the format specs?~~ | ✅ | Applied to `docs/src/formats/text/misc-types.md`: `.ovh` documented as co-equal format, `[type]` values enumerated, Q-MS01 resolved. |

---

## 📝 Provenance

- **Installation Inspected:** `/home/louisgoodnews/nvme2n1/steamapps/common/OMSI 2`
- **Directories Walked:** 23,318 directories across 35 top-level category trees.
- **Files Inspected:** 622,305 files in scope (214.5 GiB footprint).
- **Techniques Used:**
  - Filesystem directory tree traversal and metadata census.
  - Header sampling and keyword parsing of text configs (`.bus`, `.ovh`, `.cfg`, `.sco`, `.sli`, `.zug`, `.otp`, `.hum`, `.osn`).
  - Cross-mode relationship census across AI lists, train consists, model configs, and map tiles.
  - Encoding-aware inspection (UTF-16LE with BOM vs Windows-1252 / UTF-8).
  - Symlink enumeration and resolution audit.

**Clean-room attestation (ADR-0002):**
- ✅ No OMSI binaries (`.exe`, `.dll`, `.so`) were disassembled, decompiled, or inspected.
- ✅ No GPL source code was read or referenced.
- ✅ No OMSI assets or content files were copied or committed to the repository.
- ✅ Zero Rust implementation code written during the schema design phase.

---

## 🔗 Related Documents

- [Format Inventory](../formats/inventory.md)
- [Miscellaneous Text Formats (`.ovh`, `.zug`, `.hum`)](../formats/text/misc-types.md)
- [Vehicle Bus Definition Specification](../formats/text/vehicles/bus.md)
- [Map AI Lists & Train Consists](../formats/map/lists.md)
- [Map Chrono Overlays & Variants](../formats/map/chrono.md)
- [Architecture Decision Record: ADR-0002 Clean-Room Policy](../adr/0002-clean-room-policy.md)
