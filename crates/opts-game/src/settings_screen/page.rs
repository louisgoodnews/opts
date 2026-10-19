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

/// Spawns the settings page as a child of `parent` and returns it. `category` is the id of the
/// category to show (`graphics`, ...; the deep link `screen:settings/graphics`), the first one
/// when `None` or unknown.
pub fn spawn_settings_page(
    commands: &mut Commands,
    parent: Entity,
    category: Option<&str>,
) -> Entity {
    let category = category
        .and_then(Category::parse)
        .unwrap_or(Category::Graphics);
    commands.insert_resource(SettingsPageState {
        category,
        query: String::new(),
        clear_search_field: false,
        dirty: true,
        restore_focus: None,
        rebinding: None,
        notice: None,
    });
    let page = commands
        .spawn((
            Node {
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                flex_direction: FlexDirection::Column,
                padding: UiRect::all(Val::Px(16.0)),
                row_gap: Val::Px(12.0),
                min_height: Val::Px(0.0),
                ..default()
            },
            SettingsPage,
            UiId::new("settings"),
            Name::new("SettingsPage"),
            ChildOf(parent),
        ))
        .id();
    let header = commands
        .spawn((
            Node {
                flex_direction: FlexDirection::Row,
                flex_wrap: FlexWrap::Wrap,
                align_items: AlignItems::Center,
                justify_content: JustifyContent::SpaceBetween,
                column_gap: Val::Px(16.0),
                row_gap: Val::Px(8.0),
                ..default()
            },
            ChildOf(page),
        ))
        .id();
    label(
        commands,
        header,
        "settings-title",
        TypeScale::Headline,
        ColorRole::OnSurface,
    );
    let field = TextFieldBuilder::outlined("").compact().spawn(commands);
    commands.entity(field).insert((
        SettingsSearch,
        UiId::new("settings/search"),
        ChildOf(header),
    ));
    let body = commands
        .spawn((
            Node {
                flex_direction: FlexDirection::Row,
                flex_grow: 1.0,
                column_gap: Val::Px(16.0),
                min_height: Val::Px(0.0),
                ..default()
            },
            ChildOf(page),
        ))
        .id();
    let categories = commands
        .spawn((
            Node {
                flex_direction: FlexDirection::Column,
                row_gap: Val::Px(6.0),
                flex_shrink: 0.0,
                min_width: Val::Px(150.0),
                min_height: Val::Px(0.0),
                // seven categories do not fit under 480 logical px (UI scale 200 %)
                overflow: Overflow::scroll_y(),
                ..default()
            },
            PageCategories,
            ChildOf(body),
        ))
        .id();
    opts_gui::patterns::scroll::wheel_scroll(commands, categories);
    commands.spawn((
        Node {
            flex_direction: FlexDirection::Column,
            flex_grow: 1.0,
            flex_basis: Val::Px(0.0),
            min_width: Val::Px(0.0),
            overflow: Overflow::scroll_y(),
            ..default()
        },
        ScrollPosition::default(),
        PagePane,
        FocusRegion(Landmark::Content),
        ChildOf(body),
    ));
    page
}

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

/// Marks the page dirty when something it cannot update in place changed: the language, or a
/// path or free text value.
#[allow(clippy::too_many_arguments)]
pub fn mark_dirty(
    mut state: ResMut<SettingsPageState>,
    mut changes: MessageReader<SettingChanged>,
    registry: Res<SettingsRegistry>,
    loc: Res<Localization>,
    added: Query<(), Added<SettingsPage>>,
    facts: Res<crate::content_facts::ContentFactsStore>,
    light: Option<Res<opts_bevy::light_quality::LightQuality>>,
    fonts: Option<Res<crate::ui_typography::FontChoices>>,
    mut themes: Local<Vec<String>>,
) {
    if fonts.is_some_and(|f| f.is_changed()) {
        state.dirty = true;
    }
    if light.is_some_and(|l| l.is_changed()) {
        // The values the automatic mode shows (and which controls are locked) changed.
        state.dirty = true;
    }
    if themes.as_slice() != facts.theme_files() {
        *themes = facts.theme_files().to_vec();
        state.dirty = true;
    }
    let structural = changes.read().any(|c| {
        c.key == crate::settings_registry::lighting::MODE_KEY
            || registry.get(&c.key).is_some_and(|d| {
                matches!(d.kind, SettingKind::Path)
                    || (matches!(d.kind, SettingKind::Text) && d.key != "theme_file")
            })
    });
    if structural || loc.is_changed() || !added.is_empty() {
        state.dirty = true;
    }
}

/// Query of the elements `refresh_rows` updates.
type OptionButtons<'w, 's> = Query<
    'w,
    's,
    (
        &'static SettingOp,
        &'static mut BackgroundRole,
        &'static mut BorderRole,
        Option<&'static Children>,
    ),
>;

