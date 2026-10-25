# 🚀 Application Startup Architecture & Wiring

This document specifies the startup lifecycle, plugin composition, CLI interface, and shutdown protocol for the primary OPTS game executable (`opts-game`).

---

## 🏛️ Overview & Architectural Role

[`opts-game`](../../crates/opts-game) serves as the top-level application composition root (Layer 3 in [ADR-0005](../adr/0005-crate-architecture.md)). It does not implement low-level asset parsing or physics kinematics itself; rather, it coordinates:
- Foundational diagnostic logging via [`opts-logging`](../adr/0004-logging-architecture.md).
- Engine bridge plugins and asset loaders via [`opts-bevy`](../architecture/crate-graph.md).
- Material Design 3 user interface systems via [`opts-gui`](../architecture/gui-design-system.md).
- Top-level application state transitions (`Loading` $\to$ `MainMenu` $\to$ `InGame`).
- Command-line argument parsing and persistent user preferences (`GameSettings`).

---

## ⏱️ Startup Sequence

The execution flow from binary launch to active game loop execution proceeds through a strict 15-step sequence:

```mermaid
sequenceDiagram
    autonumber
    actor User as User / Shell
    participant Main as src/main.rs
    participant CLI as opts_cli::CliArgs
    participant Log as opts_logging
    participant App as opts_game::build_app()
    participant Bevy as Bevy Engine Runtime

    User->>Main: Launch (cargo run or cargo run -p opts-game)
    Main->>CLI: parse_args()
    CLI-->>Main: CliArgs instance
    Main->>Log: init(log_config) with CLI level/dir
    Log-->>Main: LogGuard (RAII buffer handle)
    Main->>App: build_app(args)
    App->>App: Configure WindowPlugin / Headless mode
    App->>App: Add DefaultPlugins (disable built-in LogPlugin)
    App->>App: Register AppState (Loading -> MainMenu)
    App->>App: Insert GameSettings & MaterialTheme
    App->>App: Register OptsAssetPlugin (mesh, coords, scenery, vehicle)
    App->>App: Register OptsGuiPlugin (tokens, widgets, themes)
    App->>App: Register MainMenuPlugin & StubScreensPlugin
    App->>App: Register ShutdownPlugin
    App-->>Main: Configured Bevy App
    Main->>Bevy: app.run()
    Note over Bevy: Startup systems run; window appears;<br/>state transitions to MainMenu.
    User->>Bevy: Exit application (ESC or window close)
    Bevy->>Main: app.run() returns
    Main->>Log: Drop LogGuard -> workers flushed
    Main-->>User: Exit code 0
```

---

## 🎛️ Command-Line Interface (CLI)

The CLI is powered by `clap` with derive macros. All options are optional and provide defaults suited for interactive desktop execution.

| Flag / Option | Type | Default | Description |
| :--- | :---: | :--- | :--- |
| `--map-path <PATH>` | `Option<PathBuf>` | None | OMSI map folder (`<OMSI>/maps/<Map>`) loaded by the in-game session |
| `--omsi-path <PATH>` | `Option<PathBuf>` | derived | OMSI installation root; derived from `--map-path` (`<root>/maps/<Map>`) when omitted |
| `--vehicle-path <FILE>` | `Option<PathBuf>` | map `ailists.cfg` | Player vehicle `.bus` file; falls back to the first vehicle in the map's `ailists.cfg` |
| `--autostart` | `bool` | `false` | Skip the menus and enter the in-game session immediately |
| `--screenshot <FILE>` / `--screenshot-delay <SECS>` | `Option<PathBuf>` / `f32` | None / `4` | Developer aid: save the window to a PNG after the HUD appears, then exit |
| `--recent-map <PATH>` | `Option<PathBuf>` | None | Persist specified map path into settings as the last-loaded map |
| `--log-level <LEVEL>` | `Option<String>` | `"info"` | Global log filter level (`error`, `warn`, `info`, `debug`, `trace`) |
| `--log-file <PATH>` | `Option<PathBuf>` | Platform default | Directory where subsystem log files are written |
| `--fullscreen` | `bool` | `false` | Launch window in borderless fullscreen mode |
| `--window-size <WxH>` | `Option<String>` | `"1920x1080"` | Initial window dimension (e.g. `1920x1080`); raised to the 1024x640 minimum if smaller |
| `--headless` | `bool` | `false` | Run without opening a window (for automated testing & CI) |

---

## 🪟 Window & Display Configuration

