# Per-Map Configuration Profiles and Load Profiler (design)

| Metadata | Value |
| :--- | :--- |
| **Milestone** | [M-0130](../roadmap/phase-9/M-0130.md), phase A (design) |
| **Status** | 📝 Proposed 2026-10-19 — awaiting the project lead's approval (checkpoint 1); no code written |
| **Crates** | `opts-game` (profiles, lifecycle, modal), `opts-core` (static profiler's reader), `opts-catalogue` (cached static profile) |
| **Related** | [Settings registry](settings-registry.md), [Settings reference](../reference/settings.md), [Home and shell](main-menu.md), [Chrono](../formats/map/chrono.md), [Map lists](../formats/map/lists.md), [Map tiles](../formats/map/tiles.md), [Lighting system](lighting-system.md) |

## 0. What the code and the data say (read first)

Five findings changed the design relative to the task text. Each is measured or read from the tree, not assumed.

1. **"Map selection" is the *Drive* wizard** (`new_game_screen`: *Where / What / When / Review*), not a separate menu. The modal belongs on the *Where* step (and is reachable from *Review*).
2. **Chrono is date-driven, not a list of year folders.** A map's `Chrono/` holds dated overlays; 18 maps have one, together 455 entries, some nested per match day (`Ahlheim 5/Chrono/1_FCA Heimspiele/<date>/`). Which entry is active follows from the session's *map date* (`MapDateChanged`, `ChronoRebuild`). So `chrono_id` cannot be a user-typed "1990"/"2020"; it is the **resolved entry** for the chosen date (§3.3).
3. **Settings are declared once as descriptors and stored as strings** (`SettingDescriptor`, `settings.db`, diff-saving against a baseline). A profile must ride on that, not on a second hand-written struct that drifts from it (§1).
4. **Applying settings is not one moment.** Some keys apply *at once*, some *after restart* (`graphics.water`), some are read when the map loads (`gameplay.traffic`, `gameplay.tile_radius`, `gameplay.load_all_tiles`). Profile values must therefore be applied **before the drive loads**, i.e. at *Review → Start*, not when the map finishes (§3).
5. **Static profiling cannot always meet 100 ms.** Measured on this installation (warm file cache, byte scan of the UTF-16 tiles, `[object]` and `[spline]` records): 47 maps in 7.7 s in total; median 0.05 s per map, but `Ruhrgebiet` (2,156 tiles, 197 MB) 1.07 s, `Gladbeck` 0.81 s, `Ahlheim 5` (776 tiles, 174 MB, 354,958 objects) 0.35 s warm and more cold; two maps (`Neuendorf`, `Winsenburg`) are not UTF-16 and need the other encoding paths. So the scan runs **in the background during the catalogue scan and is cached**; the modal reads the cache in microseconds, and the 100 ms budget applies to the cached read and to scanning one *sample* of tiles (§4.2).

## 1. A1 — Scope: which settings may differ per map

Every key of the generated [settings reference](../reference/settings.md) is sorted into three buckets. The rule: a key may be map-profile eligible only if it changes **performance or the simulated content of a map** and has no meaning that belongs to the person or the machine.

| Bucket | Keys |
| :--- | :--- |
| **Always global** (person or machine) | all of *Interface* (`theme_mode`, `auto_*`, `high_contrast`, `active_theme`, `theme_file`, `ui_scale`, `density`, `reduced_motion`, `interface.*`, all `hud.*`); all *Controls* (`controls.*`, key bindings); all *Audio*; all *Content* (`omsi_path`, scan keys, `recent_map_path`); `graphics.window_mode`, `graphics.resolution`, `graphics.vsync`, `graphics.hdr` (they describe the display and the GPU, and apply the same on every map); `look`, `graphics.default_fov` (taste, not load); `gameplay.autosave_minutes`, `gameplay.time_scale` (habits); all *Advanced* (`advanced.*`, developer tools, log level, hardware) |
| **Map-profile eligible** | `graphics.view_distance` (declared, not available yet, M-0029: the profile reserves the key); `graphics.water`; `graphics.weather_fog`, `weather_clouds`, `weather_precipitation`, `weather_wet_ground`; `graphics.light_quality_mode`, `light_shafts`, `light_night_shafts`, `light_lantern_cones`, `light_headlight_cones`, `light_cone_cap`, `light_cone_depth_fade`, `light_governor_target_fps`; `graphics.mirror_count`, `mirror_rate`; `graphics.glass_reflections`, `graphics.bloom`; `texture_filtering`; `gameplay.traffic`, `gameplay.pedestrians`; `gameplay.load_all_tiles`, `gameplay.tile_radius`; `advanced.streaming_budget_ms` |
| **Map-locked** (never in a profile) | the fixed simulation steps (the fixed-update rate of the vehicle physics, the 0.1 s traffic sub-step, the script tick): they define the simulation and must not differ between maps; the entry point, vehicle, repaint and date (they are the drive, not a setting) |

The task's example "reflection resolution" does not exist as a setting yet; the nearest keys are `graphics.glass_reflections` and the mirror pair. A future setting joins a bucket by declaring it, so the table above is generated from the registry, not kept by hand.

### 1.1 The data model

The registry is the schema, so the profile declares its scope there:

```rust
/// Added to `SettingDescriptor` (default `Global`, so nothing changes until a key opts in).
pub enum ProfileScope { Global, Map, Locked }

/// One saved profile: only keys whose descriptor says `ProfileScope::Map`.
pub struct MapProfile {
    pub schema: u32,                          // 1
    pub name: String,                         // shown in the dropdown
    pub created: String,                      // RFC 3339, informational
    pub based_on: Option<String>,             // preset it was created from ("performance", ...)
    pub settings: BTreeMap<String, String>,   // key -> stored text, as SettingKind::to_text writes it
}
```

*Why a map of strings rather than 22 typed fields:* a new eligible setting costs one descriptor line and appears in the profile, the file, the validation and the docs without a second edit (the registry's own goal). Loading **validates every value with the descriptor** (type, range, enum) and drops, with a warning, a key that is unknown, no longer `Map` scope or out of range; the profile still loads. A typed read-only view for code that wants fields (`MapProfileView`) can be generated later.

Other types: `MapKey { map_id: String, chrono_id: String }` (§3.3) and `ProfileManager` (resource: discovery, load, save, delete, resolve with fallback, apply and restore, §3).

**Built-in presets** (not files, shipped in code): *Performance*, *Balanced*, *Quality*, each a `MapProfile` over the eligible keys; the load profiler recommends one by name (§4.3). They are the "Performance profile" of the task's heuristic.

## 2. A2 — File format and storage

**TOML**, in the user data directory beside `drive_history.json` and `viewpoints.json` (`DriveHistory::path()`'s folder; on Linux `~/.local/share/opts/`):

```
profiles/
  <map_id>/
    default.toml          # profile for the map without an active Chrono entry
    <chrono_id>.toml      # profile for one Chrono entry (slug of its relative path)
    metrics.toml          # dynamic profile: recent sessions per Chrono key
```

```toml
# profiles/Berlin-Spandau/default.toml
schema = 1
name = "Spandau, smooth"
created = "2026-10-19T12:00:00Z"
based_on = "balanced"

[settings]
"graphics.light_shafts" = "low"
"graphics.mirror_count" = "1"
"gameplay.traffic" = "60"
"gameplay.tile_radius" = "3"
```

- **`map_id`** is the map's folder name made filesystem-safe (percent-encoding of anything outside `[A-Za-z0-9._ -]`); two maps with the same folder name in different places are one map for this purpose (the catalogue treats them as one map too). `chrono_id` is `default` or the same encoding of the Chrono entry's path relative to `Chrono/` (nesting becomes `__`).
- **One profile per key.** The task's layout holds one file per `(map, chrono)`; the dropdown (§5) lists this key's profile, the map's other keys' profiles and the presets. *Apply Selected* uses the chosen one for the session; *Create New from Current* writes this key's file.
- **Writing** is atomic (temp file in the same folder, then rename). A file that does not parse is moved aside as `*.toml.bad` and the key falls back (the same rule as `viewpoints.json`). A `schema` newer than the build's is read as far as it is understood and never rewritten.
- **`metrics.toml`** keeps the **last five** sessions per Chrono key (never an unbounded log): average and 95th-percentile frame time in ms, seconds played, the vehicle and the `settings_hash` (a hash of the eligible keys as they were), date. The advice uses the newest session of at least 60 s. A different `settings_hash` is shown ("measured with other settings"), not hidden. Metrics live beside the profiles, not in a central database: deleting `profiles/<map>/` forgets everything about a map, and no database lock is needed for a once-per-session write.
- The files are never written into the OMSI folder, and never into the global settings.

## 3. A3 — Lifecycle and fallback

### 3.1 Resolution (the fallback chain)

For the chosen map and date: `{map_id}/{chrono_id}.toml` → `{map_id}/default.toml` → global settings. The modal states which one resolved ("Profile for this Chrono entry", "Profile for the map", "No profile: your global settings"), so a silent fallback never looks like a match. A Chrono entry without its own file therefore uses the map's default profile.

### 3.2 When things happen

| Moment | What happens |
| :--- | :--- |
| **Map chosen** (*Where* step) | `ProfileManager::resolve(map_id, chrono_id)` (reads one small file); the modal's profiler feedback reads the cached static profile and `metrics.toml`. Nothing is applied. |
| **Date changed** (*When* step) | the resolved Chrono entry may change, so the key and the shown profile are resolved again. |
| **Start** (*Review → Start*, before `AppState::InGame`) | the resolved profile (or the one picked in the modal) is **applied**: a snapshot of the eligible global values is taken, the profile's values are set on `GameSettings` in memory. This is the only point early enough for `Apply::OnRestart` keys and for values the load reads. |
| **Map finished loading** | nothing is applied again; the dynamic profiler starts counting (§4.4). If the profile contained a key that could not apply live (a restart-only key whose value already differs), the session shows a one-time notice naming it. |
| **During the drive** | a user change to an overlaid key edits the *session* value; the settings screen marks overlaid rows ("from the profile of this map") and offers *Save to profile*. |
| **Return to the main menu / Home** (leaving `InGame`) and **on quit** | the metrics of the session are written; the snapshot is restored; the profile is released. |

### 3.3 Non-destructive application

`GameSettings` saves by diffing against a baseline, so merely overwriting its fields would persist profile values on the next `save()`. The design is therefore: **the overlay is applied to the in-memory `GameSettings`, but `save()` writes the snapshot's value for every overlaid key** (and a user edit of an overlaid key becomes a session edit, never a global one, unless the user chooses *Save to profile*). Consequences: a crash cannot leave profile values in the global file (the file never saw them), and restoring is a copy of the snapshot back. A test per rule: apply → save → the file holds the global values; apply → restore → equality with the snapshot.

### 3.4 What `chrono_id` is

The resolved Chrono entry for the session's map date (rule of [chrono](../formats/map/chrono.md): the entries are dated overlays; nested entries are entries at any depth). The modal shows "Chrono entry for this date: …" next to the profile. Because some maps have an entry *per match day*, **a profile is never created per Chrono entry automatically**: the default for *Create New from Current* is the map's `default.toml`, and a Chrono-specific file is an explicit choice offered only when an entry is active. `chrono_id` is `default` when no entry covers the date.

## 4. A4 — The load profiler

### 4.1 `MapLoadProfile`

```rust
pub struct MapLoadProfile {
    pub tile_count: usize,
    pub object_count: usize,          // [object] records, all tiles
    pub spline_count: usize,          // [spline] and [spline_h] records
    pub ai_pool_vehicles: usize,      // distinct vehicle entries in the map's AI lists
    pub map_size_km2: f32,            // tile_count * 0.09 (tiles are 300 m x 300 m)
    pub estimated: bool,              // counted from a sample of tiles
    pub avg_frame_time_ms: Option<f32>,   // last session of this key, from metrics.toml
    pub p95_frame_time_ms: Option<f32>,
}
```

**`ai_vehicle_count` of the task is renamed `ai_pool_vehicles`**: the AI list files (`ailists.cfg`) name the vehicles that *may* appear and their shares, not how many appear; the number on the road is capped by `gameplay.traffic` (default 100). The figure is the size of the pool, and the heuristic for AI load looks at pool size together with the traffic setting (§4.3).

### 4.2 Static profiler

- **Reader** (`opts-core`): a function over the bytes of a tile that counts the records `[object]`, `[spline]`, `[spline_h]` without building the tile (no strings kept, no 3D data), for UTF-16LE with BOM (13,151 of 13,938 tiles), and for UTF-8/CP1252 with the same keyword match after BOM detection (the two maps that count 0 above are of that kind and are part of the test corpus).
- **When it runs:** in the background, inside the catalogue scan (it already visits the maps and stores `tile_count`); the three counts are stored as catalogue columns. Opening the modal is a cached read. For a map the scan has not reached, or whose files changed (mtime), the modal shows *Estimating…* and the answer arrives through the same background task; the main thread never touches the tile files ([M-0088](../roadmap/phase-9/M-0088.md)).
- **The budget:** measured warm, a full count takes 0.01 s (small maps) to 1.07 s (`Ruhrgebiet`); the median map is 0.05 s. A map over 100 ms is counted from a **sample** (every k-th tile, k chosen to bound the work, scaled by the tile count and marked *estimated*); the acceptance test checks that the sample estimate of the five largest maps is within 10 % of the full count. Counting in the catalogue scan removes the cost from the user's path entirely; the 100 ms criterion is for the cached read and for the sample.

### 4.3 The heuristic engine

A pure function `advise(&MapLoadProfile, &Settings) -> Vec<Advice>`; each `Advice { id, severity, message_key, suggested: Vec<(key, value)> }` carries message keys (de/en) and the settings change it suggests, so the modal can offer *Apply suggestion* and tests assert on `id`s.

The task's thresholds are the starting point; the corpus calibrates them. Object counts over the 47 maps of this installation: median 40,985, 75th percentile 100,107, 90th percentile 242,118, largest 354,958 (`Ahlheim 5`); smallest `Grundorf` 1,095. A single "over 50,000" line would flag about 45 % of all maps as heavy, so the engine uses **tiers** with the task's 50,000 as the first note:

| Metric | Rule | Advice |
| :--- | :--- | :--- |
| `object_count` | > 50,000 | note: *geometry load is above the typical map*; suggests lower view distance (when available) and shadow-related lighting |
| | > 150,000 | *High geometry load*: suggests `graphics.light_shafts` low, `graphics.light_cone_cap` lower, `gameplay.tile_radius` smaller, `graphics.mirror_count` 1 |
| | < 20,000 | *Light map*: suggests the *Quality* preset |
| `spline_count` | > 40,000 | road-geometry note (`St-Servan`, `Ruhrgebiet` class) |
| `ai_pool_vehicles` × traffic | pool > 200 and `gameplay.traffic` > 100 | *High AI load*: suggests a lower `gameplay.traffic` |
| `avg_frame_time_ms` | > 33.0 | *This map ran slowly last time (X ms average)*: suggests the *Performance* preset |
| | ≤ 16.7 with the latest settings | *Ran smoothly*: suggests raising one quality step |
| missing metrics | none | no timing advice, a line *Not played yet with these settings* |

Thresholds are named constants in one module with tests, not map names ("do not hardcode map names or object counts" is met by deriving them from the corpus percentiles and documenting how; they are re-derived when the corpus changes). Frame times depend on the computer, so advice from metrics uses only this computer's `metrics.toml`.

### 4.4 Dynamic profiler

A system in `InGame` that accumulates the frame time (the same measurement the lighting governor already makes, see [lighting §9](lighting-system.md); one source, not two clocks) as a running mean and a coarse histogram for the 95th percentile, **excluding** the loading phase, pauses and the first 10 s after load (shader and tile warm-up). On leaving `InGame` it writes one entry to `metrics.toml` (atomic, off the main thread). Sessions under 60 s are discarded. Nothing is sent anywhere.

## 5. A5 — The modal

Reached from a *Map configuration* button on the *Where* step (and from *Review*); `opts-gui` `ModalBuilder`, same page conventions as the vehicle menu.

```
Map Configuration — Berlin-Spandau                    [Chrono entry for this date: none]
┌ Load profile ────────────────────────────────────────────────────────────────────────┐
│ 29,450 objects · 2,486 splines · 329 tiles · 29.6 km² · AI pool 178 vehicles          │
│ Last session on this computer: 21 ms average (p95 34 ms), 12 min, 2026-10-18          │
│ ▸ Typical geometry load.   ▸ Ran smoothly: you may raise one quality step. [Apply]    │
└──────────────────────────────────────────────────────────────────────────────────────┘
Profile  [ ▾ This map: "Spandau, smooth" ]   Resolved: profile for the map
         presets: Performance · Balanced · Quality   |  other keys of this map
[Apply Selected]  [Create New from Current]  [Delete]  [Cancel]
```

- *Apply Selected*: use the chosen profile for the next drive (applied at *Start*, §3.2); a checkbox *Make it this map's profile* writes the file.
- *Create New from Current*: asks for a name, writes `default.toml` (or the Chrono file when an entry is active and the user picks it) from the **current global values of the eligible keys**.
- *Delete* asks first (safe choice first, Esc cancels) and is disabled for presets.
- Texts come as message keys in English and German; every control has a `UiId` and a control-protocol equivalent (`profiles.list`, `profiles.get`, `profiles.apply`, `profiles.save`, `profiles.delete`, `profiles.advice`), so the lifecycle tests run without a window.
- While the static profile is being counted the section shows *Estimating…* and fills in; it never blocks.

## 6. Tests planned (phase C)

Profile discovery, load, validation (unknown, out-of-range and scope-changed keys), the fallback chain (Chrono → map → global) with an overlay of two Chrono entries; apply → save → global file unchanged; apply → restore → snapshot equality; atomic write and `.bad` handling; the reader on a UTF-16 fixture, a CP1252 fixture and a nested-record fixture, and against the full count on three real maps (install-gated); the sample estimate within 10 % on the five largest maps; `advise` for each row of §4.3; metrics ring of five; the 60 s discard rule; the modal through the control protocol.

## 7. Decisions for the project lead

1. **Profile data model:** a string map validated by the registry (recommended) or a typed struct of the 22 keys?
2. **Chrono key:** the resolved Chrono entry with an explicit, opt-in Chrono-specific profile (recommended) or a profile per entry automatically?
3. **Where the static counts live:** cached in the catalogue by the background scan (recommended) or computed when the modal opens (cannot meet 100 ms on the largest maps)?
4. **`ai_vehicle_count`:** accept `ai_pool_vehicles` (pool size) plus the traffic setting?
5. **Thresholds:** the tiered table of §4.3 (calibrated on the 47 installed maps) instead of one 50,000 line?
6. **Overlaid rows in the settings screen:** a user edit of an overlaid key stays in the session unless *Save to profile* is chosen (recommended).
