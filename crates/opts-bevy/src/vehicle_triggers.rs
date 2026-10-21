//! Player input to vehicle script triggers (M-0045): key bindings and cockpit hot-spots.
//!
//! * **Keys** come from OMSI's `Inputs/keyboard.cfg` (`docs/src/formats/text/engine-config.md`):
//!   every `[vehicles]` entry names a script trigger. Pressing the key raises `<name>`; releasing
//!   a *held* binding (modifier bit 1) raises `<name>_off`. Bindings on keys OPTS itself uses
//!   ([`OPTS_RESERVED_KEYS`], unless they need Ctrl) are left out, so driving and camera keys
//!   keep working.
//! * **Hot-spots** are meshes with `[mouseevent] <name>`: a left click raises `<name>`, the first
//!   mouse movement while held raises `<name>_drag`, releasing raises `<name>_off`. While held,
//!   the system values `mouse_x`/`mouse_y` carry the mouse movement of the frame in pixels ❓.
//!   Hot-spots react in the cockpit and passenger views only (left-drag orbits the external
//!   camera).

use bevy::input::mouse::MouseMotion;
use bevy::prelude::*;
use opts_core::formats::engine_config::KeyboardConfig;

use crate::camera::{CameraMode, CameraState};
use crate::vehicle_script::VehicleScripts;
use crate::vehicle_sim::PlayerVehicle;

mod click;
mod hover;
mod pseudo;
pub use click::{scripted_click_system, ScriptedClick, ScriptedClicks};
pub use hover::{hover_system, HotSpotPicker, HoveredControl};
use pseudo::{toggle_cabin_light, toggle_door_side};

/// The pseudo trigger of the tram's door side selection: one key switches between the right and
/// the left side, where the scripts only have a toggle for each (`kw_s_1`, `kw_s_2`).
pub const DOOR_SIDE_TOGGLE: &str = "@door_side_toggle";

/// The pseudo trigger of the cabin light key: the cab light on or off, whatever the vehicle's
/// switch has (a bus's three-position switch off, auto, on; a tram's plain toggle).
pub const CABIN_LIGHT_TOGGLE: &str = "@cabin_light_toggle";

/// Keys OPTS binds in-game (driving, gears, cameras, overlay, reporting). OMSI bindings on these
/// keys are dropped unless they require Ctrl; a rebinding to one of them is refused (A, E, Q
/// and 6-9 are free: the steering is on the arrows, the free camera is gated off the vehicle).
pub const OPTS_RESERVED_KEYS: &[KeyCode] = &[
    KeyCode::KeyW,
    KeyCode::KeyS,
    KeyCode::KeyD,
    KeyCode::KeyN,
    KeyCode::KeyR,
    KeyCode::Space,
    // The viewpoint editor of the driver's view (Tab; Home resets inside it).
    KeyCode::Tab,
    KeyCode::ArrowUp,
    KeyCode::ArrowDown,
    KeyCode::ArrowLeft,
    KeyCode::ArrowRight,
    KeyCode::Digit0,
    KeyCode::Digit1,
    KeyCode::Digit2,
    KeyCode::Digit3,
    KeyCode::Digit4,
    KeyCode::Digit5,
    KeyCode::F1,
    KeyCode::F2,
    KeyCode::F8,
    // The mini map: F7 shows it, Shift+F7 turns it, + and - zoom. F9 is the quick load.
    KeyCode::F7,
    KeyCode::F9,
    KeyCode::Minus,
    KeyCode::Equal,
    // The door camera cycles its door with F4 (M-0109); the full-size map is F6 (M-0108; M stays
    // the engine start of OMSI).
    KeyCode::F4,
    KeyCode::F6,
    KeyCode::F10,
    KeyCode::F12,
    KeyCode::Escape,
];

