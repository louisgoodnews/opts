//! The CI rules of `docs/src/roadmap/phase-9/expandability.md` section 7, as functions over a
//! running UI so the library, the game screens and any later screen test the same things.
//!
//! Each function returns the violations as readable lines; an empty list passes. Sizes come from
//! the computed layout when it has run and from the declared styles otherwise (a headless test has
//! no window): that is a floor, the screenshots remain the proof of the real layout.

use std::path::Path;

use bevy::prelude::*;

use crate::actions::UiAction;
use crate::interaction::activation::Activated;
use crate::interaction::state::{Disabled, DisabledReason};
use crate::not_yet::NotAvailableYet;
use crate::testing::{act, focused};
use crate::ui_id::UiId;
use crate::ui_tree::{role_of, snapshot, UiRole};

/// Smallest interactive target (logical pixels).
pub const MIN_TARGET: f32 = 32.0;
/// Smallest text size (logical pixels).
pub const MIN_TEXT_PX: f32 = 12.0;

fn px(v: Val) -> Option<f32> {
    match v {
        Val::Px(p) => Some(p),
        _ => None,
    }
}

/// Declared size of an element: its own `Px` sizes, else its padding plus its content (text by an
/// estimate, children laid out along the flex direction with their gaps).
fn declared_size(world: &World, entity: Entity, depth: usize) -> Vec2 {
    if depth > 8 {
        return Vec2::ZERO;
    }
    let node = world.get::<Node>(entity);
    let mut content = Vec2::ZERO;
    if let Some(text) = world.get::<Text>(entity) {
        let font = match world.get::<TextFont>(entity).map(|f| f.font_size) {
            Some(FontSize::Px(p)) => p,
            _ => 14.0,
        };
        let longest = text.0.lines().map(|l| l.chars().count()).max().unwrap_or(0);
        content = Vec2::new(
            longest as f32 * font * 0.55,
            font * 1.3 * text.0.lines().count().max(1) as f32,
        );
    }
    let horizontal = node.is_none_or(|n| {
        matches!(
            n.flex_direction,
            FlexDirection::Row | FlexDirection::RowReverse
        )
    });
    let gap = node
        .map(|n| px(if horizontal { n.column_gap } else { n.row_gap }).unwrap_or(0.0))
        .unwrap_or(0.0);
    let kids: Vec<Vec2> = world
        .get::<Children>(entity)
        .into_iter()
        .flat_map(|c| c.iter())
        .filter(|c| {
            world.get::<Node>(*c).is_some_and(|n| {
                n.display != Display::None && n.position_type != PositionType::Absolute
            })
        })
        .map(|c| declared_size(world, c, depth + 1))
        .collect();
    if !kids.is_empty() {
        let gaps = gap * (kids.len() - 1) as f32;
        let along: f32 = kids
            .iter()
            .map(|k| if horizontal { k.x } else { k.y })
            .sum::<f32>()
            + gaps;
        let across = kids
            .iter()
            .map(|k| if horizontal { k.y } else { k.x })
            .fold(0.0, f32::max);
        let kid_size = if horizontal {
            Vec2::new(along, across)
        } else {
            Vec2::new(across, along)
        };
        content = content.max(kid_size);
    }
    let Some(node) = node else { return content };
    let pad = Vec2::new(
        px(node.padding.left).unwrap_or(0.0) + px(node.padding.right).unwrap_or(0.0),
        px(node.padding.top).unwrap_or(0.0) + px(node.padding.bottom).unwrap_or(0.0),
    );
    Vec2::new(
        px(node.width)
            .unwrap_or(content.x + pad.x)
            .max(px(node.min_width).unwrap_or(0.0)),
        px(node.height)
            .unwrap_or(content.y + pad.y)
            .max(px(node.min_height).unwrap_or(0.0)),
    )
}

/// The size of an element in logical pixels: computed when the layout has run, else declared.
pub fn effective_size(world: &World, entity: Entity) -> Vec2 {
    if let Some(c) = world.get::<ComputedNode>(entity) {
        let s = c.size() * c.inverse_scale_factor;
        if s.x > 0.0 && s.y > 0.0 {
            return s;
        }
    }
    declared_size(world, entity, 0)
}

