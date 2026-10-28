//! The shell frame: navigation rail, banner area and content area.
//!
//! Spawned once at start-up from the [`ScreenRegistry`]. The rail is 72 px wide with icons only in
//! the *Compact* layout class and 240 px with labels otherwise. While a drive runs the frame is
//! hidden and [`ShellContent`] is `None`, so full-window overlays build as roots.

use bevy::prelude::*;
use opts_gui::interaction::state::{Disabled, DisabledReason, Selected};
use opts_gui::interaction::tooltip::Tooltip;
use opts_gui::l10n::{L10nArgs, L10nKey, L10nTooltip};
use opts_gui::not_yet::{Availability, NotAvailableYet};
use opts_gui::prelude::*;
use opts_gui::theme::LayoutClass;

use super::registry::{ScreenDescriptor, ScreenGroup, ScreenRegistry};
use super::{icons, keys, nav, ShellContent};
use crate::app_state::AppState;

/// Rail width with icons only.
pub const RAIL_COMPACT_PX: f32 = 72.0;
/// Rail width with labels.
pub const RAIL_WIDE_PX: f32 = 240.0;

/// Root of the shell frame.
#[derive(Component, Debug, Clone, Copy)]
pub struct ShellRoot;
/// The navigation rail.
#[derive(Component, Debug, Clone, Copy)]
pub struct ShellRail;
/// Where banners go (top of the content column).
#[derive(Component, Debug, Clone, Copy)]
pub struct ShellBanners;

/// A rail entry for a registered destination.
#[derive(Component, Debug, Clone, Copy)]
pub struct RailItem {
    /// The destination id.
    pub id: &'static str,
}

/// The actions of the rail footer.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub enum RailAction {
    /// Switch between the dark and the light theme (F2).
    ThemeToggle,
    /// Quit OPTS (with confirmation).
    Quit,
}

/// The icon of a rail entry.
#[derive(Component, Debug, Clone, Copy)]
pub struct RailIcon;
/// Text that is only shown when the rail is wide (labels, brand, version).
#[derive(Component, Debug, Clone, Copy)]
pub struct RailWideOnly;
/// The red *Feature not available yet!* line of a rail entry.
#[derive(Component, Debug, Clone, Copy)]
pub struct RailNotYet;
/// What a rail entry offered when it was last updated.
#[derive(Component, Debug, Clone, PartialEq, Eq)]
pub struct RailAvailability(pub Availability);

fn text_node(key: &'static str, role: ColorRole, scale: TypeScale) -> impl Bundle {
    (
        Text::new(key),
        L10nKey(key),
        TextFont {
            font_size: FontSize::Px(14.0),
            ..default()
        },
        TextColor::default(),
        TypeStyleToken(scale),
        ForegroundRole(role),
    )
}

fn icon_node(glyph: &'static str) -> impl Bundle {
    (
        Text::new(glyph),
        TextFont {
            font: FontSource::Handle(opts_gui::fonts::ICON_FONT),
            font_size: FontSize::Px(24.0),
            ..default()
        },
        TextColor::default(),
        ForegroundRole(ColorRole::OnSurfaceVariant),
        RailIcon,
        opts_gui::ui_tree::A11yHidden,
    )
}

fn item_node() -> Node {
    Node {
        min_height: Val::Px(48.0),
        // never squeezed below its content when the rail is taller than the window
        flex_shrink: 0.0,
        padding: UiRect::axes(Val::Px(12.0), Val::Px(6.0)),
        flex_wrap: FlexWrap::Wrap,
        align_content: AlignContent::Center,
        column_gap: Val::Px(12.0),
        align_items: AlignItems::Center,
        ..default()
    }
}

/// Spawns one entry (icon, label, hidden red marker) as a child of `rail`.
fn spawn_entry(
    rail: &mut ChildSpawnerCommands,
    icon: &'static str,
    title_key: &'static str,
    ui_id: String,
) -> Entity {
    let mut entry = rail.spawn((
        Button,
        item_node(),
        BackgroundColor(Color::NONE),
        StateLayerRole(ColorRole::OnSurface),
        RadiusToken(RadiusLevel::Full),
        Tooltip::new(""),
        L10nTooltip(title_key),
        UiId::new(ui_id),
    ));
    entry.with_children(|item| {
        item.spawn(icon_node(icon));
        item.spawn((
            text_node(title_key, ColorRole::OnSurface, TypeScale::BodyMedium),
            RailWideOnly,
            Node {
                flex_grow: 1.0,
                ..default()
            },
        ));
        // A direct child of the entry, so it sits above the disabled scrim (z 1) and stays red.
        item.spawn((
            Text::new(opts_gui::not_yet::MESSAGE),
            L10nKey(opts_gui::l10n::keys::NOT_AVAILABLE),
            TextColor::default(),
            ForegroundRole(ColorRole::Error),
            TypeStyleToken(TypeScale::Label),
            ZIndex(2),
            RailNotYet,
            opts_gui::ui_tree::A11yHidden,
            Node {
                display: Display::None,
                flex_basis: Val::Percent(100.0),
                margin: UiRect::left(Val::Px(36.0)),
                ..default()
            },
        ));
    });
    entry.id()
}

/// Spawns the frame and publishes [`ShellContent`].
pub fn spawn_shell_frame(mut commands: Commands, registry: Res<ScreenRegistry>) {
    let root = commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                left: Val::Px(0.0),
                top: Val::Px(0.0),
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                flex_direction: FlexDirection::Row,
                ..default()
            },
            BackgroundRole(ColorRole::Surface),
            GlobalZIndex(-10),
            ShellRoot,
            Name::new("ShellRoot"),
        ))
        .id();
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
    let main = commands
        .spawn(Node {
            flex_grow: 1.0,
            min_width: Val::Px(0.0),
            // lets the content area shrink below its page and scroll
            min_height: Val::Px(0.0),
            flex_direction: FlexDirection::Column,
            ..default()
        })
        .id();
    let banners = commands
        .spawn((
            Node {
                flex_direction: FlexDirection::Column,
                row_gap: Val::Px(8.0),
                // Banners never take more than half the window: at a large UI scale the page
                // below keeps room and the banners scroll.
                max_height: Val::Percent(50.0),
                flex_shrink: 0.0,
                overflow: Overflow::scroll_y(),
                ..default()
            },
            ShellBanners,
        ))
        .id();
    let content = commands
        .spawn((
            Node {
                flex_grow: 1.0,
                min_height: Val::Px(0.0),
                flex_direction: FlexDirection::Column,
                // A page taller than the window (large UI scale, small window) scrolls.
                overflow: Overflow::scroll_y(),
                ..default()
            },
            FocusRegion(Landmark::Content),
            Name::new("ShellContent"),
        ))
        .id();
    opts_gui::patterns::scroll::wheel_scroll(&mut commands, content);
    opts_gui::patterns::scroll::wheel_scroll(&mut commands, rail);
    opts_gui::patterns::scroll::wheel_scroll(&mut commands, banners);
    commands.entity(main).add_children(&[banners, content]);
    commands.entity(root).add_children(&[rail, main]);
    commands.insert_resource(ShellContent(Some(content)));
}

fn spawn_registered(rail: &mut ChildSpawnerCommands, d: &ScreenDescriptor) {
    let entry = spawn_entry(rail, d.icon, d.title_key, format!("shell.nav.{}", d.id));
    rail.commands().entity(entry).insert(RailItem { id: d.id });
}

mod rail;
pub use rail::{
    hide_shell, rail_click_system, refresh_rail_availability, show_shell, sync_rail_layout,
    sync_rail_not_yet_labels, sync_rail_selection,
};
