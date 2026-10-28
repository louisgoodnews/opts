//! Navigation rail layout, selection, availability, and click systems for the shell frame.

use super::*;

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