/// Roles that are operated directly (containers and panels are not targets).
fn is_target(role: UiRole) -> bool {
    !matches!(
        role,
        UiRole::Tablist
            | UiRole::Tabpanel
            | UiRole::Dialog
            | UiRole::Toast
            | UiRole::Region
            | UiRole::Tree
    )
}

/// Interactive elements smaller than [`MIN_TARGET`] in either direction.
pub fn small_targets(world: &mut World) -> Vec<String> {
    let snap = snapshot(world);
    let mut out = Vec::new();
    for n in snap.flatten() {
        if !is_target(n.role) || !n.state.visible || !n.focusable {
            continue;
        }
        let Some(entity) = world
            .query::<(Entity, &UiId)>()
            .iter(world)
            .find(|(_, u)| u.0 == n.id)
            .map(|(e, _)| e)
        else {
            continue;
        };
        // A text field is operated through its frame.
        let s = effective_size(world, entity);
        if s.x < MIN_TARGET || s.y < MIN_TARGET {
            out.push(format!(
                "{} ({}) is {:.0}x{:.0}, below {MIN_TARGET}x{MIN_TARGET}",
                n.id,
                n.role.as_str(),
                s.x,
                s.y
            ));
        }
    }
    out
}

/// Visible text below [`MIN_TEXT_PX`].
pub fn small_text(world: &mut World) -> Vec<String> {
    let mut out = Vec::new();
    let mut q = world.query::<(Entity, &Text, &TextFont)>();
    for (e, text, font) in q.iter(world) {
        if text.0.trim().is_empty() {
            continue;
        }
        if let FontSize::Px(p) = font.font_size {
            if p < MIN_TEXT_PX {
                out.push(format!(
                    "text {:?} on {e:?} is {p} px, below {MIN_TEXT_PX}",
                    text.0
                ));
            }
        }
    }
    out
}

/// Disabled controls that give neither a reason nor the not-yet marker.
pub fn disabled_without_reason(world: &mut World) -> Vec<String> {
    let mut out = Vec::new();
    let mut q = world.query_filtered::<(
        Entity,
        Option<&UiId>,
        Has<DisabledReason>,
        Has<NotAvailableYet>,
    ), With<Disabled>>();
    for (e, id, reason, not_yet) in q.iter(world) {
        if !reason && !not_yet {
            out.push(format!(
                "{} is disabled without a reason",
                id.map_or(format!("{e:?}"), |i| i.0.to_string())
            ));
        }
    }
    out
}

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

/// Hard-coded colours in library source outside the token and theme files.
pub fn hardcoded_colours(src_dir: &Path) -> Vec<String> {
    const BANNED: &[&str] = &[
        "Color::srgb",
        "Color::srgba",
        "Color::rgb",
        "Color::rgba",
        "Color::hsl",
        "Color::hsla",
        "Color::linear",
        "Color::BLACK",
        "Color::WHITE",
        "Srgba::",
        "LinearRgba::",
        "Color::hex",
    ];
    fn walk(dir: &Path, out: &mut Vec<std::path::PathBuf>) {
        for e in std::fs::read_dir(dir).into_iter().flatten().flatten() {
            let p = e.path();
            if p.is_dir() {
                walk(&p, out);
            } else if p.extension().is_some_and(|x| x == "rs") {
                out.push(p);
            }
        }
    }
    let mut files = Vec::new();
    walk(src_dir, &mut files);
    let mut bad = Vec::new();
    for f in files {
        let rel = f
            .strip_prefix(src_dir)
            .unwrap_or(&f)
            .to_string_lossy()
            .replace('\\', "/");
        if rel.starts_with("theme/") || rel.starts_with("tokens/") || rel.starts_with("testing/") {
            continue;
        }
        let text = std::fs::read_to_string(&f).unwrap_or_default();
        let body = text.split("#[cfg(test)]").next().unwrap_or("");
        for (n, line) in body.lines().enumerate() {
            let code = line.split("//").next().unwrap_or("");
            if BANNED.iter().any(|b| code.contains(b)) {
                bad.push(format!("{rel}:{}: {}", n + 1, code.trim()));
            }
        }
    }
    bad
}

/// All roles an element may report (a guard that [`role_of`] stays total for the fixtures).
pub fn roles_of_all(world: &mut World) -> Vec<UiRole> {
    let mut q = world.query::<EntityRef>();
    q.iter(world).filter_map(|e| role_of(&e)).collect()
}
