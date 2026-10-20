//! The rebindable vehicle key actions: OMSI's `keyboard.cfg` vehicle triggers that matter for
//! driving a bus or tram (battery, engine start, lights, indicators, hazard lights, doors, horn,
//! interior lights, tickets), each with a setting `controls.vehicle.<id>` that holds the key.
//!
//! The default keys are OMSI's own (`Inputs/keyboard.cfg`, see `docs/src/formats/text/engine-config.md`
//! and `docs/src/simulation/vehicle-keys.md`). The settings are the only source for these actions:
//! the key raises the script trigger on the player's vehicle (`opts_bevy::vehicle_triggers`), and
//! the other `keyboard.cfg` vehicle bindings (IBIS, destination blinds, cash desk) stay as the
//! installation has them. A binding is refused when another action or a key OPTS uses itself has
//! the same chord ([`conflict`]).

use bevy::prelude::*;
use opts_bevy::{KeyChord, ScriptKeyBase, ScriptKeyBinding, ScriptKeyBindings};

use crate::settings::GameSettings;
use crate::settings_registry::{Category, SettingDescriptor, SettingKind, SettingsRegistry};

/// One rebindable vehicle key action.
#[derive(Debug, Clone, Copy)]
pub struct VehicleKeyAction {
    /// Short name (`lights`), the argument of `vehicle.key`.
    pub id: &'static str,
    /// The setting that holds the key.
    pub setting: &'static str,
    /// Message key of the label .
    pub label_key: &'static str,
    /// Message key of the description.
    pub desc_key: &'static str,
    /// The script trigger the key raises.
    pub trigger: &'static str,
    /// OMSI's default key, as setting text.
    pub default: &'static str,
    /// Releasing the key raises `<trigger>_off`.
    pub held: bool,
    /// Which vehicles the key is for: a bus's door keys do nothing on a tram and the tram's
    /// side selection shares its key with the bus's second door.
    pub scope: Scope,
    /// A second trigger the same key raises (vehicles name the same function differently).
    pub also: Option<&'static str>,
}

/// The vehicles a key action is for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Scope {
    /// Buses, trolleybuses and trams.
    Both,
    /// Buses and trolleybuses.
    Bus,
    /// Trams.
    Tram,
}

impl Scope {
    /// Whether an action of this scope applies to a vehicle (`tram`: it runs on rails).
    #[must_use]
    pub fn applies(self, tram: bool) -> bool {
        match self {
            Self::Both => true,
            Self::Bus => !tram,
            Self::Tram => tram,
        }
    }

    /// Whether two scopes can be in use on the same vehicle, so their keys must differ.
    #[must_use]
    pub fn overlaps(self, other: Self) -> bool {
        self == Self::Both || other == Self::Both || self == other
    }
}

macro_rules! action {
    ($id:literal, $trigger:expr, $default:literal, $held:literal) => {
        action!($id, $trigger, $default, $held, Scope::Both, None)
    };
    ($id:literal, $trigger:expr, $default:literal, $held:literal, $scope:expr, $also:expr) => {
        VehicleKeyAction {
            id: $id,
            setting: concat!("controls.vehicle.", $id),
            label_key: concat!("set-vk-", $id),
            desc_key: concat!("set-vk-", $id, "-desc"),
            trigger: $trigger,
            default: $default,
            held: $held,
            scope: $scope,
            also: $also,
        }
    };
}

