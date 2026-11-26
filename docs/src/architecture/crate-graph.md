# 📦 Crate Architecture & Dependency Graph

This document defines the architectural layers, crate boundaries, and dependency relationships across the OPTS workspace per [ADR-0005](../adr/0005-crate-architecture.md).

---

## 🏛️ Architecture Layers

The OPTS workspace enforces a strict, unidirectional 4-tier layering model. No upward dependencies or dependency cycles are permitted.

```mermaid
graph TD
    classDef foundation fill:#2d3748,stroke:#4a5568,stroke-width:2px,color:#fff;
    classDef domain fill:#1a365d,stroke:#2b6cb0,stroke-width:2px,color:#fff;
    classDef bridge fill:#234e52,stroke:#319795,stroke-width:2px,color:#fff;
    classDef app fill:#44337a,stroke:#805ad5,stroke-width:2px,color:#fff;

    %% Layer 0: Foundations
    subgraph L0 ["Layer 0 — Foundations & Diagnostics"]
        opts_debounce["opts-debounce<br/>• Trailing debounce of file-change events<br/>• No dependencies · Zero Bevy"]:::foundation
        opts_map_advice["opts-map-advice<br/>• Map load profile and advice heuristics<br/>• No dependencies · Zero Bevy"]:::foundation
        opts_logging["opts-logging<br/>• Non-blocking tracing writers<br/>• Subsystem file routing<br/>• Panic hook & crash telemetry"]:::foundation
        opts_cli["opts-cli<br/>• Control-protocol types<br/>• Command-line client<br/>• Zero Bevy"]:::foundation
        opts_hardware["opts-hardware<br/>• Privacy-respecting hardware probe<br/>• Default recommendations<br/>• No network, no identifiers"]:::foundation
        opts_shell_icons["opts-shell-icons<br/>• Material Icons codepoints of the shell<br/>• No dependencies · Zero Bevy"]:::foundation
        opts_collision["opts-collision<br/>• Oriented-box contact (separating axes)<br/>• Velocity response & spatial grid<br/>• Zero Bevy · Zero File I/O"]:::foundation
        opts_hud_keys["opts-hud-keys<br/>• HUD message key tables<br/>• No dependencies · Zero Bevy"]:::foundation
        opts_shell_keys["opts-shell-keys<br/>• Shell message key tables<br/>• Depends on opts-shell-keys-help · Zero Bevy"]:::foundation
        opts_shell_keys_help["opts-shell-keys-help<br/>• Help overlay message keys and the keys! macro<br/>• No dependencies · Zero Bevy"]:::foundation
        opts_minimap_zoom_canvas["opts-minimap-zoom-canvas<br/>• Mini map speed zoom filter<br/>• RGBA8 pixel canvas<br/>• No dependencies · Zero Bevy"]:::foundation
        opts_save_error["opts-save-error<br/>• Save container error type<br/>• No dependencies · Zero Bevy"]:::foundation
        opts_load_problem["opts-load-problem<br/>• Load problem type and severity table<br/>• No dependencies · Zero Bevy"]:::foundation
        opts_vehicle_gears["opts-vehicle-gears<br/>• AI engine RPM and gear model<br/>• No dependencies · Zero Bevy"]:::foundation
    end

    %% Layer 1: Domain Core & Parsers
    subgraph L1 ["Layer 1 — Shared Types, Native Model, Legacy Readers"]
        opts_types["opts-types<br/>• Shared geometry & tile-grid maths<br/>• No file formats<br/>• Zero Bevy"]:::domain
        opts_l10n["opts-l10n<br/>• Fluent-subset parser, language packs<br/>• Checks & JSON interchange<br/>• No OMSI content"]:::domain
        opts_omsi["opts-omsi (facade)<br/>• Re-exports the OMSI reader crates at the old paths<br/>• Leaf adapter · Zero Bevy · Zero File I/O"]:::domain
        opts_omsi_binary["opts-omsi-binary<br/>• OMSI binary mesh readers: .o3d and .x<br/>• Parsers over byte slices<br/>• No Bevy · Zero File I/O"]:::domain
        opts_omsi_vehicles["opts-omsi-vehicles<br/>• OMSI vehicle readers: .bus, model.cfg, paths.cfg, .hof<br/>• Setvar catalogue and light reader<br/>• No Bevy · Zero File I/O"]:::domain
        opts_omsi_map["opts-omsi-map<br/>• OMSI map readers: global.cfg, tiles, terrain, water<br/>• Chronology overlay and map writers<br/>• No Bevy · Zero File I/O"]:::domain
        opts_omsi_scenery["opts-omsi-scenery<br/>• OMSI scenery readers: .sco, .sli, crossings<br/>• Train consist reader<br/>• No Bevy · Zero File I/O"]:::domain
        opts_omsi_text["opts-omsi-text<br/>• OMSI text readers: keyword blocks, .cfg, .cti, .oft, .org<br/>• Timetables, weather, sound lists<br/>• No Bevy · Zero File I/O"]:::domain
        opts_omsi_scan["opts-omsi-scan<br/>• OMSI keyword-block scanner and text decoding<br/>• Positional value parsers<br/>• No Bevy · Zero File I/O"]:::domain
        opts_omsi_timetable["opts-omsi-timetable<br/>• OMSI timetable readers: Busstops.cfg, .ttp, .ttl, .ttr<br/>• Timetable query model and interpolation<br/>• No Bevy · Zero File I/O (folder loader excepted)"]:::domain
        opts_omsi_weather["opts-omsi-weather<br/>• OMSI weather readers: .owt presets, clouds.cfg, .dsc names<br/>• Weather folder loader<br/>• No Bevy · Zero File I/O (folder loader excepted)"]:::domain
        opts_wav["opts-wav<br/>• RIFF/WAVE decoder (PCM 8/16/24, float 32)<br/>• Samples to interleaved f32<br/>• No dependencies · Zero Bevy"]:::domain
        opts_vdf["opts-vdf<br/>• Valve KeyValues (VDF) parser<br/>• Steam library and app manifest text<br/>• No dependencies · Zero Bevy"]:::domain
        opts_steam["opts-steam<br/>• Steam library and OMSI install discovery<br/>• Reads libraryfolders.vdf and app manifests<br/>• No Bevy"]:::domain
        opts_position_tag["opts-position-tag<br/>• Player position text for log messages<br/>• Process-wide tile and world coordinates<br/>• No dependencies · Zero Bevy"]:::domain
        opts_model["opts-model<br/>• Native content model & TOML form<br/>• ContentSource trait<br/>• No OMSI knowledge"]:::domain
        opts_challenge_model["opts-challenge-model<br/>• Challenge rules, challenge, run events and result<br/>• No dependencies · Zero Bevy · Zero File I/O"]:::domain
        opts_importer["opts-importer<br/>• opts-omsi to opts-model<br/>• The only crate that knows both"]:::domain
        opts_core["opts-core (temporary facade)<br/>• Re-exports opts-omsi and opts-l10n at the old paths<br/>• Removed when the last consumer is migrated"]:::domain
    end

    %% Layer 2: Domain Simulation & Engine Bridges
    subgraph L2 ["Layer 2 — Simulation & Engine Presentation Bridges"]
        opts_sim["opts-sim<br/>• Vehicle physics & kinematics<br/>• Drivetrain, suspension, steering<br/>• Script VM bytecode interpreter<br/>• Zero Bevy · Zero File I/O"]:::bridge
        opts_sim_passengers["opts-sim-passengers<br/>• Passenger model: boarding, tickets, seats<br/>• Standing places and getting off<br/>• Zero Bevy · Zero File I/O"]:::bridge
        opts_sim_passenger_model["opts-sim-passenger-model<br/>• Passenger data: cabin, paths, tickets<br/>• Path graph of a cabin and the random source<br/>• No dependencies · Zero Bevy · Zero File I/O"]:::bridge
        opts_sim_pedestrian["opts-sim-pedestrian<br/>• Footpath lanes & lane network<br/>• Walkers and gait (ISS-066, stage 2)<br/>• Zero Bevy · Zero File I/O"]:::domain
        opts_traffic["opts-traffic<br/>• Lane graph & car following<br/>• Conflict zones, right of way<br/>• Zero Bevy"]:::bridge
        opts_car_following["opts-car-following<br/>• Intelligent Driver Model<br/>• Vehicle classes & driver parameters<br/>• Zero Bevy · Zero File I/O"]:::bridge
        opts_signals["opts-signals<br/>• Traffic-light programs<br/>• Aspect shown to each vehicle<br/>• Zero Bevy · Zero File I/O"]:::bridge
        opts_scripted_drive["opts-scripted-drive<br/>• Pedals from the desired acceleration<br/>• Script torque back to acceleration<br/>• Zero Bevy · Zero File I/O"]:::bridge
        opts_scene_trace["opts-scene-trace<br/>• Scene trace records and store<br/>• Bevy types, no OMSI"]:::bridge
        opts_spatial_zones["opts-spatial-zones<br/>• Ceilings and tunnels<br/>• Sampling, tracking, overlays"]:::bridge
        opts_spatial_zones_core["opts-spatial-zones-core<br/>• Hysteresis tracker and 10 Hz sample gate<br/>• No dependencies · Zero Bevy · Zero File I/O"]:::bridge
        opts_stream_budget["opts-stream-budget<br/>• Tile-streaming frame budget<br/>• Wanted and evictable tiles<br/>• Zero Bevy · Zero File I/O"]:::bridge
        opts_traffic_weather["opts-traffic-weather<br/>• Grip and fog for AI drivers<br/>• Car-following parameter rules<br/>• Zero Bevy · Zero File I/O"]:::bridge
        opts_vehicle_traits["opts-vehicle-traits<br/>• Traffic traits of a vehicle type<br/>• Mapped onto the driver parameters<br/>• Zero Bevy · Zero File I/O"]:::bridge
        opts_kinematic["opts-kinematic<br/>• Cheap kinematic acceleration (M-0103)<br/>• Drive torque curve over speed<br/>• Zero Bevy · Zero File I/O"]:::bridge
        opts_challenge["opts-challenge (facade)<br/>• Re-exports the challenge model and the scorer at the old paths<br/>• Zero Bevy"]:::bridge
        opts_challenge_scorer["opts-challenge-scorer<br/>• Applies run events to a challenge<br/>• Produces the challenge result<br/>• Zero Bevy · Zero File I/O"]:::bridge
        opts_audio["opts-audio<br/>• Sound engine: clips, emitters, voice budget<br/>• Distance fade and mix buses<br/>• Bevy types, no file formats"]:::bridge
        opts_audio_curves["opts-audio-curves<br/>• Gain curves: distance fade, dB, percent, semitones<br/>• No dependencies · Zero Bevy · Zero File I/O"]:::bridge
        opts_light_quality["opts-light-quality<br/>• Effective lighting quality and fallback ladder<br/>• Automatic frame-time governor<br/>• Bevy types"]:::bridge
        opts_light_quality_core["opts-light-quality-core<br/>• Shafts, light mode and bloom levels<br/>• Names and parsers<br/>• Zero Bevy · Zero File I/O"]:::bridge
        opts_main_camera["opts-main-camera<br/>• MainCamera marker (the player's view)<br/>• Bevy types"]:::bridge
        opts_catalogue["opts-catalogue<br/>• Installation scan into a catalogue<br/>• Map & vehicle lists"]:::bridge
        opts_gui["opts-gui<br/>• Material Design 3 tokens<br/>• Reusable UI widgets<br/>• Glassmorphism layout<br/>• Bevy UI integration"]:::bridge
        opts_theme_files["opts-theme-files<br/>• RON theme files in the config folder<br/>• Discovery and contrast-checked loading<br/>• No Bevy"]:::bridge
        opts_bevy["opts-bevy<br/>• 3D mesh loaders & materials<br/>• Coordinate system mapping<br/>• ECS VehicleController bridge<br/>• Scenery object spawning"]:::bridge
        opts_bevy_bus_stops["opts-bevy-bus-stops<br/>• Bus stop marker component of the map<br/>• Bevy-facing leaf"]:::bridge
        opts_bevy_ground["opts-bevy-ground<br/>• Ground texture CPU images and BC decoding<br/>• Tile ground composition<br/>• Bevy-facing leaf"]:::bridge
        opts_bevy_instancing["opts-bevy-instancing<br/>• Mesh instancing layers (dedupe, static merge)<br/>• render.stats diagnostics<br/>• Bevy-facing leaf"]:::bridge
        opts_bevy_moon_material["opts-bevy-moon-material<br/>• Moon disc material and its shader<br/>• Bevy-facing leaf"]:::bridge
        opts_bevy_season["opts-bevy-season<br/>• Season state and snow cover folders<br/>• No dependencies · Zero Bevy"]:::bridge
        opts_bevy_texture_sentinel["opts-bevy-texture-sentinel<br/>• Benchmark-only missing-texture sentinel<br/>• Bevy-facing leaf"]:::bridge
        opts_road_surf["opts-road-surf<br/>• Road-texture unevenness: .surf height maps and texture strips<br/>• Synthetic roughness per surface type<br/>• No Bevy"]:::bridge
        opts_world_stats["opts-world-stats<br/>• World counters, positions and camera setup<br/>• Bevy types, no I/O"]:::bridge
        opts_world_load["opts-world-load<br/>• The in-progress world load: WorldLoadJob and its closure (ISS-064)<br/>• Bevy systems over the split crates; no opts-game dependency"]:::bridge
        opts_vehicle_script_texture["opts-vehicle-script-texture<br/>• Script texture canvases and colour maps<br/>• Bevy types, no I/O"]:::bridge
    end

    %% Layer 3: Application Executables & Tools
    subgraph L3 ["Layer 3 — Application Executables & Tooling"]
        opts_exhaust["opts-exhaust<br/>• Exhaust plumes of the vehicles nearest the camera<br/>• Puff emission, outlets, distance fade<br/>• Bevy systems over opts-bevy and opts-sim"]:::app
        opts_game_state["opts-game-state<br/>• App states, settings types & defaults<br/>• In-game component and resource types<br/>• Bevy-facing type crate: no systems, no I/O"]:::app
        opts_fleet_number["opts-fleet-number<br/>• Vehicle fleet number from the .org registration list<br/>• Bevy systems over opts-bevy types"]:::app
        opts_mesh_dir["opts-mesh-dir<br/>• Base folder of a vehicle's mesh paths<br/>• Config folder or model folder, by resolved meshes"]:::app
        opts_world_session["opts-world-session<br/>• Resolved content paths and the menu request for a world session"]:::app
        opts_shell_calendar["opts-shell-calendar<br/>• Date and time steppers and choice chips<br/>• Bevy UI over opts-bevy and opts-gui"]:::app
        opts_shell_dialog["opts-shell-dialog<br/>• Confirmation dialogs and their answer systems<br/>• Bevy UI over opts-gui"]:::app
        opts_shell_nav["opts-shell-nav<br/>• Screen registry, rail frame, back stack and Esc<br/>• Banners and the shell content area<br/>• Bevy UI over opts-bevy, opts-gui and opts-game-state"]:::app
        opts_notification["opts-notification<br/>• Toasts, and errors with their durable place<br/>• Refused-navigation toast system<br/>• Bevy UI over opts-gui, opts-shell-nav and opts-shell-keys"]:::app
        opts_settings_io["opts-settings-io<br/>• settings.toml and settings.db file I/O<br/>• Background settings writer (one thread, newest copy per file)<br/>• Free functions over opts-game-state types"]:::app
        opts_attachments["opts-attachments<br/>• Spline attachment placements along splines<br/>• Map-wide spline index<br/>• Zero file I/O · Bevy transforms over opts-bevy"]:::app
        opts_tile_loading["opts-tile-loading<br/>• Tile load-all and radius decisions (M-0080)<br/>• Streaming budget and residency as pure functions<br/>• Pure Rust over opts-hardware, opts-stream-budget and opts-game-state"]:::app
        opts_new_game_types["opts-new-game-types<br/>• Plan-a-drive catalogue and choice types<br/>• Trip choice and planner date arithmetic<br/>• Bevy derives over opts-bevy and opts-shell-calendar"]:::app
        opts_repaints["opts-repaints<br/>• Vehicle repaint discovery for the new-game vehicle tab<br/>• .bus, model.cfg and .cti chain as pure file reads<br/>• Over opts-bevy, opts-core, opts-new-game-types and opts-steam"]:::app
        opts_app_args["opts-app-args<br/>• AppArgs: the parsed command line as a Bevy resource<br/>• Bevy resource over opts-cli"]:::app
        opts_catalogue_service["opts-catalogue-service<br/>• Startup of the SQLite catalogue and menu lists<br/>• Background discovery thread and selection reconciliation<br/>• Bevy systems over opts-catalogue, opts-steam, opts-repaints and opts-game-state"]:::app
        opts_traffic_schedule["opts-traffic-schedule<br/>• Timetable logic for scheduled vehicles<br/>• Which runs are on the road at a clock time, and where a run is along its stops<br/>• Pure functions over opts-core"]:::app
        opts_pause_report["opts-pause-report<br/>• Report dialog state for the pause menu<br/>• Bevy state over opts-bevy"]:::app
        opts_hud_widgets["opts-hud-widgets<br/>• HUD widget slots, vehicle kinds and descriptors<br/>• Built-in widget registry<br/>• Bevy CameraMode via opts-bevy"]:::app
        opts_game["opts-game (Binary)<br/>• Main simulation executable<br/>• AppState orchestrator<br/>• Main menu & UI flow"]:::app
        opts_benchmark["opts-benchmark (Binary)<br/>• Scenery load verification<br/>• Off-screen worker & supervisor"]:::app
        opts_tools["opts-tools (Binary: opts)<br/>• CLI asset inspector<br/>• Mesh & texture converter<br/>• Dependency & format validator"]:::app
    end

    %% Dependencies
    opts_benchmark --> opts_bevy
    opts_benchmark --> opts_catalogue
    opts_benchmark --> opts_core
    opts_benchmark --> opts_game
    opts_benchmark --> opts_sim
    opts_bevy --> opts_bevy_bus_stops
    opts_bevy --> opts_bevy_ground
    opts_bevy --> opts_bevy_instancing
    opts_bevy --> opts_bevy_moon_material
    opts_bevy --> opts_bevy_season
    opts_bevy --> opts_bevy_texture_sentinel
    opts_bevy --> opts_cli
    opts_bevy --> opts_core
    opts_bevy --> opts_debounce
    opts_bevy --> opts_gui
    opts_bevy -.->|diagnostics| opts_logging
    opts_bevy --> opts_position_tag
    opts_bevy --> opts_audio
    opts_bevy --> opts_light_quality
    opts_bevy --> opts_light_quality_core
    opts_light_quality --> opts_light_quality_core
    opts_bevy --> opts_main_camera
    opts_audio --> opts_main_camera
    opts_audio --> opts_audio_curves
    opts_bevy --> opts_scene_trace
    opts_bevy --> opts_spatial_zones
    opts_spatial_zones --> opts_spatial_zones_core
    opts_bevy --> opts_sim
    opts_bevy --> opts_traffic
    opts_catalogue --> opts_core
    opts_core --> opts_l10n
    opts_core --> opts_omsi
    opts_fleet_number --> opts_core
    opts_mesh_dir --> opts_core
    opts_road_surf --> opts_core
    opts_world_stats --> opts_core
    opts_world_load --> opts_core
    opts_world_load --> opts_bevy
    opts_world_load --> opts_sim
    opts_world_load --> opts_traffic
    opts_world_load --> opts_traffic_schedule
    opts_world_load --> opts_game_state
    opts_world_load --> opts_world_stats
    opts_world_load --> opts_world_session
    opts_world_load --> opts_tile_loading
    opts_world_load --> opts_stream_budget
    opts_world_load --> opts_attachments
    opts_world_load --> opts_repaints
    opts_world_load --> opts_road_surf
    opts_world_load --> opts_vehicle_script_texture
    opts_world_load --> opts_fleet_number
    opts_world_load --> opts_mesh_dir
    opts_world_load --> opts_collision
    opts_world_load --> opts_logging
    opts_world_load --> opts_gui
    opts_vehicle_script_texture --> opts_core
    opts_exhaust --> opts_bevy
    opts_exhaust --> opts_sim
    opts_mesh_dir --> opts_bevy
    opts_world_session --> opts_bevy
    opts_pause_report --> opts_bevy
    opts_shell_calendar --> opts_bevy
    opts_shell_calendar --> opts_gui
    opts_shell_dialog --> opts_gui
    opts_shell_nav --> opts_bevy
    opts_shell_nav --> opts_game_state
    opts_shell_nav --> opts_gui
    opts_shell_nav --> opts_shell_icons
    opts_shell_nav --> opts_shell_keys
    opts_notification --> opts_gui
    opts_notification --> opts_shell_keys
    opts_notification --> opts_shell_nav
    opts_settings_io --> opts_game_state
    opts_attachments --> opts_bevy
    opts_attachments --> opts_core
    opts_tile_loading --> opts_game_state
    opts_tile_loading --> opts_hardware
    opts_tile_loading --> opts_stream_budget
    opts_new_game_types --> opts_bevy
    opts_new_game_types --> opts_shell_calendar
    opts_repaints --> opts_bevy
    opts_repaints --> opts_core
    opts_repaints --> opts_new_game_types
    opts_repaints --> opts_steam
    opts_app_args --> opts_cli
    opts_catalogue_service --> opts_app_args
    opts_catalogue_service --> opts_bevy
    opts_catalogue_service --> opts_catalogue
    opts_catalogue_service --> opts_game_state
    opts_catalogue_service --> opts_gui
    opts_catalogue_service --> opts_new_game_types
    opts_catalogue_service --> opts_repaints
    opts_catalogue_service --> opts_settings_io
    opts_catalogue_service --> opts_steam
    opts_game --> opts_app_args
    opts_game --> opts_catalogue_service
    opts_game --> opts_bevy
    opts_game --> opts_game_state
    opts_game_state --> opts_bevy
    opts_game_state --> opts_gui
    opts_game --> opts_catalogue
    opts_game --> opts_cli
    opts_game --> opts_collision
    opts_game --> opts_core
    opts_game --> opts_traffic_schedule
    opts_traffic_schedule --> opts_core
    opts_fleet_number --> opts_bevy
    opts_game --> opts_exhaust
    opts_game --> opts_fleet_number
    opts_game --> opts_gui
    opts_game --> opts_hardware
    opts_game --> opts_hud_keys
    opts_game --> opts_hud_widgets
    opts_game --> opts_l10n
    opts_game --> opts_load_problem
    opts_game -.->|diagnostics| opts_logging
    opts_game --> opts_map_advice
    opts_game --> opts_mesh_dir
    opts_game --> opts_road_surf
    opts_game --> opts_world_stats
    opts_game --> opts_vehicle_script_texture
    opts_game --> opts_pause_report
    opts_game --> opts_shell_calendar
    opts_game --> opts_shell_dialog
    opts_game --> opts_shell_icons
    opts_game --> opts_shell_nav
    opts_game --> opts_notification
    opts_game --> opts_settings_io
    opts_game --> opts_attachments
    opts_game --> opts_tile_loading
    opts_game --> opts_new_game_types
    opts_game --> opts_repaints
    opts_game --> opts_minimap_zoom_canvas
    opts_game --> opts_save_error
    opts_game --> opts_shell_keys
    opts_shell_keys --> opts_shell_keys_help
    opts_game --> opts_sim
    opts_game --> opts_steam
    opts_game --> opts_theme_files
    opts_game --> opts_traffic
    opts_game --> opts_world_session
    opts_game --> opts_world_load
    opts_game --> opts_vehicle_gears
    opts_gui --> opts_l10n
    opts_hud_widgets --> opts_bevy
    opts_importer --> opts_model
    opts_importer --> opts_omsi
    opts_model --> opts_types
    opts_challenge --> opts_challenge_model
    opts_challenge --> opts_challenge_scorer
    opts_challenge_scorer --> opts_challenge_model
    opts_omsi --> opts_omsi_binary
    opts_omsi --> opts_omsi_map
    opts_omsi --> opts_omsi_scenery
    opts_omsi --> opts_omsi_text
    opts_omsi --> opts_omsi_vehicles
    opts_omsi --> opts_wav
    opts_omsi_binary --> opts_omsi_text
    opts_omsi_map --> opts_omsi_text
    opts_omsi_map --> opts_types
    opts_omsi_scenery --> opts_omsi_binary
    opts_omsi_scenery --> opts_omsi_text
    opts_omsi_scenery --> opts_omsi_vehicles
    opts_omsi_vehicles --> opts_omsi_text
    opts_omsi_text --> opts_omsi_scan
    opts_omsi_text --> opts_omsi_timetable
    opts_omsi_timetable --> opts_omsi_scan
    opts_omsi_text --> opts_omsi_weather
    opts_omsi_weather --> opts_omsi_scan
    opts_sim --> opts_core
    opts_sim --> opts_sim_passengers
    opts_sim --> opts_sim_pedestrian
    opts_sim_passengers --> opts_sim_passenger_model
    opts_sim_pedestrian --> opts_core
    opts_steam --> opts_vdf
    opts_theme_files --> opts_gui
    opts_tools --> opts_core
    opts_tools --> opts_hardware
    opts_tools --> opts_l10n
    opts_traffic --> opts_car_following
    opts_traffic --> opts_vehicle_traits
    opts_vehicle_traits --> opts_car_following
    opts_traffic --> opts_signals
    opts_traffic --> opts_traffic_weather
    opts_traffic_weather --> opts_car_following
    opts_game --> opts_stream_budget
    opts_game --> opts_scripted_drive
    opts_bevy --> opts_scene_trace
    opts_bevy --> opts_spatial_zones
    opts_spatial_zones --> opts_spatial_zones_core
    opts_traffic --> opts_core
```

