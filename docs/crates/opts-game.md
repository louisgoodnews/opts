# `opts-game`

| Metadata | Value |
| :--- | :--- |
| **Layer** | L6 · Application |
| **Role** | Application Composition Root & Consumer UI Screens |
| **Kind** | Binary (`opts-game`) + Library (`opts_game`) |
| **Status** | 🟢 Implemented (Main Menu & Lifecycle State Machine) |
| **Last Updated** | 2026-10-08 |

---

## 🎯 Purpose

`opts-game` is the primary executable and presentation orchestrator for the Open Public Transport Simulator. It provides:
1. Top-level application initialization, logging setup, and window configuration.
2. The global lifecycle state machine (`AppState`).
3. The interactive **Main Menu** featuring Material Design 3 styling, glassmorphic translucent surfaces, and dynamic map-backed background loading.
4. Destination screen routing with return navigation.
5. User settings persistence (`GameSettings`) via platform standard paths (`settings.toml` baseline, `settings.db` authoritative). Theme and interface keys (`theme_mode`, `auto_light_from`, `auto_dark_from`, `high_contrast`, `active_theme`, `theme_file`, `ui_scale`, `density`, `reduced_motion`) and the migration from `is_dark_theme` are in `settings_theme.rs`; RON theme files in `theme_files.rs`; the design is in the [GUI design system](../src/architecture/gui-design-system.md#64-settings-keys-and-migration).

---

## 🏗️ Architecture & State Machine

Navigation and simulation execution are governed deterministically by the Bevy `AppState` machine:

```rust
#[derive(States, Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum AppState {
    #[default]
    Loading,
    MainMenu,
    VehicleSelection,
    LoadGame,
    Settings,
    Credits,
    InGame,
}
```

### State Flow
```mermaid
stateDiagram-v2
    [*] --> Loading
    Loading --> MainMenu: Assets & Plugins Ready
    
    MainMenu --> VehicleSelection: Play
    MainMenu --> LoadGame: Load (Disabled if no save)
    MainMenu --> Settings: Settings
    MainMenu --> Credits: Credits
    MainMenu --> [*]: Quit (AppExit)
    
    VehicleSelection --> MainMenu: Escape / Back Button
    LoadGame --> MainMenu: Escape / Back Button
    Settings --> MainMenu: Escape / Back Button
    Credits --> MainMenu: Escape / Back Button
```

---

## 📂 Menu Module Hierarchy

The main menu subsystem is isolated under `crates/opts-game/src/menu/`:

| Module | File | Role & Implementation |
| :--- | :--- | :--- |
| **Module Root** | `menu/mod.rs` | Exposes public API and `MainMenuPlugin`. |
| **Plugin** | `menu/plugin.rs` | Registers lifecycle hooks (`OnEnter`, `OnExit`) and update systems conditioned to `in_state(AppState::MainMenu)`. |
| **Layout** | `menu/layout.rs` | Builds viewport hierarchy: background (`z=0`), content (`z=1`), floating controls (`z=2`). |
| **Entries** | `menu/entries.rs` | Spawns canonical entries (Play, Load, Settings, Credits, Quit), sets tab indices, and handles quit actions. |
| **Background** | `menu/background.rs` | Loads `<recent_map_path>/picture.jpg` into Bevy `Image` texture, overlays a contrast scrim computed from the picture and the theme (`menu_scrim_color`, text keeps 4.5:1 over bright and dark pictures), and falls back to the `Surface` role. |
| **Glass Surfaces** | `menu/glass.rs` | Manages `GlassSurface` component; `sync_glass_surfaces` system dynamically applies translucent alpha (0.25–0.80) from theme tokens. |
| **Interactions** | `menu/interactions.rs` | Focus ring visualizer (2px solid outline), hover/pressed state luminance adjustments, and [Enter]/[Space] keyboard button activation. |
| **Theme Toggle** | `menu/theme_toggle.rs` | Floating button (top-right) and [F2] hotkey flipping Dark/Light by changing `GameSettings::theme_mode`; `settings_theme::sync_theme_settings` moves it into the `opts-gui` resources. |
| **Systems** | `menu/systems.rs` | Setup/teardown triggers and initial keyboard focus management. |

---

## 🛠️ How to Add a Future Screen

Adding a new application screen (e.g. Vehicle Selection, Settings, Map Chooser) follows a standardized three-step workflow:

### Step 1: Declare or Target the `AppState` Variant
Ensure the target state is represented in `crates/opts-game/src/app_state.rs`:
```rust
pub enum AppState {
    ...
    VehicleSelection,
}
```

### Step 2: Implement the Screen Module & Plugin
Create a dedicated submodule under `crates/opts-game/src/<screen>/` (e.g. `vehicle_select/`):
```rust
use bevy::prelude::*;
use crate::app_state::AppState;

pub struct VehicleSelectPlugin;

impl Plugin for VehicleSelectPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(OnEnter(AppState::VehicleSelection), spawn_vehicle_select_ui)
           .add_systems(OnExit(AppState::VehicleSelection), cleanup_vehicle_select_ui)
           .add_systems(Update, vehicle_select_update.run_if(in_state(AppState::VehicleSelection)));
    }
}
```

### Step 3: Retire the Destination Stub & Register Plugin
1. In `crates/opts-game/src/stub_screens/mod.rs`, remove the stub registration for that state.
2. In `crates/opts-game/src/lib.rs`, add your new plugin to `OptsGamePlugin`:
```rust
app.add_plugins(VehicleSelectPlugin);
```

---

## 📦 Dependencies

| Dependency | Purpose |
| :--- | :--- |
| `opts-cli` | Command-line interface definitions and `CliArgs` parser. |
| `opts-logging` | Structured diagnostic logging and crash telemetry. |
| `opts-gui` | Material Design 3 widgets, design tokens, and focus subsystem. |
| `opts-bevy`, `opts-sim`, `opts-core` | Core simulation bridge and domain data models. |
| `bevy` | Engine, ECS, asset pipeline, UI layout, and window management. |
| `directories` | Standard OS user config path resolution (`~/.config/opts/`, `%APPDATA%\opts\`). |
| `toml` | Configuration serialization for `settings.toml`. |
| `image` | Dynamic runtime texture decoding for map `picture.jpg`. |
| `serde` | Struct serialization for settings. |

---

## 🚀 Application Startup & Wiring

Application startup is decoupled into composition (`build_app`) and execution:

- **Root Binary (`src/main.rs`):** The primary `opts` binary parses CLI flags via `opts_cli::CliArgs`, initializes `opts_logging`, builds the Bevy app, and executes `app.run()`.
- **Game Binary (`crates/opts-game/src/main.rs`):** Dedicated entrypoint for running the game package directly.
- **Composition Root (`crates/opts-game/src/app.rs`):** `build_app(args: CliArgs) -> App` builds a complete Bevy app configured with `WindowPlugin` (or headless runner), settings, assets, UI, and stub plugins.
- **Graceful Degradation (`crates/opts-game/src/stubs/`):** Missing subsystems (like `opts-discovery`) register lightweight stub plugins emitting non-blocking warnings and fallback resources.
- **Render options (`crates/opts-game/src/look.rs`, `settings_screen/render_choice.rs`):** two persisted settings in the Display & Sound tab. *Texture filtering* (`texture_filtering`: off/4/8/16, default 16) feeds `opts_bevy::set_texture_filtering`; the *colour look* (`look`: vivid/natural/bevy, default Vivid) is the `ActiveLook` resource, which `LookPlugin` applies live to the `Tonemapping`/`ColorGrading` of every `Camera3d` and which `sky.rs` reads for the ambient scale. Both default when the key is absent from `settings.db`. `--texture-filter` and `--look` override them for one run without saving; the control protocol exposes `render.look`.
- **Shutdown Persistence (`crates/opts-game/src/shutdown.rs`):** Intercepts `AppExit` events to persist `GameSettings` to disk prior to process termination.

---

## 🏃 Running the Application

```bash
# Launch default game window and main menu:
cargo run

# Launch via opts-game crate directly:
cargo run -p opts-game

# View all CLI flags and options:
cargo run -- --help

# Run headlessly in automated CI environments:
cargo run -- --headless

# Run with custom map background preview:
cargo run -- --map-path /path/to/omsi/maps/Berlin-Spandau
```

## Plan a drive and loading (M-0062)

`new_game_screen` is the four-step wizard (*Where, What, When, Review*) built on `opts_gui::patterns::master_detail`; the choices live in `NewGameSetup` and are remembered through `drive_history` (`PlannerHistory`). `thumbnails.rs` decodes map pictures off the main thread into a capped cache. `in_game::loading_ui`/`loading_pipeline` show real stages and the outcomes *Ready*, *Ready with N problems*, *Failed*; everything that went wrong is kept in `in_game::load_report::LoadReport`. Specification: [wizard and loading](../src/roadmap/phase-9/wizard-and-loading.md).