When `--headless` is **false**:
- **Title:** `"OPTS — Open Public Transport Simulator"`
- **Default Resolution:** 1920×1080 (or overridden by `--window-size <W>x<H>`; sizes below the minimum are raised to it)
- **Resize Constraints:** Minimum **1024×640** logical pixels (`opts_cli::MIN_WINDOW_SIZE`, the Phase 9 decision D-UI-04; before M-0061 it was 800×600), resizable enabled.
- **Present Mode:** `AutoVsync` (smooth tear-free frame presentation).
- **Window Mode:** `Windowed` (or `BorderlessFullscreen` when `--fullscreen` is passed).

When `--headless` is **true**:
- The `WindowPlugin` is omitted or configured with `primary_window: None`.
- `WinitPlugin` is replaced or configured for headless execution, or `ScheduleRunnerPlugin::run_loop()` is used to step frames in CI tests.

---

## 🧩 Plugin Composition Matrix

| Plugin | Source Crate | Lifecycle | Purpose |
| :--- | :--- | :--- | :--- |
| **`OptsAssetPlugin`** | `opts-bevy` | Core Bridge | Registers `.o3d`, `.x`, `.dds`, `.bmp`, `.sco`, `.sli` loaders and coordinate systems |
| **`OptsVehiclePlugin`** | `opts-bevy` | Engine Bridge | Registers `VehicleController`, inputs, and physics step systems |
| **`OptsGuiPlugin`** | `opts-gui` | UI Foundation | Registers MD3 theme tokens, focus tree, and UI builder systems |
| **`MainMenuPlugin`** | `opts-game` | UI Consumer | Spawns glassmorphic main menu, sidebar navigation, and map background |
| **`StubScreensPlugin`**| `opts-game` | UI Consumer | Placeholder views for Settings, Timetable, Bus Selection, and Map Selector |
| **`ShutdownPlugin`** | `opts-game` | Lifecycle | Intercepts `AppExit`, saves user preferences, and cleans up state |

---

## 📂 OMSI Installation Check (Startup)

`OmsiSetupPlugin` (`opts-game/src/omsi_setup.rs`) runs before the catalogue service and decides with the pure function `first_run::decide`:

