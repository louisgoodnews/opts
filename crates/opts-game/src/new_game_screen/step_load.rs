//! The *Load all tiles* row of the Review step (M-0080): a switch for the global setting
//! `gameplay.load_all_tiles`, what keeping the whole map resident costs on this machine, and a
//! way back to the tile radius when the memory would not fit.
//!
//! The row is an empty node when the review is built; [`refresh_load_row`] fills it, so it
//! follows the setting and the hardware profile without the wizard's build context knowing them.

use bevy::prelude::*;
use opts_gui::interaction::events::SwitchToggled;
use opts_gui::interaction::Activated;
use opts_gui::l10n::Localization;
use opts_gui::prelude::*;
use opts_hardware::recommend::{estimate_load_all, safe_resident_mib, MapStats, MemoryVerdict};

use super::types::{GameCatalogue, NewGameSetup};
use super::ui::{column, plain};
use crate::app_state::AppState;
use crate::hardware::HardwareState;
use crate::settings::GameSettings;
use crate::settings_registry::{
    ChangeSource, SettingChanged, SettingsApi, SettingsCapabilities, SettingsRegistry,
};

/// The setting the switch writes.
const KEY: &str = "gameplay.load_all_tiles";

/// The node [`refresh_load_row`] fills.
#[derive(Component)]
pub struct LoadRow;

/// The switch of the row.
#[derive(Component)]
pub struct LoadAllSwitch;

/// The button that turns *Load all tiles* off again.
#[derive(Component)]
pub struct UseRadiusButton;

/// Registers the row's systems; they wait for the settings, which a bare test app lacks.
pub fn register(app: &mut App) {
    app.add_message::<SettingChanged>().add_systems(
        Update,
        (apply_load_all_switch, apply_use_radius, refresh_load_row)
            .run_if(in_state(AppState::VehicleSelection))
            .run_if(resource_exists::<SettingsRegistry>)
            .run_if(resource_exists::<SettingsCapabilities>)
            .run_if(resource_exists::<GameSettings>)
            .after(opts_gui::interaction::activation::ActivationSet),
    );
}

/// The empty row; spawned by the review step.
pub fn spawn_load_row(commands: &mut Commands) -> Entity {
    commands
        .spawn((
            Node {
                width: Val::Percent(100.0),
                max_width: Val::Px(760.0),
                ..column(8.0)
            },
            LoadRow,
            Name::new("LoadAllRow"),
        ))
        .id()
}

/// Memory as the user reads it: GB with one decimal below 10, whole GB above, MB under a GB.
pub fn memory_text(mib: u64) -> String {
    if mib < 1024 {
        format!("{mib} MB")
    } else if mib < 10 * 1024 {
        format!("{:.1} GB", mib as f64 / 1024.0)
    } else {
        format!("{} GB", (mib as f64 / 1024.0).round() as u64)
    }
}

/// A time range: seconds up to a minute and a half, else minutes.
pub fn time_text(seconds: (u32, u32)) -> String {
    let (lo, hi) = seconds;
    if hi <= 90 {
        format!("{lo}–{hi} s")
    } else {
        format!("{}–{} min", lo.div_ceil(60), hi.div_ceil(60))
    }
}

/// What the row's content is built from.
#[derive(bevy::ecs::system::SystemParam)]
pub struct RowInputs<'w> {
    hardware: Option<Res<'w, HardwareState>>,
    catalogue: Res<'w, GameCatalogue>,
    setup: Res<'w, NewGameSetup>,
    loc: Res<'w, Localization>,
    registry: Res<'w, SettingsRegistry>,
    settings: Res<'w, GameSettings>,
}

/// Rebuilds the row's content when it appears, a setting changes or the profile arrives.
pub fn refresh_load_row(
    rows: Query<(Entity, Has<Children>), With<LoadRow>>,
    fresh: Query<(), Added<LoadRow>>,
    mut changed: MessageReader<SettingChanged>,
    inputs: RowInputs,
    mut commands: Commands,
) {
    let setting_moved = changed.read().any(|c| c.key == KEY);
    let profile_moved = inputs.hardware.as_ref().is_some_and(|h| h.is_changed());
    if fresh.is_empty() && !setting_moved && !profile_moved {
        return;
    }
    let on = inputs.registry.text(&inputs.settings, KEY).as_deref() == Some("true");
    let tiles = inputs
        .catalogue
        .map_by_name(&inputs.setup.selected_map)
        .map_or(0, |m| u32::try_from(m.tile_count).unwrap_or(0));
    let profile = inputs.hardware.as_ref().and_then(|h| h.profile.as_ref());
    for (row_entity, has_content) in &rows {
        if has_content {
            commands.entity(row_entity).despawn_related::<Children>();
        }
        let content = content(&mut commands, &inputs.loc, on, tiles, profile);
        commands.entity(row_entity).add_children(&content);
    }
}

