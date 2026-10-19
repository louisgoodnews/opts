//! Keyboard and directional traversal checks.

use super::*;

/// What a keyboard-only traversal achieved.
#[derive(Debug, Default)]
pub struct Traversal {
    /// Ids reached by `next`, in order.
    pub reached: Vec<String>,
    /// Focusable elements the traversal never reached.
    pub missed: Vec<String>,
    /// Elements `confirm` activated.
    pub activated: Vec<String>,
    /// Enabled elements (not text entries) that did not react to `confirm`.
    pub inert: Vec<String>,
}

/// Entities that received [`Activated`] since the log was last cleared (filled by
/// [`keyboard_traversal`]'s own system).
#[derive(Resource, Default, Debug)]
pub struct ActivationLog(pub Vec<Entity>);

fn log_activations(mut log: ResMut<ActivationLog>, mut activated: MessageReader<Activated>) {
    log.0.extend(activated.read().map(|a| a.0));
}

/// Walks the UI with `next` only, pressing `confirm` on each element; reports what was missed.
pub fn keyboard_traversal(app: &mut App) -> Traversal {
    if !app.world().contains_resource::<ActivationLog>() {
        app.init_resource::<ActivationLog>()
            .add_systems(Update, log_activations);
    }
    let snap = snapshot(app.world_mut());
    let wanted: Vec<(String, UiRole, bool)> = snap
        .flatten()
        .into_iter()
        .filter(|n| n.focusable && n.state.visible && is_target(n.role))
        .map(|n| (n.id.clone(), n.role, n.state.disabled.is_some()))
        .collect();
    let mut report = Traversal::default();
    // Pass 1: reach everything with `next` alone.
    for _ in 0..wanted.len() + 2 {
        act(app, UiAction::FocusNext);
        let Some(entity) = focused(app) else { continue };
        if let Some(id) = ui_id_of(app.world(), entity) {
            if !report.reached.contains(&id) {
                report.reached.push(id);
            }
        }
    }
    // Pass 2: put focus on each element again and press `confirm`; undo a toggle of "expanded" so
    // later elements stay visible.
    for (id, role, disabled) in &wanted {
        if !report.reached.contains(id) {
            continue;
        }
        let Some(target) = crate::ui_tree::find_entity(app.world_mut(), id) else {
            continue;
        };
        let focus_target = app
            .world()
            .get::<crate::interaction::layer::FocusProxy>(target)
            .map_or(target, |p| p.0);
        app.world_mut()
            .write_message(crate::interaction::focus::FocusEvent::Set(focus_target));
        app.update();
        app.update();
        let before = snapshot(app.world_mut())
            .find(id)
            .and_then(|n| n.state.expanded);
        app.world_mut().resource_mut::<ActivationLog>().0.clear();
        act(app, UiAction::Confirm);
        let hit = app.world().resource::<ActivationLog>().0.contains(&target);
        match (role, disabled) {
            (UiRole::Textbox, _) | (_, true) => {}
            _ if hit => report.activated.push(id.clone()),
            _ => report.inert.push(id.clone()),
        }
        let after = snapshot(app.world_mut())
            .find(id)
            .and_then(|n| n.state.expanded);
        if before.is_some() && before != after {
            act(app, UiAction::Confirm);
        }
    }
    report.missed = wanted
        .into_iter()
        .map(|(id, _, _)| id)
        .filter(|id| !report.reached.contains(id))
        .collect();
    report
}

/// What a gamepad-only traversal (D-pad / left stick = `Navigate`, A = `Confirm`) achieved.
#[derive(Debug, Default)]
pub struct DirectionalTraversal {
    /// Ids reached by following `Navigate(dir)` from the first focus, breadth first.
    pub reached: Vec<String>,
    /// Focusable, visible elements no sequence of directions reaches.
    pub unreachable: Vec<String>,
}

/// Starts at the first focus and follows the four directions from every reached element (the
/// gamepad has no `Tab`): what is not reached cannot be used with a gamepad or a wheel's D-pad.
pub fn directional_traversal(app: &mut App) -> DirectionalTraversal {
    use crate::actions::Direction;
    let wanted: Vec<String> = snapshot(app.world_mut())
        .flatten()
        .into_iter()
        .filter(|n| n.focusable && n.state.visible && is_target(n.role))
        .map(|n| n.id.clone())
        .collect();
    let mut out = DirectionalTraversal::default();
    act(app, UiAction::FocusNext);
    let mut queue = std::collections::VecDeque::new();
    if let Some(id) = focused(app).and_then(|e| ui_id_of(app.world(), e)) {
        out.reached.push(id.clone());
        queue.push_back(id);
    }
    while let Some(id) = queue.pop_front() {
        for dir in [
            Direction::Down,
            Direction::Right,
            Direction::Up,
            Direction::Left,
        ] {
            let Some(target) = crate::ui_tree::find_entity(app.world_mut(), &id) else {
                continue;
            };
            let proxy = app
                .world()
                .get::<crate::interaction::layer::FocusProxy>(target)
                .map_or(target, |p| p.0);
            app.world_mut()
                .write_message(crate::interaction::focus::FocusEvent::Set(proxy));
            app.update();
            app.update();
            act(app, UiAction::Navigate(dir));
            if let Some(next) = focused(app).and_then(|e| ui_id_of(app.world(), e)) {
                if !out.reached.contains(&next) {
                    out.reached.push(next.clone());
                    queue.push_back(next);
                }
            }
        }
    }
    out.unreachable = wanted
        .into_iter()
        .filter(|id| !out.reached.contains(id))
        .collect();
    out
}

fn ui_id_of(world: &World, entity: Entity) -> Option<String> {
    world
        .get::<UiId>(entity)
        .or_else(|| {
            world
                .get::<crate::components::text_field::TextFieldInput>(entity)
                .and_then(|i| world.get::<UiId>(i.0))
        })
        .map(|u| u.0.to_string())
}
