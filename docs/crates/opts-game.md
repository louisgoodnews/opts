# `opts-game`

| Metadata | Value |
| :--- | :--- |
| **Layer** | L6 · Application |
| **Role** | Application Composition Root & Consumer UI Screens |
| **Kind** | Binary (`opts-game`) + Library (`opts_game`) |
| **Status** | 🟢 Implemented (application shell, Home, first run, pause, Help; M-0062 shell part) |
| **Last Updated** | 2026-10-12 |

---

## 🎯 Purpose

`opts-game` is the primary executable and presentation orchestrator for the Open Public Transport Simulator. It provides:
1. Top-level application initialization, logging setup, and window configuration.
2. The global lifecycle state machine (`AppState`).
3. The **application shell** (navigation rail, screen registry, back stack, Esc, dialogs, banners, notifications) and its pages: **Home** (Continue, Quick drive, recent drives), **Help** (controls, guides, report, About), the **first-run** flow, the **pause overlay** and the **session summary**.
4. Screen routing through the registry: `Navigate` messages, deep links (`screen:settings/graphics`), the control-protocol command `ui.navigate`.
5. User settings persistence (`GameSettings`) via platform standard paths (`settings.toml` baseline, `settings.db` authoritative). Theme and interface keys (`theme_mode`, `auto_light_from`, `auto_dark_from`, `high_contrast`, `active_theme`, `theme_file`, `ui_scale`, `density`, `reduced_motion`) and the migration from `is_dark_theme` are in `settings_theme.rs`; RON theme files in `theme_files.rs`; the design is in the [GUI design system](../src/architecture/gui-design-system.md#64-settings-keys-and-migration).

---

## 🏗️ Architecture & State Machine

Navigation and simulation execution are governed deterministically by the Bevy `AppState` machine:

```rust
#[derive(States, Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum AppState {
    #[default]
    Loading,
    MainMenu,         // Home
    VehicleSelection, // Drive (Plan a drive)
    Settings,
    Help,
    InGame,
}
```

### State Flow
```mermaid
stateDiagram-v2
    [*] --> Loading
    Loading --> MainMenu: Assets & Plugins Ready
    MainMenu --> VehicleSelection: New drive / rail
    MainMenu --> Settings: rail
    MainMenu --> Help: rail / F1
    MainMenu --> InGame: Continue / Quick drive / recent drive
    VehicleSelection --> InGame: Launch
    InGame --> MainMenu: Leave drive (Esc, Pause, Leave drive, Leave to Home)
    VehicleSelection --> MainMenu: Esc / Back
    Settings --> MainMenu: Esc / Back
    Help --> MainMenu: Esc / Back
```

Esc never quits and never leaves a drive without the pause overlay and one confirmation; the rest of the rules are in [screens](../src/roadmap/phase-9/screens.md).

---

## 📂 Shell Module Hierarchy

| Module | Role |
| :--- | :--- |
| `shell/registry.rs` | `ScreenDescriptor`, `ScreenRegistry`, `App::register_screen`; the built-in destinations |
| `shell/frame.rs` | rail (icons only in *Compact*), banner area, content area (`ShellContent`), theme toggle (F2) and *Quit OPTS* in the footer |
| `shell/nav.rs` | `Navigate`, `NavigateBack`, `BackStack`, `PendingDeepLink`, `NavigationRefused` |
| `shell/esc.rs` | the one implementation of Esc / Back (`EscGate`; `ScreenBackSteps` / `ScreenBack` for a screen's own inner steps, used by *Plan a drive*) |
| `shell/dialog.rs`, `shell/banner.rs`, `shell/notify.rs` | confirmation dialogs, banners, toasts with a required durable place for errors |
| `shell/keys.rs`, `shell/l10n/` | message keys and the English and German tables (merged with `Localization::extend`) |
| `home/` | Home page (`view`, `model`, `hero` with the `cover` crop, `StartDrive`) |
| `first_run/` | confirm or choose the OMSI folder (`choose_omsi_folder` is the function Settings calls), catalogue stage, welcome |
| `pause/` | pause overlay, leave confirmation, `SessionStats` |
| `session_summary/` | the end-of-drive card on Home |
| `help/` | Help page, controls table, About, overlay in a drive, F1 |
| `hud/` | HUD registry: widgets, RON profiles per vehicle kind, slots, alerts, camera label, Details dialog ([design](../src/architecture/hud-registry.md)) |
| `menu/` | what is left of the old menu: `glass` tint and `background` (map picture with its computed scrim) |
| `isolation.rs` | `isolate_user_dirs()` keeps tests out of `~/.config/opts` |

Specifications: [screens](../src/roadmap/phase-9/screens.md), [Home and shell](../src/architecture/main-menu.md), [startup](../src/architecture/application-startup.md).

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

### Step 3: Register the Screen
1. Register the destination with `app.register_screen(ScreenDescriptor { id, title_key, icon, group, order, availability, state })`; the rail follows. Build the page with `spawn_screen_root` (it is placed in the shell's content area) and keep Esc out of it: the shell owns it (a screen with inner steps sets `ScreenBackSteps` and answers `ScreenBack`).
2. In `crates/opts-game/src/lib.rs`, add your plugin to `OptsGamePlugin`. A feature that does not work yet is registered as a not-yet place (`opts_gui::not_yet`) and carries the red marker.

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
