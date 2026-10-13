//! The Settings screen (`AppState::Settings`): the generic page of the settings registry inside
//! the shell's content area (or a full window before the shell exists).
//!
//! - [`page`]: frame, category list, rebuild; [`rows`]: one row and its control per kind;
//!   [`input`]: what controls do and the revert prompt; [`mod@info`]: read-only lines (locations,
//!   catalogue, keys); [`content`]: the OMSI folder chooser, *Scan now*, *Clear cache*.
//! - [`spawn_settings_page`] hosts the page anywhere (deep link `screen:settings/<category>`).
//! - Spec: `docs/src/architecture/settings-registry.md`.

pub mod content;
pub mod info;
pub mod input;
pub mod overlay;
pub mod page;
pub mod rows;
pub mod scan_status;
pub mod systems;
pub mod types;

use bevy::prelude::*;
use opts_gui::components::button::ButtonBuilder;
use opts_gui::prelude::*;

use crate::app_state::AppState;
use crate::shell::{spawn_screen_root, ShellContent};

pub use input::{PendingRevert, SettingActionRequested};
pub use overlay::{open_settings_overlay, SettingsOverlay};
pub use page::{spawn_settings_page, SettingsPageState};
pub use systems::{cleanup_settings_screen, handle_back_to_menu};
pub use types::{BackToMenuButton, SettingsScreenRoot};

/// Plugin managing the interactive Settings screen.
#[derive(Default)]
pub struct SettingsScreenPlugin;

impl Plugin for SettingsScreenPlugin {
    fn build(&self, app: &mut App) {
        if !app.is_plugin_added::<crate::settings_registry::SettingsRegistryPlugin>() {
            app.add_plugins(crate::settings_registry::SettingsRegistryPlugin);
        }
        app.add_plugins(page::SettingsPagePlugin)
            .init_resource::<crate::look::ActiveLook>()
            .add_message::<crate::first_run::OmsiFolderChosen>()
            .add_systems(OnEnter(AppState::Settings), spawn_settings_screen)
            .add_systems(OnExit(AppState::Settings), cleanup_settings_screen)
            .add_systems(
                Update,
                (
                    handle_back_to_menu,
                    content::run_actions,
                    content::sync_chosen_folder,
                )
                    .run_if(any_with_component::<page::SettingsPage>)
                    .run_if(opts_bevy::reporting::not_modal_active),
            )
            .add_systems(
                Update,
                scan_status::refresh_scan_chip.run_if(any_with_component::<scan_status::ScanChip>),
            )
            .add_systems(
                Update,
                overlay::close_overlay.run_if(any_with_component::<SettingsOverlay>),
            );
    }
}

/// Spawns the Settings screen: the page inside the shell's content area, or full window with a
/// back button when there is no shell.
pub fn spawn_settings_screen(mut commands: Commands, shell: Option<Res<ShellContent>>) {
    let shell = shell.map_or_else(ShellContent::default, |s| *s);
    let root = spawn_screen_root(&mut commands, &shell, "settings");
    commands.entity(root).insert(SettingsScreenRoot);
    if shell.0.is_none() {
        commands.entity(root).insert((
            Node {
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                position_type: PositionType::Absolute,
                left: Val::Px(0.0),
                top: Val::Px(0.0),
                flex_direction: FlexDirection::Column,
                ..default()
            },
            BackgroundRole(ColorRole::Surface),
        ));
        let back = ButtonBuilder::outlined("settings-back")
            .with_key("settings-back")
            .spawn(&mut commands);
        commands.entity(back).insert((
            BackToMenuButton,
            UiId::new("settings/back"),
            Node {
                margin: UiRect::new(Val::Px(16.0), Val::Auto, Val::Px(12.0), Val::Px(0.0)),
                align_self: AlignSelf::FlexStart,
                min_height: Val::Px(40.0),
                padding: UiRect::axes(Val::Px(24.0), Val::Px(10.0)),
                column_gap: Val::Px(8.0),
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                border: UiRect::all(Val::Px(1.0)),
                ..default()
            },
            ChildOf(root),
        ));
    }
    spawn_settings_page(&mut commands, root, None);
}

#[cfg(test)]
mod tests;
