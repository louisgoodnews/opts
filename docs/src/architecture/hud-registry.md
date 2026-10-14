# HUD Registry

Milestones [M-0064](../roadmap/phase-9/M-0064.md) (registry, slots) and [M-0071](../roadmap/phase-9/M-0071.md) (bottom-left cluster, mini map). Design: [expandability §3](../roadmap/phase-9/expandability.md#3-hud-registry),
[design language §9](../roadmap/phase-9/design-language.md#9-the-instrument-and-hud-look),
[interaction §8](../roadmap/phase-9/interaction-and-feel.md#8-in-game-feel) (principle UI-P13: driving-safe).
Code: `crates/opts-game/src/hud/`.

**Goal:** the HUD is *data* binding simulation values to widgets in slots. A new widget is one
`HudWidget` descriptor and a profile entry; the HUD systems do not change. Third parties can ship
profiles as data (RON), never code.

## 1. Pieces

| Module | Role |
| :--- | :--- |
| `widget` | `Slot`, `VehicleKind`, `Binding`, `Format`, `Condition`, `WidgetSwitch` and the `WIDGETS` table of `HudWidget` descriptors |
| `profile` | `HudProfile` (RON), validation, `HudProfiles` (shipped `profiles/<kind>.ron` plus user overrides) |
| `values` | `HudValues`, sampled once per frame from the player's vehicle and the camera |
| `build` / `parts` | spawn the HUD of the vehicle's profile; markers and building blocks (all colours from the instrument tokens) |
| `update` | change-driven texts, display by switch and condition, alert line, camera label |
| `alerts` | `RaiseAlert` / `ClearAlert`, `AlertQueue`, the parking-brake alert |
| `fade` | `HudFade`: status bar, hints and the 1.5 s camera label |
| `details` | `ShowDetailsToast`: toast with a *Details* dialog (Copy, Report) |
| `layout` | `HudLayout`: compact or normal, map size, margins, from the window and the UI scale; raises the toast stack above the map |
| `minimap` | the mini map: `raster` (pure drawing), `source` (read-only roads), settings, keys, 10 Hz redraw |
| `viewpoint` | the viewpoint editor panel (M-0084) |
| `protocol` | control methods `hud.state`, `hud.alert`, `hud.toast`, `hud.minimap` |

## 2. Widgets

A `HudWidget` has an `id`, a message key for its name, a `Binding` (the simulation value: `Speed`,
`Gear`, `Rpm`, `ParkingBrake`, `SessionInfo`, `SessionClock`, `ControlsHint`, `Alerts`,
`CameraLabel`, `MiniMap`), a `Format`, a default `Slot` and the slots a profile may move it to, a `priority`,
an optional user `WidgetSwitch` and a `Condition` (vehicle kinds, needs an engine speed, camera
modes in which it is hidden).

| Widget | Slot (row) | Switch (setting) | Notes |
| :--- | :--- | :--- | :--- |
| `speed` | ClusterBottomLeft (instruments) | `hud.speed` | whole numbers, km/h or mph from `interface.units`, fixed width |
| `gear` | ClusterBottomLeft (instruments) | `hud.gear` | `D`, `N`, `R` or the gear number |
| `rpm` | ClusterBottomLeft (instruments) | `hud.rpm` | **hidden** (not "--") until the simulation provides an engine speed, see `RPM_PROVIDED` |
| `parking-brake` | ClusterBottomLeft (instruments) | `hud.parking_brake` | word and colour (`ON` alarm, `OFF` normal) |
| `session` / `clock` | StatusBar | `hud.session_info` / `hud.clock` | fades after 8 s; pinned while the game is paused |
| `controls-hint` | ClusterBottomLeft (hints) | `interface.hud_hints` | fades after 12 s; the older *Show hints* setting is its switch; a short text in a compact layout |
| `alerts` | ClusterBottomLeft (alerts) | none (alerts keep the driver safe) | one alert at a time |
| `camera-label` | CameraLabel | `hud.camera_label` | 1.5 s after a camera or mode change |
| `minimap` | MiniMap | `hud.minimap` (F7) | roads and vehicles around the player, see section 4a; also `hud.minimap_zoom` and `hud.minimap_orientation` |

The switches are ordinary settings of the [settings registry](settings-registry.md) (Interface page,
`settings.set`, reset, search): `settings_registry/builtin.rs` generates one descriptor per
`WidgetSwitch`, a test checks the two tables agree.

## 3. Profiles

```ron
HudProfile(
    kind: Tram,
    widgets: [
        (id: "controls-hint", order: 10),
        (id: "alerts", order: 20),
        (id: "speed", order: 30),
        (id: "parking-brake", order: 40),
        (id: "gear", order: 50),
        (id: "minimap", order: 10),
    ],
)
```

`kind` is `Bus`, `ArticulatedBus`, `Tram` or `Trolleybus`; `slot` (optional) must be one the widget
allows; `order` sorts inside a slot (inside the cluster: inside its instrument row). Profiles written before M-0071 may still say `slot: Some(Instruments)`, `Hints` or `Alerts`: all three are read as `ClusterBottomLeft`. Shipped files are `crates/opts-game/src/hud/profiles/*.ron`.
A file of the same name in `<config dir>/hud/` replaces the shipped one; a file with a syntax error
or for another kind is ignored with a warning, entries with an unknown widget, a duplicate or a
disallowed slot are dropped with a warning. The game never fails on a profile.

The kind of the player's vehicle comes from the world: a `RailPlayer` is a tram, an
`ArticulatedChain` an articulated bus, otherwise a bus. **A trolleybus cannot be recognised** (the
vehicle data has no such field), so its profile exists but is not selected automatically.

## 4. Slots and layout

Fixed regions; nothing is in the middle of the view and nothing moves.

| Slot | Place | Behaviour |
| :--- | :--- | :--- |
| StatusBar | top left | fades, pinned while paused, back for 8 s after a resume |
| (pause button) | top right | chrome, `PointerOnly`: clickable, never focused |
| CameraLabel | top centre below the bar | hidden until the camera changes, then 1.5 s (1.2 s plus a 0.3 s fade); with reduced motion 1.5 s without fading |
| ClusterBottomLeft | bottom left | three rows in a **fixed order**, top to bottom: *hints* (fade), *alerts* (a minimum height: an alert appears and leaves without shifting anything), *instruments* (speed, gear, engine speed, parking brake; the profile orders these). A row whose widgets are all off leaves the layout; the cluster is as wide as its content but never reaches the map |
| MiniMap | bottom right | a square panel (26 % of the layout height, 112 to 224 px) with the map and one caption line (radius and what points up) |

**Responsive layout** (`hud::layout`): the layout area is the window divided by the UI scale. Below 760 x 420 it is *compact*: smaller margins (12 instead of 20), narrower readouts, the short hint text, the smallest map. That is what keeps 1024 x 640 at a UI scale of 200 % (a layout area of 512 x 320) free of overlaps. Toasts also live at the bottom right: `layout::toast_inset` raises the toast stack (`opts_gui` `ToastInset`) above the map while the map is shown. The viewpoint editor's panel sits at the right edge from 68 px below the top; if it still reaches the map (low windows, large UI scale) the map is hidden, not removed, while the editor is open. `tests/hud_matrix.rs` checks all of this from 1024 x 640 to 2560 x 1440 at 75 to 200 %.

The instrument colours are the `MaterialTheme` instrument tokens (day and night palettes, contrast
tested over a bright and a dark scene in `opts-gui`); `InstrumentFade` multiplies their alpha for
the fades, so no colour literal exists in the HUD (a source test covers `hud/`).

## 4a. The mini map (M-0071)

Code: `hud/minimap/`. A top-down **vector** drawing of the roads around the player, redrawn into one RGBA texture (`ImageNode`) at most **10 times per second** (`UPDATE_INTERVAL`); the budget of a redraw is **1 ms** (`BUDGET_US`; a test draws 1500 chords at 448 px and `hud.state` reports `last_us` and `worst_us` live).

| Part | What |
| :--- | :--- |
| Source | `MapSource::roads_near(centre, radius, out)`: a read-only list of road paths (straight or arc, width). The traffic crate's road paths implement it (`TrafficState` in `omsi_world/traffic.rs`: the lane segments of the resident tiles); `FixtureRoads` implements it for tests and the golden screenshot. The map never reads the tile or terrain code, so changes there do not touch it. Rails and terrain are **not** drawn. |
| Centre | the active vehicle (`PlayerVehicle`); without a vehicle (map-only session) the free camera, which then is also the marker |
| Markers | every session vehicle: the active one is a *filled* arrow with a ring in the `text` token, the others are *hollow* arrows in the `warning` token (shape and lightness differ, never red against green); a mark on the edge shows where north is |
| Zoom | the visible radius: 100, 200, 500 or 1000 m (setting `hud.minimap_zoom`, default 200); the keys `+` and `-` step through them. With `hud.minimap_zoom_mode` = `speed` (default; Ctrl+F7 toggles) that step is the *largest* radius and the map zooms in to a third of it (at least 40 m) when slow or stopped; see [the mini map](minimap.md) |
| Orientation | `hud.minimap_orientation`: `heading` (default, the vehicle's heading is up) or `north`; Shift+F7 switches |
| Off | `hud.minimap` (F7); the widget then leaves the layout and nothing is drawn |
| Colours | all from the instrument tokens; the **ground is opaque** (`panel` without transparency) so the map reads over any scene, and every drawing colour has at least 3:1 against it in the day and the night palette (test) |

The keys write the settings (`SettingsApi`), so the page, the keys, `settings.set` and `hud.minimap` agree and the choice is remembered. F7, `+`, `-` are in `OPTS_RESERVED_KEYS` (OMSI key bindings on them are dropped unless they need Ctrl) and in the Help controls table (`help::controls::GROUPS`, group *Mini map*).

## 5. Alerts (UI-P13)

Sources send `RaiseAlert { id, severity, key, args }` and `ClearAlert(id)`. The queue shows the most
severe, newest alert. **Critical** alerts stay until their source clears them; information and
warnings leave after 6 s. The line starts with a word (*Note*, *Warning*, *Alert*), colour is never
the only signal. No sound, no modal, no focus. Built-in source: the parking brake held on while
moving (warning above 3 km/h, critical above 15 km/h).

## 6. Details dialog

`ShowDetailsToast { severity, message, details }` spawns a toast with a *Details* action. The
dialog shows the whole text; **Copy** puts it on the system clipboard (the dialog stays open and a
toast says *Copied*), **Report** opens the report dialog (screenshot and log, as in Help), **Close**
is first and has the focus; Esc closes. Producers will use it for lists of problems; today the control
method `hud.toast` and the tests produce it (the loading screen keeps its own *minor problems*
dialog).

## 7. Developer panel

The F10 block is off by default. `WorldStats.overlay_visible` follows the setting
`advanced.developer_panel` (*Settings → Advanced*); F10 changes that **setting** (through
`SettingsApi`, so the page, the key and the saved value agree). It is painted with the instrument
tokens and sits below the status bar.

## 8. Control protocol

`hud.state` returns the vehicle kind, the widgets with their visibility, every text, the fades, the
visible alert and the developer panel; `hud.alert {severity}` / `{clear: true}` raise and clear a test
alert; `hud.toast {message, details}` spawns a details toast; `hud.minimap {zoom?, orientation?, enabled?}` reads or changes the mini map (the same object is `minimap` in `hud.state`: enabled, shown, zoom, orientation, texture size, redraw count and times, number of chords and markers, centre and heading). They exist because the HUD has no
focusable elements, so `ui.tree` alone cannot show what it says.

## 8a. Viewpoint panel

The viewpoint editor of the driver's view (Tab, M-0084) draws its panel in this look (`hud/viewpoint.rs`): instrument tokens, `PointerOnly` buttons, spawned only while the editor is open, no focus taken. See [camera system](camera-system.md).

## 9. Not done yet

- The *Help and controls* overlay generated from the bindings and the vehicle type, and *Pause → Vehicle*
  (pause wiring belongs to the pause work): `help.controls-generated` stays a "not available yet" place.
- Size classes per widget: the compact layout is one switch for the whole HUD; the descriptors carry a `priority` no system uses yet.
- Mini map: rails, terrain, water, buildings and AI traffic are not drawn; no route line; no pan.
- Script variables as bindings (a widget bound to a vehicle script variable by name).
- Free-form layouts (the design defers them).
- Measurement of glance time (usability test of M-0065) and the HUD review checklist's sign-off.