/// The switch, the estimate and, when it does not fit, the warning with its way out.
fn content(
    commands: &mut Commands,
    loc: &Localization,
    on: bool,
    tiles: u32,
    profile: Option<&opts_hardware::HardwareProfile>,
) -> Vec<Entity> {
    let switch = SwitchBuilder::new(loc.t("wiz-load-all-title"))
        .with_value(on)
        .spawn(commands);
    commands
        .entity(switch)
        .insert((LoadAllSwitch, UiId::new("wizard.review.load_all")));
    let mut out = vec![switch];
    if tiles == 0 {
        out.push(note(commands, loc.t("wiz-load-all-unknown-map")));
        return out;
    }
    let Some(profile) = profile else {
        out.push(note(commands, loc.t("wiz-load-all-no-profile")));
        return out;
    };
    let estimate = estimate_load_all(profile, tiles, &MapStats::default());
    let args = [
        ("tiles".to_string(), (tiles as i32).into()),
        (
            "memory".to_string(),
            memory_text(estimate.memory_mib).into(),
        ),
        ("time".to_string(), time_text(estimate.seconds).into()),
    ];
    out.push(note(commands, loc.t_args("wiz-load-all-note", &args)));
    if !estimate.storage_measured {
        out.push(note(commands, loc.t("wiz-load-all-slow-disk")));
    }
    if estimate.verdict == MemoryVerdict::TooLarge {
        out.push(warning(commands, loc, estimate.memory_mib, profile));
    }
    out
}

fn note(commands: &mut Commands, text: String) -> Entity {
    commands
        .spawn(plain(
            text,
            TypeScale::BodyMedium,
            ColorRole::OnSurfaceVariant,
        ))
        .id()
}

/// Red text and the button back to the radius; shown whether or not the switch is on, so the
/// user learns it before switching.
fn warning(
    commands: &mut Commands,
    loc: &Localization,
    memory_mib: u64,
    profile: &opts_hardware::HardwareProfile,
) -> Entity {
    let safe = safe_resident_mib(profile).unwrap_or(0);
    let args = [
        ("memory".to_string(), memory_text(memory_mib).into()),
        ("safe".to_string(), memory_text(safe).into()),
    ];
    let text = commands
        .spawn(plain(
            loc.t_args("wiz-load-all-too-large", &args),
            TypeScale::BodyMedium,
            ColorRole::Error,
        ))
        .id();
    let button = ButtonBuilder::tonal("wiz-load-all-use-radius")
        .with_key("wiz-load-all-use-radius")
        .spawn(commands);
    commands
        .entity(button)
        .insert((UseRadiusButton, UiId::new("wizard.review.use_radius")));
    let box_ = commands.spawn(column(8.0)).id();
    commands.entity(box_).add_children(&[text, button]);
    box_
}

/// The switch writes the global setting.
pub fn apply_load_all_switch(
    mut toggled: MessageReader<SwitchToggled>,
    switches: Query<(), With<LoadAllSwitch>>,
    mut api: SettingsApi,
) {
    for t in toggled.read() {
        if switches.get(t.entity).is_err() {
            continue;
        }
        let text = if t.is_on { "true" } else { "false" };
        if api.text(KEY) != text {
            let _ = api.change(KEY, Some(text), ChangeSource::Page, true);
        }
    }
}

/// *Use the tile radius* turns the whole-map loading off.
pub fn apply_use_radius(
    mut activated: MessageReader<Activated>,
    buttons: Query<(), With<UseRadiusButton>>,
    mut api: SettingsApi,
) {
    for Activated(e) in activated.read() {
        if buttons.get(*e).is_ok() {
            let _ = api.change(KEY, Some("false"), ChangeSource::Page, true);
        }
    }
}

#[cfg(test)]
mod tests;
