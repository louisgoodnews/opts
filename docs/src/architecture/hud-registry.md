# HUD Registry

Milestone [M-0064](../roadmap/phase-9/M-0064.md). Design: [expandability §3](../roadmap/phase-9/expandability.md#3-hud-registry),
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
| `protocol` | control methods `hud.state`, `hud.alert`, `hud.toast` |

## 2. Widgets

A `HudWidget` has an `id`, a message key for its name, a `Binding` (the simulation value: `Speed`,
`Gear`, `Rpm`, `ParkingBrake`, `SessionInfo`, `SessionClock`, `ControlsHint`, `Alerts`,
`CameraLabel`), a `Format`, a default `Slot` and the slots a profile may move it to, a `priority`,
an optional user `WidgetSwitch` and a `Condition` (vehicle kinds, needs an engine speed, camera
modes in which it is hidden).

| Widget | Slot | Switch (setting) | Notes |
| :--- | :--- | :--- | :--- |
| `speed` | Instruments | `hud.speed` | whole numbers, km/h or mph from `interface.units`, fixed width |
| `gear` | Instruments | `hud.gear` | `D`, `N`, `R` or the gear number |
| `rpm` | Instruments | `hud.rpm` | **hidden** (not "--") until the simulation provides an engine speed, see `RPM_PROVIDED` |
| `parking-brake` | Instruments | `hud.parking_brake` | word and colour (`ON` alarm, `OFF` normal) |
| `session` / `clock` | StatusBar | `hud.session_info` / `hud.clock` | fades after 8 s; pinned while the game is paused |
| `controls-hint` | Hints | `interface.hud_hints` | fades after 12 s; the older *Show hints* setting is its switch |
| `alerts` | Alerts | none (alerts keep the driver safe) | one alert at a time |
| `camera-label` | CameraLabel | `hud.camera_label` | 1.5 s after a camera or mode change |

The switches are ordinary settings of the [settings registry](settings-registry.md) (Interface page,
`settings.set`, reset, search): `settings_registry/builtin.rs` generates one descriptor per
`WidgetSwitch`, a test checks the two tables agree.

## 3. Profiles

```ron
HudProfile(
    kind: Tram,
    widgets: [
        (id: "speed", order: 10),
        (id: "parking-brake", order: 20),
        (id: "gear", order: 30),
        (id: "controls-hint", order: 10),
    ],
)
```

`kind` is `Bus`, `ArticulatedBus`, `Tram` or `Trolleybus`; `slot` (optional) must be one the widget
allows; `order` sorts inside a slot. Shipped files are `crates/opts-game/src/hud/profiles/*.ron`.
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
| Alerts | bottom centre above the cluster | a box of fixed height: an alert appears and leaves without shifting anything |
| Instruments | bottom centre | the cluster panel; a slot whose widgets are all switched off leaves the layout |
| Hints | bottom left | fades |

The instrument colours are the `MaterialTheme` instrument tokens (day and night palettes, contrast
tested over a bright and a dark scene in `opts-gui`); `InstrumentFade` multiplies their alpha for
the fades, so no colour literal exists in the HUD (a source test covers `hud/`).

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
alert; `hud.toast {message, details}` spawns a details toast. They exist because the HUD has no
focusable elements, so `ui.tree` alone cannot show what it says.

## 8a. Viewpoint panel

The viewpoint editor of the driver's view (Tab, M-0084) draws its panel in this look (`hud/viewpoint.rs`): instrument tokens, `PointerOnly` buttons, spawned only while the editor is open, no focus taken. See [camera system](camera-system.md).

## 9. Not done yet

- The *Help and controls* overlay generated from the bindings and the vehicle type, and *Pause → Vehicle*
  (pause wiring belongs to the pause work): `help.controls-generated` stays a "not available yet" place.
- Size classes per widget (compact windows): the descriptors carry a `priority`, no system uses it yet.
- Script variables as bindings (a widget bound to a vehicle script variable by name).
- Free-form layouts (the design defers them).
- Measurement of glance time (usability test of M-0065) and the HUD review checklist's sign-off.
