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
            flex_direction: FlexDirection::Column,
            ..default()
        })
        .id();
    let banners = commands
        .spawn((
            Node {
                flex_direction: FlexDirection::Column,
                row_gap: Val::Px(8.0),
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
                ..default()
            },
            FocusRegion(Landmark::Content),
            Name::new("ShellContent"),
        ))
        .id();
    commands.entity(main).add_children(&[banners, content]);
    commands.entity(root).add_children(&[rail, main]);
    commands.insert_resource(ShellContent(Some(content)));
}

fn spawn_registered(rail: &mut ChildSpawnerCommands, d: &ScreenDescriptor) {
    let entry = spawn_entry(rail, d.icon, d.title_key, format!("shell.nav.{}", d.id));
    rail.commands().entity(entry).insert(RailItem { id: d.id });
}

/// Hides the frame and releases [`ShellContent`] when a drive starts.
pub fn hide_shell(mut content: ResMut<ShellContent>, mut roots: Query<&mut Node, With<ShellRoot>>) {
    content.0 = None;
    for mut node in &mut roots {
        node.display = Display::None;
    }
}

/// Shows the frame again when a drive ends and points [`ShellContent`] at the content area.
pub fn show_shell(
    mut content: ResMut<ShellContent>,
    mut roots: Query<&mut Node, With<ShellRoot>>,
    areas: Query<(Entity, &Name)>,
) {
    for mut node in &mut roots {
        node.display = Display::Flex;
    }
    content.0 = areas
        .iter()
        .find(|(_, n)| n.as_str() == "ShellContent")
        .map(|(e, _)| e);
}

/// Rail width and label visibility follow the layout class.
pub fn sync_rail_layout(
    class: Res<LayoutClass>,
    mut rails: Query<&mut Node, With<ShellRail>>,
    mut wide: Query<&mut Node, (With<RailWideOnly>, Without<ShellRail>)>,
    mut added: Local<Option<LayoutClass>>,
) {
    if *added == Some(*class) && !class.is_added() {
        return;
    }
    *added = Some(*class);
    let compact = *class == LayoutClass::Compact;
    for mut rail in &mut rails {
        rail.width = Val::Px(if compact {
            RAIL_COMPACT_PX
        } else {
            RAIL_WIDE_PX
        });
    }
    for mut node in &mut wide {
        node.display = if compact {
            Display::None
        } else {
            Display::Flex
        };
    }
}

/// Marks the entry of the current destination as selected (container colour and marker).
pub fn sync_rail_selection(
    state: Res<State<AppState>>,
    registry: Res<ScreenRegistry>,
    mut commands: Commands,
    items: Query<(Entity, &RailItem, Has<Selected>)>,
    children: Query<&Children>,
    mut fg: Query<&mut ForegroundRole>,
) {
    let current = registry.by_state(*state.get()).map(|s| s.id);
    for (entity, item, selected) in &items {
        let want = Some(item.id) == current;
        if want == selected && !state.is_changed() {
            continue;
        }
        let mut e = commands.entity(entity);
        if want {
            e.insert((
                Selected,
                BackgroundRole(ColorRole::SecondaryContainer),
                StateLayerRole(ColorRole::OnSecondaryContainer),
            ));
        } else {
            e.remove::<(Selected, BackgroundRole)>();
            e.insert((
                BackgroundColor(Color::NONE),
                StateLayerRole(ColorRole::OnSurface),
            ));
        }
        for child in children.iter_descendants(entity) {
            if let Ok(mut role) = fg.get_mut(child) {
                if role.0 != ColorRole::Error {
                    role.0 = match (want, role.0) {
                        (true, _) => ColorRole::OnSecondaryContainer,
                        (false, ColorRole::OnSecondaryContainer) => ColorRole::OnSurface,
                        (false, other) => other,
                    };
                }
            }
        }
    }
}

/// Re-evaluates every destination's availability a few times a second and applies it.
pub fn refresh_rail_availability(world: &mut World, mut frame: Local<u32>) {
    *frame += 1;
    if !(*frame - 1).is_multiple_of(15) {
        return;
    }
    let items: Vec<(Entity, &'static str, Option<Availability>)> = world
        .query::<(Entity, &RailItem, Option<&RailAvailability>)>()
        .iter(world)
        .map(|(e, i, a)| (e, i.id, a.map(|a| a.0.clone())))
        .collect();
    for (entity, id, last) in items {
        let Some(descriptor) = world.resource::<ScreenRegistry>().get(id).copied() else {
            continue;
        };
        let now = (descriptor.availability)(world);
        if last.as_ref() == Some(&now) {
            continue;
        }
        apply_availability(world, entity, &now);
        world.entity_mut(entity).insert(RailAvailability(now));
    }
}

/// Puts an availability on a rail entry. The red text is the entry's own line (it is hidden when
/// the rail is narrow; the tooltip then carries the reason).
fn apply_availability(world: &mut World, entity: Entity, availability: &Availability) {
    let mut e = world.entity_mut(entity);
    e.remove::<(Disabled, DisabledReason, NotAvailableYet)>();
    match availability {
        Availability::Enabled => {}
        Availability::Disabled(reason) => {
            e.insert((Disabled, DisabledReason(reason.clone())));
        }
        Availability::NotYet(place) => {
            e.insert(NotAvailableYet::new(place).without_label());
        }
    }
}

/// Entries whose availability just changed.
type NotYetChanged = Or<(Added<NotAvailableYet>, Changed<RailAvailability>)>;

/// Shows the red line of a not-yet entry when the rail is wide.
pub fn sync_rail_not_yet_labels(
    class: Res<LayoutClass>,
    entries: Query<Has<NotAvailableYet>, With<RailItem>>,
    items: Query<Entity, With<RailItem>>,
    children: Query<&Children>,
    mut labels: Query<(Entity, &mut Node), With<RailNotYet>>,
    mut commands: Commands,
    changed: Query<(), NotYetChanged>,
) {
    if !class.is_changed() && changed.is_empty() {
        return;
    }
    for entity in &items {
        let not_yet = entries.get(entity).unwrap_or(false);
        for child in children.iter_descendants(entity) {
            if let Ok((label, mut node)) = labels.get_mut(child) {
                let shown = not_yet && *class != LayoutClass::Compact;
                node.display = if shown { Display::Flex } else { Display::None };
                // A hidden marker must not be part of the entry's accessible name.
                if shown {
                    commands
                        .entity(label)
                        .remove::<opts_gui::ui_tree::A11yHidden>();
                } else {
                    commands.entity(label).insert(opts_gui::ui_tree::A11yHidden);
                }
            }
        }
    }
}

/// A rail click opens its destination (a flat jump).
pub fn rail_click_system(
    mut activated: MessageReader<opts_gui::interaction::activation::Activated>,
    items: Query<(&RailItem, Has<Disabled>)>,
    mut navigate: MessageWriter<nav::Navigate>,
) {
    for opts_gui::interaction::activation::Activated(entity) in activated.read() {
        if let Ok((item, false)) = items.get(*entity) {
            navigate.write(nav::Navigate::to(item.id));
        }
    }
}
