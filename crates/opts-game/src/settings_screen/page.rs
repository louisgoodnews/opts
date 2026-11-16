//! The generic settings page: categories on the left, rows on the right, a search field on top.
//!
//! [`spawn_settings_page`] builds the frame; [`rebuild_page`] fills the category list and the
//! pane from the registry whenever the page state, a setting or the language changes.

use bevy::input::mouse::MouseWheel;
use bevy::prelude::*;
use bevy::window::PrimaryWindow;
use opts_gui::components::button::ButtonBuilder;
use opts_gui::components::text_field::{M3TextField, TextFieldBuilder};
use opts_gui::l10n::Localization;
use opts_gui::prelude::*;

use crate::settings::GameSettings;
use crate::settings_registry::{
    search, Category, SettingChanged, SettingKind, SettingsCapabilities, SettingsRegistry,
};

use super::input::{SettingOp, SettingSwitch};
use super::rows::{label, spawn_row, Ctx, SettingValueText, ROLES_PLAIN, ROLES_SELECTED};
use opts_gui::components::switch::M3Switch;
use opts_gui::l10n::L10nArgs;

mod frame;
mod systems;

pub use frame::spawn_settings_page;
pub use systems::{restore_focus_system, scroll_pane, search_field_system, style_categories};

/// What the page shows.
#[derive(Resource, Debug, Clone)]
pub struct SettingsPageState {
    /// The selected category.
    pub category: Category,
    /// The search text; non-empty replaces the category by the matches.
    pub query: String,
    /// Set when a category button was used: the search field is emptied.
    pub clear_search_field: bool,
    /// The page needs to be filled again.
    pub dirty: bool,
    /// Id of the control that had keyboard focus when the pane was rebuilt; focus returns to it.
    pub restore_focus: Option<String>,
    /// The key setting waiting for the next key press (Controls page, *Change key*).
    pub rebinding: Option<&'static str>,
    /// A line above the rows: why a rebinding was refused, or what to press.
    pub notice: Option<String>,
}

/// Root of the page.
#[derive(Component)]
pub struct SettingsPage;
/// The list of categories.
#[derive(Component)]
pub struct PageCategories;
/// The scrolling pane of rows.
#[derive(Component)]
pub struct PagePane;
/// The search field.
#[derive(Component)]
pub struct SettingsSearch;

/// The systems of the page (also used by tests without the application states).
pub struct SettingsPagePlugin;

impl Plugin for SettingsPagePlugin {
    fn build(&self, app: &mut App) {
        if !app.is_plugin_added::<crate::content_facts::ContentFactsPlugin>() {
            app.add_plugins(crate::content_facts::ContentFactsPlugin);
        }
        app.add_message::<super::input::SettingActionRequested>()
            .add_systems(
                Update,
                (
                    super::scan_status::refresh_scan_chip
                        .run_if(any_with_component::<super::scan_status::ScanChip>),
                    super::info::fill_content_lines
                        .run_if(any_with_component::<super::info::ContentLine>),
                ),
            )
            .add_systems(
                Update,
                (
                    mark_dirty,
                    refresh_rows,
                    rebuild_page,
                    restore_focus_system,
                    style_categories,
                    search_field_system,
                    scroll_pane.run_if(opts_bevy::reporting::not_modal_active),
                    super::input::handle_page_input,
                    super::input::capture_rebind,
                    super::input::revert_prompt_system,
                )
                    .chain()
                    .run_if(resource_exists::<SettingsPageState>),
            );
    }
}

mod params;
mod refresh;
pub use refresh::{mark_dirty, refresh_rows};

/// The message key of a theme kind's name.
pub fn theme_name_key(kind: opts_gui::theme::ThemeKind) -> &'static str {
    use opts_gui::l10n::keys::theme as k;
    match kind {
        opts_gui::theme::ThemeKind::Dark => k::MODE_DARK,
        opts_gui::theme::ThemeKind::Light => k::MODE_LIGHT,
        opts_gui::theme::ThemeKind::HighContrast => k::HIGH_CONTRAST,
    }
}

/// Marks the "right now the ... theme is shown" line of the Interface page.
#[derive(Component)]
pub struct ThemeNowText;

mod rebuild;
pub use rebuild::{level_key, rebuild_page};