---

## 📋 Crate Directory & Inventory

| Crate | Layer | Purpose | Dependencies (Internal) | Depended By | Engine Coupling |
| :--- | :---: | :--- | :--- | :--- | :---: |
| **`opts-logging`** | L0 | Structured diagnostics, subsystem log files, rotation, panic hook | *None* | `opts-bevy`, `opts-game` | Pure Rust |
| **`opts-cli`** | L0 | Control-protocol types and the command-line client that remote-controls a running game | *None* | `opts-bevy`, `opts-game` | Pure Rust |
| **`opts-collision`** | L0 | Planar oriented-box collision: separating-axis contact, velocity response and a spatial grid | *None* | `opts-game` | Pure Rust |
| **`opts-hud-keys`** | L0 | HUD message key tables (split out of `opts-game`, ISS-062 stage 1; [issue](../../dev/issues/ISS-062-split-opts-game.md)) | *None* | `opts-game` | Pure Rust |
| **`opts-shell-keys`** | L0 | Shell message key tables and their group modules (split out of `opts-game`, ISS-062 stage 1); facade over `opts-shell-keys-help` (ISS-091) | `opts-shell-keys-help` | `opts-game` | Pure Rust |
| **`opts-shell-keys-help`** | L0 | Help overlay message key table and the `keys!` macro (split out of `opts-shell-keys`, ISS-091 stage 1) | *None* | `opts-shell-keys` | Pure Rust |
| **`opts-minimap-zoom-canvas`** | L0 | Mini map speed zoom filter and the RGBA8 pixel canvas (split out of `opts-game`, ISS-062 stage 1) | *None* | `opts-game` | Pure Rust |
| **`opts-save-error`** | L0 | Save container error type (split out of `opts-game`, ISS-062 stage 1) | *None* | `opts-game` | Pure Rust |
| **`opts-load-problem`** | L0 | Load problem type and its severity table (split out of `opts-game`, ISS-062 stage 1) | *None* | `opts-game` | Pure Rust |
| **`opts-vehicle-gears`** | L0 | AI engine RPM and gear model of the vehicle sound stream (split out of `opts-game`, ISS-062 stage 1) | *None* | `opts-game` | Pure Rust |
| **`opts-types`** | L1 | Shared geometry and tile-grid maths both the legacy readers and the native model need ([ADR-0014](../adr/0014-data-model-separation.md)) | *None* | `opts-model`, `opts-omsi-map` | Pure Rust |
| **`opts-l10n`** | L1 | Localisation without the engine: Fluent-subset parser, language packs, checks, JSON interchange | *None* | `opts-core`, `opts-game`, `opts-gui`, `opts-tools` | Pure Rust |
| **`opts-omsi`** | L1 | Facade over the OMSI reader crates (`opts-omsi-text`, `-binary`, `-vehicles`, `-map`, `-scenery`): re-exports them at the old `opts_omsi::formats::...` paths, so callers are unchanged ([plan](../../dev/crate-split-omsi.md)); a leaf adapter | `opts-omsi-binary`, `opts-omsi-map`, `opts-omsi-scenery`, `opts-omsi-text`, `opts-omsi-vehicles`, `opts-wav` | `opts-core`, `opts-importer` | Pure Rust |
| **`opts-omsi-binary`** | L1 | OMSI binary mesh readers (`.o3d`, `.x`) split out of `opts-omsi` ([plan](../../dev/crate-split-omsi.md)) | `opts-omsi-text` | `opts-omsi`, `opts-omsi-scenery` | Pure Rust |
| **`opts-omsi-vehicles`** | L1 | OMSI vehicle readers (`.bus`, `model.cfg`, `paths.cfg`, `.hof`, script textures), the setvar catalogue and the `.sco` light reader, split out of `opts-omsi` ([plan](../../dev/crate-split-omsi.md)) | `opts-omsi-text` | `opts-omsi`, `opts-omsi-scenery` | Pure Rust |
| **`opts-omsi-map`** | L1 | OMSI map readers and writers (`global.cfg`, `tile_*.map`, `.map.terrain`, `.water`, OSN times) and the `chrono` overlay, split out of `opts-omsi` ([plan](../../dev/crate-split-omsi.md)) | `opts-omsi-text`, `opts-types` | `opts-omsi` | Pure Rust |
| **`opts-omsi-scenery`** | L1 | OMSI scenery readers (`.sco`, `.sli`, crossings) and the train consist reader, split out of `opts-omsi` ([plan](../../dev/crate-split-omsi.md)) | `opts-omsi-binary`, `opts-omsi-text`, `opts-omsi-vehicles` | `opts-omsi` | Pure Rust |
| **`opts-omsi-text`** | L1 | OMSI text readers: the text-only parsers (`.cfg`, `.cti`, `.oft`, `.org`, sound lists) split out of `opts-omsi` ([plan](../../dev/crate-split-omsi.md)); re-exports the scanner, the timetable and the weather readers as `formats::text`, `formats::timetable` and `formats::weather` | `opts-omsi-scan`, `opts-omsi-timetable`, `opts-omsi-weather` | `opts-omsi`, `opts-omsi-binary`, `opts-omsi-map`, `opts-omsi-scenery`, `opts-omsi-vehicles` | Pure Rust |
| **`opts-omsi-scan`** | L1 | OMSI keyword-block scanner: text decoding, the `[keyword]` block scan and the positional value parsers, split out of `opts-omsi-text` (ISS-087 stage 0; [plan](../../dev/issues/ISS-087-epic-split-opts-omsi-text.md)) | *None* | `opts-omsi-text`, `opts-omsi-timetable`, `opts-omsi-weather` | Pure Rust (external: `glam`)
| **`opts-omsi-timetable`** | L1 | OMSI timetable readers (`Busstops.cfg`, `.ttp`, `.ttl`, `.ttr`) and the timetable query model, split out of `opts-omsi-text` (ISS-087 stage 1; [plan](../../dev/issues/ISS-087-epic-split-opts-omsi-text.md)) | `opts-omsi-scan` | `opts-omsi-text` | Pure Rust
| **`opts-omsi-weather`** | L1 | OMSI weather readers (`.owt` presets, `clouds.cfg`, `.dsc` names) and the weather folder loader, split out of `opts-omsi-text` (ISS-087 stage 2; [plan](../../dev/issues/ISS-087-epic-split-opts-omsi-text.md)) | `opts-omsi-scan` | `opts-omsi-text` | Pure Rust
| **`opts-wav`** | L1 | RIFF/WAVE sound decoder (PCM 8/16/24 bit, IEEE float 32 bit) to interleaved `f32` samples (split out of `opts-omsi`, M-0117 budget room; [plan](../../dev/crate-split-weather.md)) | *None* | `opts-omsi` | Pure Rust |
| **`opts-vdf`** | L1 | Valve KeyValues (VDF) text parser for Steam library and app manifest files (split out of `opts-game`, ISS-034 budget room; [plan](../../dev/crate-split-game.md)) | *None* | `opts-steam` | Pure Rust |
| **`opts-steam`** | L1 | Steam library and OMSI 2 install discovery: reads `libraryfolders.vdf` and the app manifests through `opts-vdf` (split out of `opts-game`, ISS-034 budget room; [plan](../../dev/crate-split-steam.md)) | `opts-vdf` | `opts-game` | Pure Rust |
| **`opts-position-tag`** | L1 | Player position text for log messages (tile, local and world coordinates; split out of `opts-bevy`, M-0122) | *None* | `opts-bevy` | Pure Rust |
| **`opts-model`** | L1 | The native content model and its TOML form (stubs: `VehicleDefinition`, `ContentSource`) | `opts-types` | `opts-importer` | Pure Rust |
| **`opts-challenge-model`** | L1 | The challenge model of M-0172: rules, challenge, run events and the result, split out of `opts-challenge` (ISS-089) | *None* | `opts-challenge`, `opts-challenge-scorer` | Pure Rust |
| **`opts-importer`** | L1 | Turns legacy OMSI content into the native model; the only crate that knows both | `opts-model`, `opts-omsi` | *None yet* | Pure Rust |
| **`opts-core`** | L1 | Temporary facade: re-exports `opts-omsi` and `opts-l10n` at the old paths until the last consumer is migrated | `opts-l10n`, `opts-omsi` | `opts-benchmark`, `opts-bevy`, `opts-catalogue`, `opts-fleet-number`, `opts-game`, `opts-mesh-dir`, `opts-road-surf`, `opts-sim`, `opts-tools`, `opts-traffic` | Pure Rust |
| **`opts-sim`** | L2 | Vehicle dynamics, articulated-segment kinematics, pedestrians, OMSI script VM | `opts-core` | `opts-benchmark`, `opts-bevy`, `opts-exhaust`, `opts-game` | Pure Rust |
| **`opts-traffic`** | L2 | AI road traffic: lane graph, conflict zones, right of way, agents and lane changes | `opts-car-following`, `opts-core`, `opts-vehicle-traits`, `opts-signals`, `opts-traffic-weather` | `opts-bevy`, `opts-game` | Pure Rust |
| **`opts-car-following`** | L2 | The Intelligent Driver Model car-following law, vehicle classes and driver parameters, split from `opts-traffic` ([plan](../../dev/crate-split-traffic.md)) | *None* | `opts-traffic`, `opts-vehicle-traits`, `opts-traffic-weather` | Pure Rust |
| **`opts-vehicle-traits`** | L2 | Traffic traits of a vehicle type (class, length, mass, drive and brake force) mapped onto the car-following driver parameters, split out of `opts-traffic` (crate split, M-0103) | `opts-car-following` | `opts-traffic` | Pure Rust |
| **`opts-signals`** | L2 | Traffic-light programs and the aspect each instant shows to a vehicle, split out of `opts-traffic` (crate split, M-0103) | *None* | `opts-traffic` | Pure Rust |
| **`opts-scripted-drive`** | L2 | The near-vehicle drive (M-0103): the desired acceleration as script pedals, and the script's wheel torque and brake force back as an acceleration | *None* | `opts-game` | Pure Rust |
| **`opts-scene-trace`** | L2 | The scene trace: records, filters and the trace store of what is under the camera, split out of `opts-bevy` (crate split) | *None* | `opts-bevy` | Bevy |
| **`opts-spatial-zones`** | L2 | Spatial zones (ceilings, tunnels) with their sampling, tracking and overlays, split out of `opts-bevy` (crate split); the tracker and sample gate are re-exported from `opts-spatial-zones-core` (ISS-092) | `opts-spatial-zones-core` | `opts-bevy` | Bevy |
| **`opts-spatial-zones-core`** | L2 | The Bevy-free part of the spatial zones: the hysteresis tracker and the 10 Hz sample gate, split out of `opts-spatial-zones` (ISS-092) | *None* | `opts-spatial-zones` | Pure Rust |
| **`opts-challenge`** | L2 | Facade over `opts-challenge-model` and `opts-challenge-scorer`: re-exports them at the old paths (ISS-089) | `opts-challenge-model`, `opts-challenge-scorer` | *None* | Pure Rust |
| **`opts-challenge-scorer`** | L2 | The scorer of M-0172: applies run events to a challenge and produces its result, split out of `opts-challenge` (ISS-089) | `opts-challenge-model` (external: `tracing`) | `opts-challenge` | Pure Rust |
| **`opts-stream-budget`** | L2 | Tile-streaming frame budget and prioritisation (M-0089): wanted and evictable tiles, spreading and motion policy, split out of `opts-game` (crate split) | *None* | `opts-game` | Pure Rust |
| **`opts-traffic-weather`** | L2 | Weather for AI drivers: grip and fog visibility change the car-following parameters, split out of `opts-traffic` (M-0117) | `opts-car-following` | `opts-traffic` | Pure Rust |
| **`opts-world-stats`** | L2 | Counters, positions and camera setup of the loaded world, split out of `opts-game` omsi_world (ISS-067 stage 1) | `opts-core` | `opts-game` | Bevy Engine |
| **`opts-world-load`** | L3 | The in-progress world load (ISS-067 stage 1, ISS-064 chain 3): `WorldLoadJob` and the closure it reaches, moved out of `opts-game` omsi_world in one atomic change; does not compile until the compile-fix phase | `opts-attachments`, `opts-bevy`, `opts-collision`, `opts-core`, `opts-fleet-number`, `opts-game-state`, `opts-gui`, `opts-logging`, `opts-mesh-dir`, `opts-repaints`, `opts-road-surf`, `opts-scripted-drive`, `opts-sim`, `opts-stream-budget`, `opts-tile-loading`, `opts-traffic`, `opts-traffic-schedule`, `opts-vehicle-script-texture`, `opts-world-session`, `opts-world-stats` | `opts-game` | Bevy Engine |
| **`opts-vehicle-script-texture`** | L2 | Vehicle script texture canvases, colour maps and the per-frame GPU upload, split out of `opts-game` omsi_world (ISS-067 stage 1) | `opts-core` | `opts-game` | Bevy Engine |
| **`opts-kinematic`** | L2 | Cheap kinematic acceleration of an AI vehicle from its measured drive torque over speed; the run-up law shared with the script census in `opts-sim` examples (M-0103) | *None* | *None* | Pure Rust |
| **`opts-sim-passengers`** | L2 | The passengers of a vehicle: boarding through open doors, tickets, seats and standing places, getting off (split out of `opts-sim`, ISS-066 stage 1; [epic](../../dev/issues/ISS-066-epic-split-opts-sim.md)) | `opts-sim-passenger-model` | `opts-sim` | Pure Rust |
| **`opts-sim-passenger-model`** | L2 | The passenger model's data (cabin, paths, ticket types), the path graph of a cabin and the random source, split out of `opts-sim-passengers` (ISS-086 stage 1); re-exported by `opts-sim-passengers` | *None* | `opts-sim-passengers` | Pure Rust |
| **`opts-sim`** | L2 | Vehicle dynamics, articulated-segment kinematics, pedestrians, OMSI script VM | `opts-core`, `opts-sim-passengers` | `opts-benchmark`, `opts-bevy`, `opts-exhaust`, `opts-game` | Pure Rust |
| **`opts-core`** | L1 | Temporary facade: re-exports `opts-omsi` and `opts-l10n` at the old paths until the last consumer is migrated | `opts-l10n`, `opts-omsi` | `opts-benchmark`, `opts-bevy`, `opts-catalogue`, `opts-game`, `opts-sim`, `opts-tools`, `opts-traffic` | Pure Rust |
| **`opts-sim`** | L2 | Vehicle dynamics, articulated-segment kinematics, OMSI script VM; pedestrians are re-exported from `opts-sim-pedestrian` | `opts-core`, `opts-sim-passengers`, `opts-sim-pedestrian` | `opts-benchmark`, `opts-bevy`, `opts-exhaust`, `opts-game` | Pure Rust |
| **`opts-sim-pedestrian`** | L2 | Pedestrian model: footpath lanes from splines and objects, the lane network, walkers and the gait, split out of `opts-sim` (ISS-066, stage 2) | `opts-core` | `opts-sim` | Pure Rust |
| **`opts-traffic`** | L2 | AI road traffic: lane graph, conflict zones, right of way, agents and lane changes | `opts-car-following`, `opts-core` | `opts-bevy`, `opts-game` | Pure Rust |
| **`opts-car-following`** | L2 | The Intelligent Driver Model car-following law, vehicle classes and driver parameters, split from `opts-traffic` ([plan](../../dev/crate-split-traffic.md)) | *None* | `opts-traffic` | Pure Rust |
| **`opts-audio`** | L2 | Sound engine: decoded clips, emitters, the voice budget and the distance fade, split out of `opts-bevy` (crate split); the gain curves are re-exported from `opts-audio-curves` (ISS-088) | `opts-audio-curves`, `opts-main-camera` | `opts-bevy` | Bevy |
| **`opts-audio-curves`** | L2 | Gain curves of the sound engine: the distance fade, decibel and percent gains and the semitone speed factor, split out of `opts-audio` (ISS-088) | *None* | `opts-audio` | Pure Rust |
| **`opts-light-quality`** | L2 | Lighting quality: the effective light state, the fallback ladder and the automatic frame-time governor, split out of `opts-bevy` (crate split) | `opts-light-quality-core` | `opts-bevy` | Bevy |
| **`opts-light-quality-core`** | L2 | The Bevy-free part of the lighting quality: the shafts, light mode and bloom levels with their names and parsers, split out of `opts-light-quality` (ISS-090) | *None* | `opts-bevy`, `opts-light-quality` | Pure Rust |
| **`opts-main-camera`** | L2 | The `MainCamera` marker, the one camera that shows the player's view, split out of `opts-bevy` so that `opts-audio` can query it (crate split) | *None* | `opts-audio`, `opts-bevy` | Bevy |
| **`opts-catalogue`** | L2 | Scans an installation into a map/vehicle catalogue | `opts-core` | `opts-benchmark`, `opts-game` | Pure Rust |
| **`opts-theme-files`** | L2 | RON theme files in the configuration folder: discovery and loading through the contrast checker of `opts-gui` (split out of `opts-game`, ISS-034 budget room; [plan](../../dev/crate-split-theme.md)) | `opts-gui` | `opts-game` | Pure Rust (external: `directories`) |
| **`opts-gui`** | L2 | Material Design 3 UI widget library and theme tokens | `opts-l10n` | `opts-bevy`, `opts-game`, `opts-game-state`, `opts-theme-files` | Bevy UI |
| **`opts-gui`** | L2 | Material Design 3 UI widget library and theme tokens | `opts-l10n` | `opts-bevy`, `opts-game`, `opts-shell-calendar`, `opts-shell-dialog`, `opts-theme-files` | Bevy UI |
| **`opts-debounce`** | L0 | Trailing debounce of file-change events for the hot-reload watcher (split out of `opts-bevy`, M-0117 budget room; [plan](../../dev/crate-split-weather.md)) | *None* | `opts-bevy` | Pure Rust |
| **`opts-map-advice`** | L0 | Map load profile and the heuristic advice engine: pure thresholds, no engine dependency (split out of `opts-game`, [plan](../../dev/crate-split-advice.md)) | *None* | `opts-game` | Pure Rust |
| **`opts-shell-icons`** | L0 | Material Icons codepoints used by the shell (split out of `opts-game`, ISS-068 stage 1) | *None* | `opts-game` | Pure Rust |
| **`opts-hardware`** | L0 | Privacy-respecting hardware capability probe and default recommendations ([charter](hardware-probe.md)): no network, no identifiers, Linux/macOS/Windows | *None* (external: `sysinfo`) | `opts-game`, `opts-tools` | Pure Rust |
| **`opts-bevy`** | L2 | Bevy loaders, materials, coordinates, vehicle/script/articulation bridges, control server, hot reload | `opts-cli`, `opts-core`, `opts-debounce`, `opts-gui`, `opts-logging`, `opts-position-tag`, `opts-scene-trace`, `opts-spatial-zones`, `opts-sim`, `opts-traffic` | `opts-benchmark`, `opts-exhaust`, `opts-game` | Bevy Engine |
| **`opts-bevy`** | L2 | Bevy loaders, materials, coordinates, vehicle/script/articulation bridges, control server, hot reload | `opts-audio`, `opts-cli`, `opts-core`, `opts-debounce`, `opts-gui`, `opts-light-quality`, `opts-light-quality-core`, `opts-logging`, `opts-main-camera`, `opts-position-tag`, `opts-scene-trace`, `opts-sim`, `opts-spatial-zones`, `opts-traffic` | `opts-benchmark`, `opts-exhaust`, `opts-game` | Bevy Engine |
| **`opts-bevy-bus-stops`** | L2 | Bus stop marker component of the map objects tagged `[busstop]` (split out of `opts-bevy`, ISS-065 stage 2) | *None* | `opts-bevy` | Bevy Engine |
| **`opts-bevy-ground`** | L2 | Ground texture CPU images: decoding, block-compressed texels, wrap sampling and tile ground composition (split out of `opts-bevy`, ISS-065 stage 2) | *None* | `opts-bevy` | Bevy Engine |
| **`opts-bevy-instancing`** | L2 | Mesh instancing layers (dedupe and static merge, `OPTS_INSTANCING`) and the `render.stats` diagnostics (split out of `opts-bevy`, ISS-065 stage 2) | *None* | `opts-bevy` | Bevy Engine |
| **`opts-bevy-moon-material`** | L2 | Moon disc material: the moon photograph with its phase terminator, and its shader (split out of `opts-bevy`, ISS-065 stage 2) | *None* | `opts-bevy` | Bevy Engine |
| **`opts-bevy-season`** | L2 | Season state of the scenery: half-year variables, snow cover and the season texture folders (split out of `opts-bevy`, ISS-065 stage 2) | *None* | `opts-bevy` | Pure Rust |
| **`opts-bevy-texture-sentinel`** | L2 | Benchmark-only missing-texture sentinel material (split out of `opts-bevy`, ISS-065 stage 2) | *None* | `opts-bevy` | Bevy Engine |
| **`opts-bevy`** | L2 | Bevy loaders, materials, coordinates, vehicle/script/articulation bridges, control server, hot reload | `opts-bevy-bus-stops`, `opts-bevy-ground`, `opts-bevy-instancing`, `opts-bevy-moon-material`, `opts-bevy-season`, `opts-bevy-texture-sentinel`, `opts-cli`, `opts-core`, `opts-debounce`, `opts-gui`, `opts-logging`, `opts-position-tag`, `opts-sim`, `opts-traffic` | `opts-benchmark`, `opts-exhaust`, `opts-game` | Bevy Engine |
| **`opts-exhaust`** | L3 | Engine exhaust plumes of the player and the nearest vehicles: puff emission, the model outlet, the distance fade (split out of `opts-game`, M-0117 budget room; [plan](../../dev/crate-split-exhaust.md)) | `opts-bevy`, `opts-sim` | `opts-game` | Bevy Engine |
| **`opts-game`** | L3 | Game application: menus, world loading (tiles, traffic, pedestrians), state machine | `opts-bevy`, `opts-catalogue`, `opts-cli`, `opts-collision`, `opts-core`, `opts-exhaust`, `opts-gui`, `opts-hardware`, `opts-l10n`, `opts-logging`, `opts-map-advice`, `opts-scripted-drive`, `opts-sim`, `opts-steam`, `opts-stream-budget`, `opts-theme-files`, `opts-traffic` | `opts`, `opts-benchmark` | Bevy Engine |
| **`opts-bevy`** | L2 | Bevy loaders, materials, coordinates, vehicle/script/articulation bridges, control server, hot reload | `opts-cli`, `opts-core`, `opts-debounce`, `opts-gui`, `opts-logging`, `opts-position-tag`, `opts-sim`, `opts-traffic` | `opts-benchmark`, `opts-exhaust`, `opts-game`, `opts-game-state` | Bevy Engine |
| **`opts-game-state`** | L3 | Shared state types of the game: `AppState`, `GameSettings` with its defaults, session overlay and baseline, `Look` and `LookParams`, the in-game components and resources (split out of `opts-game`, ISS-064 stage 2). Types and pure helpers only; file I/O and systems stay in `opts-game` | `opts-bevy`, `opts-gui` | `opts-game` | Bevy Engine |
| **`opts-game`** | L3 | Game application: menus, world loading (tiles, traffic, pedestrians), state machine | `opts-bevy`, `opts-catalogue`, `opts-cli`, `opts-collision`, `opts-core`, `opts-exhaust`, `opts-game-state`, `opts-gui`, `opts-hardware`, `opts-l10n`, `opts-logging`, `opts-map-advice`, `opts-sim`, `opts-steam`, `opts-theme-files`, `opts-traffic` | `opts`, `opts-benchmark` | Bevy Engine |
| **`opts-gui`** | L2 | Material Design 3 UI widget library and theme tokens | `opts-l10n` | `opts-bevy`, `opts-game`, `opts-theme-files` | Bevy UI |
| **`opts-road-surf`** | L2 | Fine road-texture unevenness: the `.surf` height maps, texture strips and the synthetic roughness per surface type (split out of `opts-game`'s `omsi_world`, ISS-067 stage 1) | `opts-core` | `opts-game` | Pure Rust (external: `image`) |
| **`opts-bevy`** | L2 | Bevy loaders, materials, coordinates, vehicle/script/articulation bridges, control server, hot reload | `opts-cli`, `opts-core`, `opts-debounce`, `opts-gui`, `opts-logging`, `opts-position-tag`, `opts-sim`, `opts-traffic` | `opts-benchmark`, `opts-exhaust`, `opts-fleet-number`, `opts-game`, `opts-mesh-dir`, `opts-world-session` | Bevy Engine |
| **`opts-fleet-number`** | L3 | Vehicle fleet number drawn from the `.org` registration list of a `.bus` with the script seed (split out of `opts-game`'s `omsi_world`, ISS-067 stage 1) | `opts-bevy`, `opts-core` | `opts-game` | Bevy Engine (via `opts-bevy`) |
| **`opts-mesh-dir`** | L3 | Base folder of a vehicle's mesh paths: the config folder or the model folder, by the meshes each resolves (split out of `opts-game`'s `omsi_world`, ISS-067 stage 1) | `opts-bevy`, `opts-core` | `opts-game` | Bevy Engine (via `opts-bevy`) |
| **`opts-world-session`** | L3 | Resolved content paths and the menu's request for a world session (split out of `opts-game`'s `omsi_world`, ISS-067 stage 1) | `opts-bevy` | `opts-game` | Bevy Engine (via `opts-bevy`) |
| **`opts-game`** | L3 | Game application: menus, world loading (tiles, traffic, pedestrians), state machine | `opts-bevy`, `opts-catalogue`, `opts-cli`, `opts-collision`, `opts-core`, `opts-exhaust`, `opts-fleet-number`, `opts-gui`, `opts-hardware`, `opts-l10n`, `opts-logging`, `opts-map-advice`, `opts-mesh-dir`, `opts-road-surf`, `opts-sim`, `opts-steam`, `opts-theme-files`, `opts-traffic`, `opts-traffic-schedule`, `opts-world-session` | `opts`, `opts-benchmark` | Bevy Engine |
| **`opts-bevy`** | L2 | Bevy loaders, materials, coordinates, vehicle/script/articulation bridges, control server, hot reload | `opts-cli`, `opts-core`, `opts-debounce`, `opts-gui`, `opts-logging`, `opts-position-tag`, `opts-sim`, `opts-traffic` | `opts-benchmark`, `opts-exhaust`, `opts-game`, `opts-pause-report`, `opts-shell-calendar` | Bevy Engine |
| **`opts-shell-calendar`** | L3 | Date and time steppers, their arithmetic and the frame of a choice chip (split out of `opts-game`, ISS-068 stage 1) | `opts-bevy`, `opts-gui` | `opts-game` | Bevy UI |
| **`opts-shell-dialog`** | L3 | Confirmation dialogs: the spec, the card and the answer systems (split out of `opts-game`, ISS-068 stage 1) | `opts-gui` | `opts-game` | Bevy UI |
| **`opts-shell-nav`** | L3 | Shell navigation: the screen registry, the rail frame, the back stack, Esc and banners (split out of `opts-game`, ISS-068 stage 2) | `opts-bevy`, `opts-game-state`, `opts-gui`, `opts-shell-icons`, `opts-shell-keys` | `opts-game` | Bevy UI |
| **`opts-notification`** | L3 | Notification service: toasts, errors with their durable place, and the refused-navigation toast (split out of `opts-game`'s `shell/notify`, ISS-068 remainder) | `opts-gui`, `opts-shell-keys`, `opts-shell-nav` (external: `bevy`, `tracing`) | `opts-game` | Bevy UI |
| **`opts-settings-io`** | L3 | Settings file I/O: `settings.toml`, `settings.db` and the background writer (split out of `opts-game`'s `settings`, ISS-068 remainder) | `opts-game-state` (external: `directories`, `rusqlite`, `toml`, `tracing`) | `opts-game` | Pure Rust |
| **`opts-attachments`** | L3 | Spline attachments: placements of street furniture, fences and parking bays along splines, and the map-wide spline index (split out of `opts-game`, ISS-067 stage 1) | `opts-bevy`, `opts-core` | `opts-game` | Bevy Engine |
| **`opts-tile-loading`** | L3 | Tile loading controls (M-0080): the load-all and tile-radius decisions, the streaming budget and residency as pure functions (split out of `opts-game`, ISS-067 stage 1) | `opts-game-state`, `opts-hardware`, `opts-stream-budget` | `opts-game` | Pure Rust |
| **`opts-new-game-types`** | L3 | Plan-a-drive types without UI: the catalogue lists, the drive choices, the trip choice and the planner date arithmetic (split out of `opts-game`, ISS-064 stage 3) | `opts-bevy`, `opts-shell-calendar` (external: `bevy`, `chrono`, `serde`) | `opts-game` | Bevy Engine |
| **`opts-repaints`** | L3 | Vehicle repaint discovery for the new-game vehicle tab: the `.bus`, `model.cfg` and `.cti` chain as pure file reads (split out of `opts-game`, ISS-064 stage 3) | `opts-bevy`, `opts-core`, `opts-new-game-types`, `opts-steam` | `opts-game` | Pure Rust |
| **`opts-app-args`** | L3 | The parsed command line as a Bevy resource, `AppArgs` (split out of `opts-game`'s `app` module, ISS-064 stage 3) | `opts-cli` (external: `bevy`) | `opts-game` | Bevy Engine |
| **`opts-catalogue-service`** | L3 | Background discovery and menu lists backed by the SQLite catalogue: startup, the discovery thread and selection reconciliation (split out of `opts-game`, ISS-064 stage 3) | `opts-app-args`, `opts-bevy`, `opts-catalogue`, `opts-game-state`, `opts-gui`, `opts-new-game-types`, `opts-repaints`, `opts-settings-io`, `opts-steam` (external: `bevy`) | `opts-game` | Bevy Engine |
| **`opts-traffic-schedule`** | L3 | Timetable logic for scheduled vehicles: which runs are on the road at a clock time and where a run is along its stops (split out of `opts-game`, ISS-064 stage 3) | `opts-core` | `opts-game` | Pure Rust |
| **`opts-pause-report`** | L3 | Report dialog state helper for the pause menu (split out of `opts-game`, ISS-068 stage 1) | `opts-bevy` | `opts-game` | Bevy Engine |
| **`opts-game`** | L3 | Game application: menus, world loading (tiles, traffic, pedestrians), state machine | `opts-bevy`, `opts-catalogue`, `opts-cli`, `opts-collision`, `opts-core`, `opts-exhaust`, `opts-gui`, `opts-hardware`, `opts-l10n`, `opts-logging`, `opts-map-advice`, `opts-pause-report`, `opts-shell-calendar`, `opts-shell-dialog`, `opts-shell-icons`, `opts-sim`, `opts-steam`, `opts-theme-files`, `opts-traffic` | `opts`, `opts-benchmark` | Bevy Engine |
| **`opts-hud-widgets`** | L3 | HUD widget vocabulary: slots, vehicle kinds, widget descriptors and the built-in registry (split out of `opts-game`, ISS-062 stage 1; [issue](../../dev/issues/ISS-062-split-opts-game.md)) | `opts-bevy` (external: `serde`) | `opts-game` | Bevy Engine |
| **`opts-game`** | L3 | Game application: menus, world loading (tiles, traffic, pedestrians), state machine | `opts-bevy`, `opts-catalogue`, `opts-cli`, `opts-collision`, `opts-core`, `opts-exhaust`, `opts-gui`, `opts-hardware`, `opts-hud-keys`, `opts-hud-widgets`, `opts-l10n`, `opts-load-problem`, `opts-logging`, `opts-map-advice`, `opts-minimap-zoom-canvas`, `opts-save-error`, `opts-shell-keys`, `opts-sim`, `opts-steam`, `opts-theme-files`, `opts-traffic`, `opts-vehicle-gears` | `opts`, `opts-benchmark` | Bevy Engine |
| **`opts`** | L3 | Workspace root binary | `opts-game` | *None (executable root)* | — |
| **`opts-tools`** | L3 | File inspection tool (`inspect`) | `opts-core`, `opts-hardware`, `opts-l10n` | *None (executable root)* | CLI Tooling |
| **`opts-benchmark`** | L3 | Scenery object load verification harness ([spec](scenery-benchmark.md)): an off-screen worker that rebuilds objects through the loader and a supervisor | `opts-bevy`, `opts-catalogue`, `opts-core`, `opts-game`, `opts-sim` | *None (executable root)* | Bevy |

*Table regenerated from `cargo metadata` (normal dependencies) on 2026-10-11 by the [critical review](../meta/critical-review-2026-10-11.md) (F-011). Two edges break the intended layering: `opts-bevy` → `opts-gui` (reporting modal) and `opts-bevy` → `opts-cli` (audit 5037f89a A-F-015, M-0046). The Mermaid diagram above has not been updated yet (M-0041).*
---

## 🔒 Invariant Boundary Rules

1. **`opts-omsi` Zero-Engine & Zero-IO Invariant, and the legacy/native boundary ([ADR-0014](../adr/0014-data-model-separation.md)):**
   - Must never depend on `bevy`, `wgpu`, or windowing crates, and never on `opts-model`.
   - Must never perform synchronous file I/O (`std::fs::File`, `std::io::Read`). All parsers operate strictly on in-memory slices (`&str`, `&[u8]`).
   - It is a **leaf adapter**: `opts-bevy`, `opts-sim`, `opts-traffic`, `opts-car-following`, `opts-gui` and `opts-catalogue` must not depend on it (directly or through the `opts-core` facade) in new code. The uses that remain are counted by `scripts/omsi_uses.py` against `docs/dev/omsi-uses-known.txt`; the count may only fall. `opts-game` is the composition root and may wire the OMSI source through `opts-importer`.
   - `opts-model` and `opts-omsi` share only `opts-types` and never import each other; `opts-importer` is the only crate that depends on both.
2. **`opts-sim` Zero-Engine & Zero-IO Invariant:**
   - Must never depend on `bevy` or rendering crates.
   - Simulation logic must remain 100% deterministic and headless-runnable.
3. **`opts-logging` Zero-OPTS Invariant:**
   - Must never depend on any OPTS internal crate (`opts-omsi`, `opts-sim`, etc.).
   - Provides pure tracing layers, rotation policies, and file writers.
4. **Engine Access Gating:**
   - Only `opts-bevy`, `opts-gui`, and `opts-game` are permitted to declare `bevy` as a dependency.
   - **Amendment (ISS-064 stage 2, owner-approved):** the exempt crates are also the Bevy-facing type crates listed in the inventory with the coupling "Bevy Engine": currently `opts-game-state`. They may declare `bevy` for the derives of their types, and they hold types and pure helpers only; systems, plugins, UI and file I/O stay in `opts-game`.
