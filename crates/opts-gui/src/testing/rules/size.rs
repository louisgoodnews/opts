//! Target and text sizes of the layout: the declared size of an element and the small-target and small-text checks.

use super::{MIN_TARGET, MIN_TEXT_PX};
use crate::ui_id::UiId;
use crate::ui_tree::{UiRole, snapshot};
use bevy::prelude::*;

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
pub(super) fn is_target(role: UiRole) -> bool {
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