/// The physical key of a DirectInput scan code (the numbering of `keyboard.cfg` and `.kyb`).
#[must_use]
pub fn dik_to_keycode(code: u16) -> Option<KeyCode> {
    use KeyCode as K;
    const LETTERS_Q: [KeyCode; 10] = [
        K::KeyQ,
        K::KeyW,
        K::KeyE,
        K::KeyR,
        K::KeyT,
        K::KeyY,
        K::KeyU,
        K::KeyI,
        K::KeyO,
        K::KeyP,
    ];
    const LETTERS_A: [KeyCode; 9] = [
        K::KeyA,
        K::KeyS,
        K::KeyD,
        K::KeyF,
        K::KeyG,
        K::KeyH,
        K::KeyJ,
        K::KeyK,
        K::KeyL,
    ];
    const LETTERS_Z: [KeyCode; 7] = [
        K::KeyZ,
        K::KeyX,
        K::KeyC,
        K::KeyV,
        K::KeyB,
        K::KeyN,
        K::KeyM,
    ];
    const DIGITS: [KeyCode; 10] = [
        K::Digit1,
        K::Digit2,
        K::Digit3,
        K::Digit4,
        K::Digit5,
        K::Digit6,
        K::Digit7,
        K::Digit8,
        K::Digit9,
        K::Digit0,
    ];
    const FKEYS: [KeyCode; 10] = [
        K::F1,
        K::F2,
        K::F3,
        K::F4,
        K::F5,
        K::F6,
        K::F7,
        K::F8,
        K::F9,
        K::F10,
    ];
    let c = usize::from(code);
    Some(match code {
        1 => K::Escape,
        2..=11 => DIGITS[c - 2],
        12 => K::Minus,
        13 => K::Equal,
        14 => K::Backspace,
        15 => K::Tab,
        16..=25 => LETTERS_Q[c - 16],
        26 => K::BracketLeft,
        27 => K::BracketRight,
        28 => K::Enter,
        29 => K::ControlLeft,
        30..=38 => LETTERS_A[c - 30],
        39 => K::Semicolon,
        40 => K::Quote,
        41 => K::Backquote,
        42 => K::ShiftLeft,
        43 => K::Backslash,
        44..=50 => LETTERS_Z[c - 44],
        51 => K::Comma,
        52 => K::Period,
        53 => K::Slash,
        54 => K::ShiftRight,
        55 => K::NumpadMultiply,
        56 => K::AltLeft,
        57 => K::Space,
        58 => K::CapsLock,
        59..=68 => FKEYS[c - 59],
        69 => K::NumLock,
        70 => K::ScrollLock,
        71 => K::Numpad7,
        72 => K::Numpad8,
        73 => K::Numpad9,
        74 => K::NumpadSubtract,
        75 => K::Numpad4,
        76 => K::Numpad5,
        77 => K::Numpad6,
        78 => K::NumpadAdd,
        79 => K::Numpad1,
        80 => K::Numpad2,
        81 => K::Numpad3,
        82 => K::Numpad0,
        83 => K::NumpadDecimal,
        86 => K::IntlBackslash,
        87 => K::F11,
        88 => K::F12,
        156 => K::NumpadEnter,
        157 => K::ControlRight,
        181 => K::NumpadDivide,
        183 => K::PrintScreen,
        184 => K::AltRight,
        197 => K::Pause,
        199 => K::Home,
        200 => K::ArrowUp,
        201 => K::PageUp,
        203 => K::ArrowLeft,
        205 => K::ArrowRight,
        207 => K::End,
        208 => K::ArrowDown,
        209 => K::PageDown,
        210 => K::Insert,
        211 => K::Delete,
        219 => K::SuperLeft,
        220 => K::SuperRight,
        221 => K::ContextMenu,
        _ => return None,
    })
}

/// A key that raises a script trigger.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScriptKeyBinding {
    /// Trigger name.
    pub trigger: String,
    /// Physical key.
    pub key: KeyCode,
    /// Shift must be held.
    pub shift: bool,
    /// Ctrl must be held.
    pub ctrl: bool,
    /// Releasing the key raises `<trigger>_off`.
    pub held: bool,
}

/// A key with the modifiers that must be held with it (the value of a key binding setting).
///
/// Stored as text: `none`, or an optional `Ctrl+` and `Shift+` followed by a protocol key name
/// (`L`, `7`, `F5`, `NumpadDivide`; see [`crate::control::keys::parse_key`]). Keys are physical,
/// so a binding keeps its position on other keyboard layouts.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct KeyChord {
    /// The physical key.
    pub key: KeyCode,
    /// Shift must be held.
    pub shift: bool,
    /// Ctrl must be held.
    pub ctrl: bool,
}

impl KeyChord {
    /// Parses `text`; `Ok(None)` is `none` (or empty: unbound).
    ///
    /// # Errors
    /// The message says what is wrong with `text`.
    pub fn parse(text: &str) -> Result<Option<Self>, String> {
        let text = text.trim();
        if text.is_empty() || text.eq_ignore_ascii_case("none") {
            return Ok(None);
        }
        let (mut shift, mut ctrl) = (false, false);
        let mut rest = text;
        while let Some((head, tail)) = rest.split_once('+') {
            if tail.is_empty() {
                break;
            }
            match head.trim().to_ascii_lowercase().as_str() {
                "shift" => shift = true,
                "ctrl" | "control" | "strg" => ctrl = true,
                other => return Err(format!("unknown modifier \"{other}\" (Ctrl, Shift)")),
            }
            rest = tail;
        }
        let rest = rest.trim();
        let Some((key, _)) = crate::control::keys::parse_key(rest) else {
            return Err(format!(
                "unknown key \"{rest}\" (A-Z, 0-9, F1-F12, Numpad0-9, NumpadDivide, ... or none)"
            ));
        };
        Ok(Some(Self { key, shift, ctrl }))
    }