/// Shows the new value of a changed setting in its row without rebuilding it (keeps focus).
#[allow(clippy::too_many_arguments)]
pub fn refresh_rows(
    mut commands: Commands,
    mut changes: MessageReader<SettingChanged>,
    registry: Res<SettingsRegistry>,
    settings: Res<GameSettings>,
    loc: Res<Localization>,
    theme: Res<MaterialTheme>,
    mut options: OptionButtons,
    mut labels: Query<&mut ForegroundRole>,
    mut switches: Query<(&SettingSwitch, &mut M3Switch)>,
    mut values: Query<(&SettingValueText, &mut Text)>,
    resets: Query<(
        Entity,
        &SettingOp,
        Has<opts_gui::interaction::state::Disabled>,
    )>,
    mut now: Query<&mut L10nArgs, With<ThemeNowText>>,
) {
    let keys: Vec<String> = changes.read().map(|c| c.key.clone()).collect();
    let text = |k: &str| registry.text(&settings, k).unwrap_or_default();
    for (op, mut bg, mut border, kids) in &mut options {
        let SettingOp::Set { key, value } = op else {
            continue;
        };
        if !keys.iter().any(|k| k == key) {
            continue;
        }
        let (b, f, o) = if text(key) == *value {
            ROLES_SELECTED
        } else {
            ROLES_PLAIN
        };
        bg.0 = b;
        border.0 = o;
        for k in kids.into_iter().flatten() {
            if let Ok(mut fg) = labels.get_mut(*k) {
                fg.0 = f;
            }
        }
    }
    for (s, mut sw) in &mut switches {
        if keys.iter().any(|k| k == s.0) {
            sw.is_on = text(s.0) == "true";
        }
    }
    for (v, mut t) in &mut values {
        if keys.iter().any(|k| k == v.0) {
            t.0 = text(v.0);
        }
    }
    for (e, op, disabled) in &resets {
        let SettingOp::Reset { key } = op else {
            continue;
        };
        if !keys.iter().any(|k| k == key) {
            continue;
        }
        let at_default = registry.get(key).is_some_and(|d| d.default == text(key));
        if at_default && !disabled {
            commands.entity(e).insert((
                opts_gui::interaction::state::Disabled,
                opts_gui::interaction::state::DisabledReason(
                    loc.t("settings-reset-disabled").into(),
                ),
            ));
        } else if !at_default && disabled {
            commands.entity(e).remove::<(
                opts_gui::interaction::state::Disabled,
                opts_gui::interaction::state::DisabledReason,
            )>();
        }
    }
    if theme.is_changed() || !keys.is_empty() {
        for mut args in &mut now {
            *args = L10nArgs::one("theme", loc.t(theme_name_key(theme.kind())));
        }
    }
}

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

/// Puts keyboard focus back on the control that had it before the pane was rebuilt.
pub fn restore_focus_system(
    mut state: ResMut<SettingsPageState>,
    ids: Query<(Entity, &UiId)>,
    mut focus: MessageWriter<opts_gui::interaction::focus::FocusEvent>,
) {
    let Some(want) = state.restore_focus.clone() else {
        return;
    };
    if let Some((e, _)) = ids.iter().find(|(_, u)| u.0 == want.as_str()) {
        focus.write(opts_gui::interaction::focus::FocusEvent::Set(e));
        state.restore_focus = None;
    }
}

/// Marks the selected category button and dims the others.
pub fn style_categories(
    state: Res<SettingsPageState>,
    mut buttons: Query<(&SettingOp, &mut BackgroundRole, Option<&Children>)>,
    mut labels: Query<&mut ForegroundRole>,
    fresh: Query<(), Added<SettingOp>>,
) {
    if !state.is_changed() && fresh.is_empty() {
        return;
    }
    let searching = !state.query.trim().is_empty();
    for (op, mut bg, kids) in &mut buttons {
        let SettingOp::Category(cat) = op else {
            continue;
        };
        let selected = !searching && *cat == state.category;
        bg.0 = if selected {
            ColorRole::Primary
        } else {
            ColorRole::Surface
        };
        for k in kids.into_iter().flatten() {
            if let Ok(mut fg) = labels.get_mut(*k) {
                fg.0 = if selected {
                    ColorRole::OnPrimary
                } else {
                    ColorRole::Primary
                };
            }
        }
    }
}

/// Gives the search field its localized placeholder and name, a usable width, and empties it
/// after a category was chosen.
pub fn search_field_system(
    mut state: ResMut<SettingsPageState>,
    loc: Res<Localization>,
    mut fields: Query<(&mut M3TextField, &mut Node), With<SettingsSearch>>,
    fresh: Query<(), Added<SettingsSearch>>,
) {
    let fresh = !fresh.is_empty();
    for (mut field, mut node) in &mut fields {
        let want = loc.t("settings-search-placeholder");
        if field.placeholder != want {
            field.placeholder.clone_from(&want);
            field.label = want;
        }
        if fresh {
            node.min_width = Val::Px(260.0);
            node.flex_grow = 0.0;
        }
        if state.clear_search_field {
            field.text.clear();
        }
    }
    if state.clear_search_field {
        state.clear_search_field = false;
    }
}

/// Mouse wheel over the pane scrolls it.
pub fn scroll_pane(
    mut wheel: MessageReader<MouseWheel>,
    window: Query<&Window, With<PrimaryWindow>>,
    mut panes: Query<(&mut ScrollPosition, &UiGlobalTransform, &ComputedNode), With<PagePane>>,
) {
    let Ok(window) = window.single() else { return };
    let Some(cursor) = window.physical_cursor_position() else {
        wheel.clear();
        return;
    };
    for e in wheel.read() {
        for (mut pos, transform, node) in &mut panes {
            if !node.contains_point(*transform, cursor) {
                continue;
            }
            let max =
                ((node.content_size().y - node.size().y) * node.inverse_scale_factor()).max(0.0);
            pos.y = (pos.y - e.y * 36.0).clamp(0.0, max);
        }
    }
}
