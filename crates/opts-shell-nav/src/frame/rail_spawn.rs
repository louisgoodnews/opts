//! Builds the navigation rail: its brand, the registered screens, the footer actions and the version.

use super::entry::{spawn_entry, text_node};
use super::*;

/// The navigation rail: the brand, the registered screens (main ones above, utility ones below),
/// the theme toggle, *Quit* and the version.
pub(super) fn spawn_rail(commands: &mut Commands, registry: &ScreenRegistry) -> Entity {
    let rail = commands
        .spawn((
            Node {
                width: Val::Px(RAIL_WIDE_PX),
                flex_shrink: 0.0,
                flex_direction: FlexDirection::Column,
                // At a large UI scale or a small window the rail is taller than the window: it
                // scrolls (and follows keyboard focus) instead of losing Settings, Help and Quit.
                overflow: Overflow::scroll_y(),
                padding: UiRect::all(Val::Px(12.0)),
                row_gap: Val::Px(4.0),
                border: UiRect::right(Val::Px(1.0)),
                ..default()
            },
            BackgroundRole(ColorRole::SurfaceContainerLow),
            BorderRole(ColorRole::OutlineVariant),
            FocusRegion(Landmark::Rail),
            ShellRail,
            Name::new("ShellRail"),
        ))
        .id();
    commands.entity(rail).with_children(|rail| {
        rail.spawn((
            text_node(keys::BRAND, ColorRole::Primary, TypeScale::TitleLarge),
            RailWideOnly,
            Node {
                margin: UiRect::axes(Val::Px(12.0), Val::Px(12.0)),
                ..default()
            },
        ));
        for d in registry.group(ScreenGroup::Main) {
            spawn_registered(rail, d);
        }
        rail.spawn(Node {
            flex_grow: 1.0,
            ..default()
        });
        for d in registry.group(ScreenGroup::Utility) {
            spawn_registered(rail, d);
        }
        rail.spawn(Node {
            height: Val::Px(1.0),
            margin: UiRect::vertical(Val::Px(4.0)),
            ..default()
        })
        .insert(BackgroundRole(ColorRole::OutlineVariant));
        let toggle = spawn_entry(
            rail,
            icons::LIGHT_MODE,
            keys::THEME_TO_LIGHT,
            "shell.theme-toggle".to_owned(),
        );
        let quit = spawn_entry(rail, icons::POWER, keys::QUIT, "shell.quit".to_owned());
        rail.commands()
            .entity(toggle)
            .insert(RailAction::ThemeToggle);
        rail.commands().entity(quit).insert(RailAction::Quit);
        rail.spawn((
            Text::new(keys::VERSION),
            L10nKey(keys::VERSION),
            L10nArgs::one("version", env!("CARGO_PKG_VERSION")),
            TextColor::default(),
            TypeStyleToken(TypeScale::Label),
            ForegroundRole(ColorRole::OnSurfaceVariant),
            RailWideOnly,
            Node {
                margin: UiRect::axes(Val::Px(12.0), Val::Px(4.0)),
                ..default()
            },
        ));
    });
    rail
}

fn spawn_registered(rail: &mut ChildSpawnerCommands, d: &ScreenDescriptor) {
    let entry = spawn_entry(rail, d.icon, d.title_key, format!("shell.nav.{}", d.id));
    rail.commands().entity(entry).insert(RailItem { id: d.id });
}