    /// The canonical text (`Ctrl+Shift+L`), `None` if the key has no protocol name.
    #[must_use]
    pub fn to_text(self) -> Option<String> {
        let name = crate::control::keys::key_name(self.key)?;
        let mut out = String::new();
        if self.ctrl {
            out.push_str("Ctrl+");
        }
        if self.shift {
            out.push_str("Shift+");
        }
        out.push_str(&name);
        Some(out)
    }

    /// Whether OPTS itself uses this key combination (see [`OPTS_RESERVED_KEYS`]).
    #[must_use]
    pub fn is_reserved(self) -> bool {
        !self.ctrl && OPTS_RESERVED_KEYS.contains(&self.key)
    }
}

/// Key bindings of vehicle script triggers: the `keyboard.cfg` ones ([`ScriptKeyBase`]) plus the
/// rebindable actions of the settings (filled by the game; empty until it has run).
#[derive(Resource, Debug, Clone, Default)]
pub struct ScriptKeyBindings(pub Vec<ScriptKeyBinding>);

/// The bindings read from `keyboard.cfg` without the rebindable actions (their keys come from the
/// settings and win over an identical chord here).
#[derive(Resource, Debug, Clone, Default)]
pub struct ScriptKeyBase(pub Vec<ScriptKeyBinding>);

impl ScriptKeyBindings {
    /// `base` without the `replaced` triggers and without bindings on a chord that `own` uses,
    /// followed by `own`.
    #[must_use]
    pub fn compose(
        base: &[ScriptKeyBinding],
        replaced: &[&str],
        own: Vec<ScriptKeyBinding>,
    ) -> Self {
        let mut out: Vec<ScriptKeyBinding> = base
            .iter()
            .filter(|b| {
                !replaced.contains(&b.trigger.as_str())
                    && !own
                        .iter()
                        .any(|o| o.key == b.key && o.shift == b.shift && o.ctrl == b.ctrl)
            })
            .cloned()
            .collect();
        out.extend(own);
        Self(out)
    }
}

impl ScriptKeyBindings {
    /// The `[vehicles]` bindings of `config` whose key is known and not reserved by OPTS.
    #[must_use]
    pub fn from_config(config: &KeyboardConfig) -> Self {
        let bindings = config
            .vehicle_bindings()
            .filter_map(|b| {
                let key = dik_to_keycode(b.scan_code)?;
                if !b.ctrl() && OPTS_RESERVED_KEYS.contains(&key) {
                    tracing::debug!(trigger = %b.action, ?key, "OMSI key binding left out: key is used by OPTS");
                    return None;
                }
                Some(ScriptKeyBinding {
                    trigger: b.action.clone(),
                    key,
                    shift: b.shift(),
                    ctrl: b.ctrl(),
                    held: b.held(),
                })
            })
            .collect();
        Self(bindings)
    }

    /// Trigger names raised this frame, in binding order (`just_pressed` for presses with
    /// exactly the binding's modifiers, `just_released` for `_off`).
    #[must_use]
    pub fn fired(&self, keys: &ButtonInput<KeyCode>) -> Vec<String> {
        let shift = keys.any_pressed([KeyCode::ShiftLeft, KeyCode::ShiftRight]);
        let ctrl = keys.any_pressed([KeyCode::ControlLeft, KeyCode::ControlRight]);
        let mut out = Vec::new();
        for b in &self.0 {
            if keys.just_pressed(b.key) && b.shift == shift && b.ctrl == ctrl {
                out.push(b.trigger.clone());
            }
            if b.held && keys.just_released(b.key) {
                out.push(format!("{}_off", b.trigger));
            }
        }
        out
    }
}

/// A mesh part that raises a script trigger when clicked (`[mouseevent]`).
#[derive(Component, Debug, Clone)]
pub struct MouseTrigger {
    /// The vehicle root that owns the script engine.
    pub owner: Entity,
    /// Trigger name.
    pub trigger: String,
}

/// The engines triggers reach: the player's vehicle and the units of its consist that run scripts
/// of their own.
type Scripted = Or<(
    With<PlayerVehicle>,
    With<crate::consist_scripts::ScriptPart>,
)>;

/// Raises `name` on `owner`'s engine; logs at `debug` whether a script handled it.
fn raise(vehicles: &Query<&VehicleScripts, Scripted>, owner: Entity, name: &str) {
    let Ok(scripts) = vehicles.get(owner) else {
        return;
    };
    if let Ok(mut engine) = scripts.engine.lock() {
        let handled = engine.trigger(name);
        tracing::debug!(trigger = name, handled, "vehicle script trigger");
    }
}

