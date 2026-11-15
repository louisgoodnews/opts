//! Building the settings page frame: the title and search row, and the category list beside the pane.

use super::*;

/// The title and the search field.
pub(super) fn spawn_header(commands: &mut Commands, page: Entity) {
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
}

/// The category list on the left and the pane the selected category fills.
pub(super) fn spawn_body(commands: &mut Commands, page: Entity) {
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
}

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
    spawn_header(commands, page);
    spawn_body(commands, page);
    page
}
