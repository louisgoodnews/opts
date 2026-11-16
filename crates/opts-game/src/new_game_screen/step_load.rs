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


mod apply;
mod rows;
pub use apply::{apply_load_all_switch, apply_use_radius};
use rows::content;


#[cfg(test)]
mod tests;