/// Raises the key-bound triggers on the player's vehicle.
pub fn script_key_trigger_system(
    keys: Res<ButtonInput<KeyCode>>,
    bindings: Res<ScriptKeyBindings>,
    state: Option<Res<CameraState>>,
    vehicles: Query<(Entity, &VehicleScripts), With<PlayerVehicle>>,
) {
    if bindings.0.is_empty() {
        return;
    }
    // The free camera uses the keys for flying; the vehicle takes no input then.
    if state.is_some_and(|s| s.mode == CameraMode::FreeRoam) {
        return;
    }
    let fired = bindings.fired(&keys);
    if fired.is_empty() {
        return;
    }
    // Keys reach the head's scripts only: the cars behind it have cabs of their own that answer
    // the same triggers (the NF6D's `automatic_D` in car B marks its cab active and the head's
    // inactive); they hear the head through the control cables.
    for (_, scripts) in &vehicles {
        let Ok(mut engine) = scripts.engine.lock() else {
            continue;
        };
        for name in &fired {
            let handled = if name == DOOR_SIDE_TOGGLE {
                toggle_door_side(&mut engine)
            } else if name == CABIN_LIGHT_TOGGLE {
                toggle_cabin_light(&mut engine)
            } else {
                engine.trigger(name)
            };
            tracing::debug!(trigger = %name, handled, "vehicle script trigger (key)");
        }
    }
}

/// The hot-spot held with the left mouse button.
#[derive(Debug, Clone)]
pub struct HeldHotSpot {
    owner: Entity,
    trigger: String,
    dragged: bool,
}

/// Raises hot-spot triggers for left clicks on [`MouseTrigger`] meshes of the player's vehicle.
#[allow(clippy::too_many_arguments)]
pub fn script_mouse_trigger_system(
    mouse: Res<ButtonInput<MouseButton>>,
    mut motion: MessageReader<MouseMotion>,
    vehicles: Query<&VehicleScripts, Scripted>,
    mut picker: HotSpotPicker,
    driving: Option<Res<crate::mouse_driving::MouseDriving>>,
    mut held: Local<Option<HeldHotSpot>>,
) {
    let delta: Vec2 = motion.read().map(|m| m.delta).sum();
    if let Some(h) = held.as_mut() {
        if let Ok(scripts) = vehicles.get(h.owner) {
            if let Ok(mut engine) = scripts.engine.lock() {
                engine.set_system("mouse_x", delta.x);
                engine.set_system("mouse_y", delta.y);
            }
        }
        // `<name>_drag` is raised on every frame in which the mouse moves while the button is
        // held (the scripts add `mouse_x`/`mouse_y` to a position each time: the Citybus 530's
        // ignition key turns by `mouse_y / -75` per call, so a single call would barely move it).
        if delta != Vec2::ZERO {
            h.dragged = true;
            raise(&vehicles, h.owner, &format!("{}_drag", h.trigger));
        }
        if !mouse.pressed(MouseButton::Left) {
            if let Ok(scripts) = vehicles.get(h.owner) {
                if let Ok(mut engine) = scripts.engine.lock() {
                    engine.set_system("mouse_x", 0.0);
                    engine.set_system("mouse_y", 0.0);
                }
            }
            raise(&vehicles, h.owner, &format!("{}_off", h.trigger));
            *held = None;
        }
        return;
    }
    if !mouse.just_pressed(MouseButton::Left) {
        return;
    }
    // The cursor is the steering wheel while the mouse drives: it clicks nothing in the cockpit.
    if driving.is_some_and(|d| d.drives()) {
        return;
    }
    let Some(spot) = picker.under_cursor() else {
        return;
    };
    raise(&vehicles, spot.owner, &spot.trigger);
    *held = Some(HeldHotSpot {
        owner: spot.owner,
        trigger: spot.trigger.clone(),
        dragged: false,
    });
}

/// Registers the trigger systems; the game fills [`ScriptKeyBindings`].
#[derive(Default)]
pub struct VehicleTriggerPlugin;

impl Plugin for VehicleTriggerPlugin {
    fn build(&self, app: &mut App) {
        // The F1 report modal takes all input: typing must not raise vehicle triggers.
        app.init_resource::<ScriptKeyBindings>()
            .init_resource::<ScriptKeyBase>()
            .init_resource::<ScriptedClicks>()
            .init_resource::<HoveredControl>()
            .add_systems(
                Update,
                (
                    scripted_click_system,
                    hover_system
                        .run_if(crate::reporting::not_modal_active)
                        .run_if(crate::camera::not_editing_viewpoint),
                    script_key_trigger_system
                        .run_if(crate::reporting::not_modal_active)
                        .run_if(crate::camera::not_editing_viewpoint),
                    script_mouse_trigger_system
                        .run_if(crate::reporting::not_modal_active)
                        .run_if(crate::camera::not_editing_viewpoint),
                ),
            );
    }
}

#[cfg(test)]
mod tests;