/// The actions, in the order of the settings page.
pub const ACTIONS: &[VehicleKeyAction] = &[
    action!("battery", "cp_batterietrennschalter_toggle", "E", true),
    action!("engine_start", "kw_m_enginestart", "M", true),
    action!("lights", "kw_scheinwerfer_toggle", "L", false),
    action!("parking_light", "kw_standlicht_toggle", "Shift+L", false),
    action!("high_beam", "kw_fernlicht_toggle", "F", false),
    action!("fog_rear", "taster_nebelschluss", "Ctrl+F", false),
    action!("indicator_left", "blinker_left_set", "Numpad7", true),
    action!("indicator_right", "blinker_right_set", "Numpad9", true),
    action!("indicator_off", "blinker_off", "NumpadDecimal", false),
    action!("hazard", "blinker_warn_toggle", "B", false),
    action!("horn", "horn", "H", true),
    action!("wiper", "kw_wipermode_up", "none", false),
    // Buses: the doors one by one and all at once; the trams only clear the doors and choose the
    // side (`bus_doorfront0/1` of a tram are its open and close buttons, which stay on the mouse).
    action!(
        "door_1",
        "bus_doorfront0",
        "NumpadDivide",
        false,
        Scope::Bus,
        None
    ),
    action!(
        "door_2",
        "bus_doorfront1",
        "NumpadMultiply",
        false,
        Scope::Bus,
        None
    ),
    action!(
        "door_3",
        "bus_doorfront2",
        "Ctrl+NumpadDivide",
        false,
        Scope::Bus,
        None
    ),
    action!(
        "door_4",
        "bus_doorfront3",
        "Ctrl+NumpadMultiply",
        false,
        Scope::Bus,
        None
    ),
    action!(
        "door_clearance",
        "bus_dooraft",
        "NumpadSubtract",
        false,
        Scope::Both,
        None
    ),
    action!(
        "door_central",
        "bus_doorfront4",
        "NumpadAdd",
        false,
        Scope::Bus,
        Some("bus_doorfront5")
    ),
    action!(
        "door_side",
        opts_bevy::DOOR_SIDE_TOGGLE,
        "NumpadMultiply",
        false,
        Scope::Tram,
        None
    ),
    action!(
        "cabin_light",
        opts_bevy::CABIN_LIGHT_TOGGLE,
        "Ctrl+L",
        false,
        Scope::Both,
        None
    ),
    action!("interior_driver", "cp_fahrerlicht_toggle", "6", false),
    action!(
        "interior_lower_right",
        "cp_licht_untenrechts_toggle",
        "7",
        false
    ),
    action!("interior_upper", "cp_licht_oberdeck_toggle", "8", false),
    action!("interior_lower", "cp_licht_unterdeck_toggle", "9", false),
    action!("microphone", "cp_microphone", "Q", true),
    action!("ticket", "ticket_give", "T", false),
    action!("ticket_change", "change_give", "Ctrl+T", false),
    action!("ticket_take", "change_take", "Shift+T", false),
];

/// The action with this id or this setting key.
#[must_use]
pub fn action(id_or_setting: &str) -> Option<&'static VehicleKeyAction> {
    ACTIONS
        .iter()
        .find(|a| a.id == id_or_setting || a.setting == id_or_setting)
}

/// The action that raises `trigger`.
#[must_use]
pub fn action_of_trigger(trigger: &str) -> Option<&'static VehicleKeyAction> {
    ACTIONS.iter().find(|a| a.trigger == trigger)
}

/// The settings, one `Binding` row each, in the Controls category.
#[must_use]
pub fn descriptors() -> Vec<SettingDescriptor> {
    ACTIONS
        .iter()
        .enumerate()
        .map(|(i, a)| {
            SettingDescriptor::new(
                a.setting,
                SettingKind::Binding,
                a.default,
                Category::Controls,
                100 + i as u16,
                a.label_key,
                a.desc_key,
            )
            .keywords(&["key", "keyboard", "binding", "vehicle", "bus", "tram"])
        })
        .collect()
}

/// Why `chord` cannot be the key of `setting`: a key OPTS uses itself, or another action's key.
#[must_use]
pub fn conflict(
    registry: &SettingsRegistry,
    settings: &GameSettings,
    setting: &str,
    chord: KeyChord,
) -> Option<String> {
    if chord.is_reserved() {
        return Some(format!(
            "{} is used by driving, the cameras or the overlays and cannot be bound (Ctrl+ combinations can)",
            chord.to_text().unwrap_or_default()
        ));
    }
    let scope = action(setting).map_or(Scope::Both, |a| a.scope);
    for a in ACTIONS
        .iter()
        .filter(|a| a.setting != setting && a.scope.overlaps(scope))
    {
        let other = registry.text(settings, a.setting).unwrap_or_default();
        if KeyChord::parse(&other).ok().flatten() == Some(chord) {
            return Some(format!(
                "{} is already the key of {} ({})",
                chord.to_text().unwrap_or_default(),
                a.id,
                a.setting
            ));
        }
    }
    None
}