| Source | Treatment |
| :--- | :--- |
| `--omsi-path` (or the root above `--map-path`), `$OMSI_PATH` | explicit: used as is, not saved |
| the folder the user **confirmed** earlier (`omsi_path` in `settings.db`) | used as is |
| an installation found **through Steam** ([below](#finding-omsi-through-steam)) | used and saved without asking (project lead, 2026-10-25; it was offered for confirmation before) |
| nothing | with a window the first-run dialog asks to choose a folder with the native dialog (`rfd`, own thread); a wrong folder is explained inline (it needs `maps` and `Vehicles`; a picked `maps/<Map>` resolves to the installation above it); without a window the catalogue stays empty |

A folder found through Steam or chosen in the dialog is saved (`first_run::adopt_root`) and the catalogue starts at once. The same function `first_run::choose_omsi_folder` serves the Home banner *Choose folder* and `Settings -> Content`; its result is the message `OmsiFolderChosen`. After the first scan, with no drive history, one welcome dialog offers *Pick a map and a vehicle* or *Quick drive* (remembered in `first_run.json`). Details: [screens §3](../roadmap/phase-9/screens.md#3-first-run-modal-flow-over-home-omsisetup). Progress of the scan is the stage the catalogue reports (scanning files, reading content, saving) in the Home status chip; the catalogue offers no item counts during a scan, the counts appear when it is done.

### Finding OMSI through Steam

OMSI 2 is Steam app **252530**. Steam records where its games are in two text files in Valve's KeyValues format (quoted keys and values, `{ }` blocks, `\\` and `\"` escapes, `//` comments):

* `<steam>/steamapps/libraryfolders.vdf` lists the **libraries** (Steam installs games into any of them). Current format: `"libraryfolders" { "0" { "path" "…" "apps" { "<appid>" "<size>" … } } "1" { … } }`; the format before 2021 has the path as the value itself: `"libraryfolders" { "1" "D:\\SteamLibrary" }`. Older Steam also kept a copy in `<steam>/config/libraryfolders.vdf`.
* `<library>/steamapps/appmanifest_252530.acf` (`"AppState" { … "installdir" "OMSI 2" … }`) names the game's folder below `<library>/steamapps/common/`.

The search (`opts-game/src/steam_library.rs`):

1. **Steam installations** of this platform: Linux `~/.steam/steam`, `~/.steam/root`, `~/.local/share/Steam`, Flatpak `~/.var/app/com.valvesoftware.Steam/.local/share/Steam` (and `…/data/Steam`), Snap `~/snap/steam/common/.local/share/Steam`; macOS `~/Library/Application Support/Steam`; Windows the `SteamPath` value of `HKEY_CURRENT_USER\Software\Valve\Steam` (read with `reg query`), then `InstallPath` under `HKEY_LOCAL_MACHINE\SOFTWARE\WOW6432Node\Valve\Steam`, then `Program Files (x86)\Steam` and `Program Files\Steam`. The same folder reached by several of these (symbolic links) is read once.
2. **Libraries** of each installation from `libraryfolders.vdf`, the installation's own folder included; the libraries whose `apps` list app 252530 come first.
3. In each library, the folder from `appmanifest_252530.acf` (else `OMSI 2`) below `steamapps/common/`; the first that is an installation (`maps` and `Vehicles`) is the result.

Nothing is written to Steam's files; a missing or unreadable file is skipped. Without a result the folder dialog asks.

## 🗄️ Catalogue Discovery (Startup)

`CataloguePlugin` (`opts-game/src/catalogue_service.rs`) runs at `Startup`: it locates the OMSI root (`--omsi-path`, root of `--map-path`, `$OMSI_PATH`, the saved settings path, the Steam libraries), opens `<config>/catalogue.db`, fills the New Game lists from it, and checks staleness. A catalogue that is missing, built for another root, or older than `discovery_interval_days × 24 h` (default 24 h) is rebuilt on a background thread (≈ 5 s for a full install); `poll_catalogue_discovery` reloads the lists and shows a toast when it commits. There are no embedded map or vehicle lists.

## 🗺️ Real-Content Loading (In-Game Session)

The in-game session loads **real OMSI content at runtime**; nothing is bundled and no placeholder geometry exists. The stage machine in `crates/opts-game/src/in_game/loading_pipeline.rs` drives `crates/opts-game/src/in_game/omsi_world/`:

| Stage | Work (real) |
| :--- | :--- |
| `InitEngine` | Resolve `--map-path`/`--omsi-path`; parse `global.cfg`; choose the start tile (first listed, existing tile with a type-0 road `[path]`); queue its 3×3 neighbourhood |
| `BuildTerrain` | One tile per frame: parse `tile_*.map`, build the terrain mesh from `.map.terrain`, extrude every `[spline]` via its `.sli`, place every `[object]` via its `.sco` → `.o3d`/`.x` + textures |
| `SetupAtmosphere` | Sun and ambient light (not OMSI content) |
| `SpawnVehicle` | Parse `.bus` → `[model]` config → load its LOD-0 meshes; spawn at the start spline's road lane; `VehicleController::from_manifest` supplies mass and axles |
| `CalibrateControls` | Spawn the chase/cockpit camera sized from the vehicle; spawn the debug overlay (F10) |

**Path resolution.** `opts_bevy::OmsiInstall::find_file(rel) resolves OMSI's backslash-separated, case-insensitive references on case-sensitive filesystems. Textures are searched in the owning object's `texture/` folder (splines: the `.sli`'s pack folder), then the global `Texture/` folder, with alternative extensions tried (`.dds`, `.bmp`, `.tga`, `.png`, `.jpg`). Lookups first consult the SQLite catalogue (`opts-catalogue`, see [catalogue-schema.md](catalogue-schema.md)) and fall back to the filesystem.

**Conventions established from the corpus** (and used by the loader):

- Map headings are **clockwise-positive, 0° = north** (a straight segment ends at `pos + L·(sin h, cos h)`; verified on connected segment pairs).
- A positive spline `radius` makes the heading **increase** with arc length (turns right).
- `[object]` height is relative to the terrain; `[spline]` height is absolute.
- Tiles are 300 m, origin at the south-west corner; terrain cells are 5 m.

**Errors.** Assets that fail to load are logged at `ERROR` and skipped. The HUD shows `--` for values the simulation cannot yet source from the vehicle's own configuration (engine RPM).

---

## 🛑 Clean Shutdown & Telemetry Flush

On application exit:
1. `ShutdownPlugin` intercepts the Bevy `AppExit` event.
2. Modified user settings (`GameSettings`) are written to `~/.config/opts/settings.toml`.
3. `main()` logs `OPTS application exited cleanly`.
4. When `main()` returns, the RAII `LogGuard` drops:
   - Closes non-blocking channel senders.
   - Background worker threads flush pending memory buffers to `opts.log`, `opts-game.log`, etc.
   - Worker threads join and process terminates with exit code 0.
