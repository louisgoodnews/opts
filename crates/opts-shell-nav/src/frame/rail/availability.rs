//! Availability of the rail entries: the 'not yet' labels and the refresh of each entry's state.

use super::*;

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