/// The chord that a setting holds for `a` now.
#[must_use]
pub fn chord_of(
    registry: &SettingsRegistry,
    settings: &GameSettings,
    a: &VehicleKeyAction,
) -> Option<KeyChord> {
    let text = registry.text(settings, a.setting)?;
    KeyChord::parse(&text).ok().flatten()
}

/// Rebuilds [`ScriptKeyBindings`] from the keyboard.cfg base and the settings whenever either
/// changes.
pub fn sync_bindings(
    registry: Res<SettingsRegistry>,
    settings: Res<GameSettings>,
    base: Res<ScriptKeyBase>,
    mut bindings: ResMut<ScriptKeyBindings>,
    rail: Query<
        (),
        (
            With<opts_bevy::PlayerVehicle>,
            With<crate::in_game::omsi_world::RailPlayer>,
        ),
    >,
    mut first: Local<Option<bool>>,
) {
    let tram = !rail.is_empty();
    if *first == Some(tram) && !settings.is_changed() && !base.is_changed() {
        return;
    }
    *first = Some(tram);
    let own: Vec<ScriptKeyBinding> = ACTIONS
        .iter()
        .filter(|a| a.scope.applies(tram))
        .filter_map(|a| {
            let c = chord_of(&registry, &settings, a)?;
            Some(
                std::iter::once(a.trigger)
                    .chain(a.also)
                    .map(move |t| ScriptKeyBinding {
                        trigger: t.to_string(),
                        key: c.key,
                        shift: c.shift,
                        ctrl: c.ctrl,
                        held: a.held,
                    }),
            )
        })
        .flatten()
        .collect();
    // The triggers of every action are the settings', never the installation's key file's (also
    // those of the actions that do not apply to this vehicle).
    let replaced: Vec<&str> = ACTIONS
        .iter()
        .flat_map(|a| std::iter::once(a.trigger).chain(a.also))
        .collect();
    let composed = ScriptKeyBindings::compose(&base.0, &replaced, own);
    if composed.0 != bindings.0 {
        *bindings = composed;
    }
}

/// Registers the sync system.
pub struct VehicleKeysPlugin;

