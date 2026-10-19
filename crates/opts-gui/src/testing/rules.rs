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
use crate::theme::contrast::{composite_over, contrast_ratio};
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

/// Result of [`text_contrast`].
#[derive(Debug, Default)]
pub struct ContrastReport {
    /// Text and border pairs that fall below the threshold.
    pub violations: Vec<String>,
    /// Pairs measured.
    pub checked: usize,
    /// Texts drawn over an image: not measurable from the tree (the scrim rule and the screenshots
    /// cover them).
    pub over_image: usize,
}

mod contrast;
pub use contrast::text_contrast;
use contrast::{hidden, in_tooltip};

/// Text that does not fit: wider than its container, cut by a clipping ancestor, outside the
/// window, or overlapping another text. Ellipsis with a tooltip is the only allowed truncation
/// (design language section 5 and expandability section 7.4), so a text that already carries
/// [`Tooltip`](crate::interaction::tooltip::Tooltip) and a single line is exempt from the
/// "wider than container" check.
///
/// Meant for the pseudo-locale (`en-XA`, 40 % longer) and long German strings: the same screen is
/// laid out in the locale under test and checked at the window size given.
pub fn text_overflow(world: &mut World, window: Vec2) -> Vec<String> {
    use crate::ui_tree::rect_of;
    let mut q = world.query::<(Entity, &Text)>();
    let texts: Vec<(Entity, String)> = q
        .iter(world)
        .filter(|(_, t)| !t.0.trim().is_empty())
        .map(|(e, t)| (e, t.0.clone()))
        .collect();
    let mut rects: Vec<(Entity, String, [f32; 4])> = Vec::new();
    let mut out = Vec::new();
    for (e, text) in texts {
        if hidden(world, e) || in_tooltip(world, e) {
            continue; // a tooltip floats over the page by design
        }
        let Some(r) = rect_of(world, e) else { continue };
        let name = world
            .get::<UiId>(e)
            .map_or(format!("{text:?}"), |u| format!("{text:?} ({})", u.0));
        // Every clipping ancestor, and the direct parent's box.
        let mut clips: Vec<([f32; 4], Overflow)> = Vec::new();
        let mut cur = world.get::<ChildOf>(e).map(|c| c.parent());
        let parent_rect = cur.and_then(|p| rect_of(world, p));
        while let Some(a) = cur {
            if let Some(n) = world.get::<Node>(a) {
                if n.overflow.x != OverflowAxis::Visible || n.overflow.y != OverflowAxis::Visible {
                    if let Some(c) = rect_of(world, a) {
                        clips.push((c, n.overflow));
                    }
                }
            }
            cur = world.get::<ChildOf>(a).map(|c| c.parent());
        }
        let tolerance = 1.5;
        let has_tooltip = world
            .get::<crate::interaction::tooltip::Tooltip>(e)
            .is_some();
        // What a player can see of the text: its box cut by every clipping ancestor.
        let mut seen = r;
        for (c, _) in &clips {
            let x0 = seen[0].max(c[0]);
            let y0 = seen[1].max(c[1]);
            let x1 = (seen[0] + seen[2]).min(c[0] + c[2]);
            let y1 = (seen[1] + seen[3]).min(c[1] + c[3]);
            seen = [x0, y0, (x1 - x0).max(0.0), (y1 - y0).max(0.0)];
        }
        if seen[2] <= 0.0 || seen[3] <= 0.0 {
            continue; // scrolled out of view
        }
        let cut_sideways = clips.iter().any(|(c, o)| {
            o.x != OverflowAxis::Visible
                && o.x != OverflowAxis::Scroll
                && (r[0] < c[0] - tolerance || r[0] + r[2] > c[0] + c[2] + tolerance)
        });
        if cut_sideways && !has_tooltip {
            out.push(format!("{name} is cut sideways by a clipping container"));
        } else if let Some(p) = parent_rect {
            if !has_tooltip && clips.is_empty() && r[0] + r[2] > p[0] + p[2] + tolerance {
                out.push(format!(
                    "{name} sticks out of its container by {:.0} px (right)",
                    r[0] + r[2] - p[0] - p[2]
                ));
            }
        }
        if seen[0] < -tolerance
            || seen[1] < -tolerance
            || seen[0] + seen[2] > window.x + tolerance
            || seen[1] + seen[3] > window.y + tolerance
        {
            out.push(format!(
                "{name} leaves the {}x{} window ({:.0},{:.0} {:.0}x{:.0})",
                window.x, window.y, seen[0], seen[1], seen[2], seen[3]
            ));
        }
        let r = seen;
        rects.push((e, name, r));
    }
    for (i, (ea, na, a)) in rects.iter().enumerate() {
        for (eb, nb, b) in &rects[i + 1..] {
            let w = (a[0] + a[2]).min(b[0] + b[2]) - a[0].max(b[0]);
            let h = (a[1] + a[3]).min(b[1] + b[3]) - a[1].max(b[1]);
            if w > 2.0 && h > 2.0 {
                // Texts on different layers (a dialog over a page) are expected to overlap.
                let layer = |e: Entity| {
                    let mut cur = Some(e);
                    while let Some(x) = cur {
                        if world
                            .get::<crate::components::modal::ModalRoot>(x)
                            .is_some()
                        {
                            return Some(x);
                        }
                        cur = world.get::<ChildOf>(x).map(|c| c.parent());
                    }
                    None
                };
                if layer(*ea) == layer(*eb) {
                    out.push(format!("{na} overlaps {nb} ({w:.0}x{h:.0} px)"));
                }
            }
        }
    }
    out
}

/// Interactive elements a screen reader could not name: no accessible label (UI-P10, WCAG 4.1.2).
/// Text entries are named by their placeholder or label; icon-only buttons need a tooltip or key.
pub fn unnamed_controls(world: &mut World) -> Vec<String> {
    let snap = snapshot(world);
    snap.flatten()
        .into_iter()
        .filter(|n| n.focusable && n.state.visible && is_target(n.role))
        .filter(|n| n.label.trim().is_empty())
        .map(|n| format!("{} ({}) has no accessible name", n.id, n.role.as_str()))
        .collect()
}

/// Every focusable element has an [`AccessibilityNode`](bevy::a11y::AccessibilityNode) with the
/// same name the tree reports (what the platform's accessibility API receives).
pub fn missing_accessibility_nodes(world: &mut World) -> Vec<String> {
    let snap = snapshot(world);
    let mut out = Vec::new();
    for n in snap
        .flatten()
        .into_iter()
        .filter(|n| n.focusable && n.state.visible && is_target(n.role))
    {
        let Some(e) = crate::ui_tree::find_entity(world, &n.id) else {
            continue;
        };
        match world.get::<bevy::a11y::AccessibilityNode>(e) {
            None => out.push(format!("{} has no AccessibilityNode", n.id)),
            Some(a) if a.label().unwrap_or_default() != n.label => out.push(format!(
                "{}: accessibility label {:?} differs from {:?}",
                n.id,
                a.label(),
                n.label
            )),
            Some(_) => {}
        }
    }
    out
}

mod traversal;
pub use traversal::{
    directional_traversal, keyboard_traversal, ActivationLog, DirectionalTraversal, Traversal,
};

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
