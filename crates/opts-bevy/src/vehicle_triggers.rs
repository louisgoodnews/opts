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
use bevy::picking::mesh_picking::ray_cast::{MeshRayCast, MeshRayCastSettings};
use bevy::prelude::*;
use bevy::window::PrimaryWindow;
use opts_core::formats::engine_config::KeyboardConfig;

use crate::camera::{CameraMode, CameraState};
use crate::vehicle_script::VehicleScripts;
use crate::vehicle_sim::PlayerVehicle;

/// Keys OPTS binds in-game (driving, gears, cameras, overlay, reporting). OMSI bindings on these
/// keys are dropped unless they require Ctrl.
pub const OPTS_RESERVED_KEYS: &[KeyCode] = &[
    KeyCode::KeyW,
    KeyCode::KeyA,
    KeyCode::KeyS,
    KeyCode::KeyD,
    KeyCode::KeyE,
    KeyCode::KeyQ,
    KeyCode::KeyN,
    KeyCode::KeyR,
    KeyCode::Space,
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
    KeyCode::Digit6,
    KeyCode::Digit7,
    KeyCode::Digit8,
    KeyCode::Digit9,
    KeyCode::F1,
    KeyCode::F2,
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

/// Key bindings of vehicle script triggers (empty until the game loads `keyboard.cfg`).
#[derive(Resource, Debug, Clone, Default)]
pub struct ScriptKeyBindings(pub Vec<ScriptKeyBinding>);

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

/// Raises `name` on `owner`'s engine; logs at `debug` whether a script handled it.
fn raise(vehicles: &Query<&VehicleScripts, With<PlayerVehicle>>, owner: Entity, name: &str) {
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
    vehicles: Query<(Entity, &VehicleScripts), With<PlayerVehicle>>,
) {
    if bindings.0.is_empty() {
        return;
    }
    let fired = bindings.fired(&keys);
    if fired.is_empty() {
        return;
    }
    for (_, scripts) in &vehicles {
        let Ok(mut engine) = scripts.engine.lock() else {
            continue;
        };
        for name in &fired {
            let handled = engine.trigger(name);
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
    state: Option<Res<CameraState>>,
    windows: Query<&Window, With<PrimaryWindow>>,
    cameras: Query<(&Camera, &GlobalTransform), With<Camera3d>>,
    hot_spots: Query<&MouseTrigger>,
    vehicles: Query<&VehicleScripts, With<PlayerVehicle>>,
    mut ray_cast: MeshRayCast,
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
        if delta != Vec2::ZERO && !h.dragged {
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
    if !state.is_some_and(|s| matches!(s.mode, CameraMode::Cockpit | CameraMode::Passenger)) {
        return;
    }
    let Some(cursor) = windows.single().ok().and_then(Window::cursor_position) else {
        return;
    };
    let Some(ray) = cameras
        .iter()
        .find(|(c, _)| c.is_active)
        .and_then(|(c, t)| c.viewport_to_world(t, cursor).ok())
    else {
        return;
    };
    let filter = |e: Entity| hot_spots.get(e).is_ok_and(|h| vehicles.contains(h.owner));
    let settings = MeshRayCastSettings::default().with_filter(&filter);
    let Some((entity, _)) = ray_cast.cast_ray(ray, &settings).first() else {
        return;
    };
    let Ok(spot) = hot_spots.get(*entity) else {
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
        app.init_resource::<ScriptKeyBindings>().add_systems(
            Update,
            (
                script_key_trigger_system.run_if(crate::reporting::not_modal_active),
                script_mouse_trigger_system.run_if(crate::reporting::not_modal_active),
            ),
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn config() -> KeyboardConfig {
        KeyboardConfig::parse(
            b"[vehicles]\n[entry]\nhorn\n35\n1\n[entry]\nbus_doorfront0\n181\n0\n[entry]\nautomatic_D\n32\n0\n[entry]\nIBIS_1\n79\n4\n[entry]\nkw_s_2\n3\n4\n",
        )
    }

    #[test]
    fn scan_codes_map_to_physical_keys() {
        assert_eq!(dik_to_keycode(25), Some(KeyCode::KeyP));
        assert_eq!(dik_to_keycode(44), Some(KeyCode::KeyZ));
        assert_eq!(dik_to_keycode(48), Some(KeyCode::KeyB));
        assert_eq!(dik_to_keycode(181), Some(KeyCode::NumpadDivide));
        assert_eq!(dik_to_keycode(2), Some(KeyCode::Digit1));
        assert_eq!(dik_to_keycode(11), Some(KeyCode::Digit0));
        assert_eq!(dik_to_keycode(68), Some(KeyCode::F10));
        assert_eq!(dik_to_keycode(500), None);
    }

    #[test]
    fn reserved_keys_are_dropped_unless_ctrl_is_needed() {
        let b = ScriptKeyBindings::from_config(&config());
        let names: Vec<_> = b.0.iter().map(|b| b.trigger.as_str()).collect();
        // automatic_D sits on D (steering); kw_s_2 needs Ctrl and stays.
        assert_eq!(names, ["horn", "bus_doorfront0", "IBIS_1", "kw_s_2"]);
    }

    #[test]
    fn press_and_release_raise_triggers() {
        let b = ScriptKeyBindings::from_config(&config());
        let mut keys = ButtonInput::<KeyCode>::default();
        keys.press(KeyCode::KeyH);
        keys.press(KeyCode::NumpadDivide);
        assert_eq!(b.fired(&keys), ["horn", "bus_doorfront0"]);
        keys.clear();
        keys.release(KeyCode::KeyH);
        keys.release(KeyCode::NumpadDivide);
        // Only the held binding reports its release.
        assert_eq!(b.fired(&keys), ["horn_off"]);
    }

    #[test]
    fn modifiers_must_match_exactly() {
        let b = ScriptKeyBindings::from_config(&config());
        let mut keys = ButtonInput::<KeyCode>::default();
        keys.press(KeyCode::Numpad1);
        assert!(b.fired(&keys).is_empty());
        keys.clear();
        keys.release(KeyCode::Numpad1);
        keys.clear();
        keys.press(KeyCode::ControlLeft);
        keys.press(KeyCode::Numpad1);
        assert_eq!(b.fired(&keys), ["IBIS_1"]);
    }
}