impl Plugin for VehicleKeysPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<ScriptKeyBase>()
            .init_resource::<ScriptKeyBindings>()
            .add_systems(PreUpdate, sync_bindings);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::settings_registry::SettingsCapabilities;

    fn parts() -> (SettingsRegistry, SettingsCapabilities, GameSettings) {
        (
            SettingsRegistry::with_builtin(),
            SettingsCapabilities::default(),
            GameSettings::default(),
        )
    }

    #[test]
    fn defaults_are_valid_unique_and_free() {
        let mut seen = std::collections::BTreeSet::new();
        // Bus and tram keys may share a key; the scopes are checked one at a time.
        for tram in [false, true] {
            seen.clear();
            for a in ACTIONS.iter().filter(|a| a.scope.applies(tram)) {
                let chord = KeyChord::parse(a.default).unwrap_or_else(|e| panic!("{}: {e}", a.id));
                if let Some(c) = chord {
                    assert!(
                        !c.is_reserved(),
                        "{} default {} is reserved",
                        a.id,
                        a.default
                    );
                    assert!(seen.insert(a.default), "{} duplicates {}", a.id, a.default);
                    assert_eq!(c.to_text().as_deref(), Some(a.default));
                }
            }
        }
    }

    #[test]
    fn every_action_has_a_registered_setting_and_messages() {
        let (reg, ..) = parts();
        let loc = crate::settings_registry::localization("en");
        let de = crate::settings_registry::localization("de");
        for a in ACTIONS {
            let d = reg.get(a.setting).expect(a.setting);
            assert!(matches!(d.kind, SettingKind::Binding));
            for l in [&loc, &de] {
                assert_ne!(l.t(a.label_key), a.label_key, "{}", a.label_key);
                assert_ne!(l.t(a.desc_key), a.desc_key, "{}", a.desc_key);
            }
        }
    }

    #[test]
    fn rebinding_is_validated_and_conflicts_are_refused() {
        let (reg, caps, mut s) = parts();
        // Free key: accepted and canonicalised.
        let (_, new) = reg
            .set(&mut s, &caps, "controls.vehicle.lights", "ctrl+shift+g")
            .unwrap();
        assert_eq!(new, "Ctrl+Shift+G");
        // Another action's key.
        let e = reg
            .set(&mut s, &caps, "controls.vehicle.hazard", "M")
            .unwrap_err();
        assert!(e.to_string().contains("engine_start"), "{e}");
        // A key OPTS uses (throttle), but a Ctrl combination of it is fine.
        assert!(reg
            .set(&mut s, &caps, "controls.vehicle.hazard", "W")
            .is_err());
        assert!(reg
            .set(&mut s, &caps, "controls.vehicle.hazard", "Ctrl+W")
            .is_ok());
        // Garbage, and unbinding.
        assert!(reg
            .set(&mut s, &caps, "controls.vehicle.hazard", "Hyper+Q")
            .is_err());
        assert!(reg
            .set(&mut s, &caps, "controls.vehicle.hazard", "nonsense")
            .is_err());
        let (_, none) = reg
            .set(&mut s, &caps, "controls.vehicle.hazard", "")
            .unwrap();
        assert_eq!(none, "none");
        // The key freed by rebinding `lights` can be taken by another action.
        assert!(reg
            .set(&mut s, &caps, "controls.vehicle.hazard", "L")
            .is_ok());
    }

    #[test]
    fn settings_drive_the_bindings() {
        let mut app = App::new();
        app.add_plugins(VehicleKeysPlugin)
            .insert_resource(SettingsRegistry::with_builtin())
            .insert_resource(GameSettings::default())
            .insert_resource(ScriptKeyBase(vec![
                ScriptKeyBinding {
                    trigger: "IBIS_1".into(),
                    key: KeyCode::Numpad1,
                    shift: false,
                    ctrl: true,
                    held: false,
                },
                // A cfg binding of a rebindable action is replaced.
                ScriptKeyBinding {
                    trigger: "kw_scheinwerfer_toggle".into(),
                    key: KeyCode::KeyK,
                    shift: false,
                    ctrl: false,
                    held: false,
                },
            ]));
        app.update();
        let lights = |app: &App| -> Vec<KeyCode> {
            app.world()
                .resource::<ScriptKeyBindings>()
                .0
                .iter()
                .filter(|b| b.trigger == "kw_scheinwerfer_toggle")
                .map(|b| b.key)
                .collect()
        };
        assert_eq!(lights(&app), [KeyCode::KeyL]);
        assert!(app
            .world()
            .resource::<ScriptKeyBindings>()
            .0
            .iter()
            .any(|b| b.trigger == "IBIS_1"));
        let caps = SettingsCapabilities::default();
        let reg = app.world().resource::<SettingsRegistry>().clone();
        reg.set(
            &mut app.world_mut().resource_mut::<GameSettings>(),
            &caps,
            "controls.vehicle.lights",
            "G",
        )
        .unwrap();
        app.update();
        assert_eq!(lights(&app), [KeyCode::KeyG]);
        // "none" unbinds.
        reg.set(
            &mut app.world_mut().resource_mut::<GameSettings>(),
            &caps,
            "controls.vehicle.lights",
            "none",
        )
        .unwrap();
        app.update();
        assert!(lights(&app).is_empty());
    }
}
