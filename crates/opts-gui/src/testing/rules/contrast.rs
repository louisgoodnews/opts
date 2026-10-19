//! Contrast and colour checks: the resolved background of a node and the text-contrast report.

use super::*;

/// Is `entity` or an ancestor hidden (`Display::None` or `Visibility::Hidden`)?
pub(super) fn hidden(world: &World, entity: Entity) -> bool {
    let mut cur = Some(entity);
    while let Some(e) = cur {
        if world
            .get::<Node>(e)
            .is_some_and(|n| n.display == Display::None)
            || world
                .get::<Visibility>(e)
                .is_some_and(|v| *v == Visibility::Hidden)
        {
            return true;
        }
        cur = world.get::<ChildOf>(e).map(|c| c.parent());
    }
    false
}

/// Is `entity` inside the floating tooltip?
pub(super) fn in_tooltip(world: &World, entity: Entity) -> bool {
    let mut cur = Some(entity);
    while let Some(e) = cur {
        if world
            .get::<crate::interaction::tooltip::TooltipNode>(e)
            .is_some()
        {
            return true;
        }
        cur = world.get::<ChildOf>(e).map(|c| c.parent());
    }
    false
}

/// Is `entity` or an ancestor disabled (WCAG exempts inactive components, except the red
/// not-available-yet marker, which must stay readable)?
fn inactive(world: &World, entity: Entity) -> bool {
    let mut cur = Some(entity);
    while let Some(e) = cur {
        if world.get::<NotAvailableYet>(e).is_some() {
            return false;
        }
        if world.get::<Disabled>(e).is_some() {
            return true;
        }
        cur = world.get::<ChildOf>(e).map(|c| c.parent());
    }
    false
}

/// The opaque colour behind `entity` (its own fill included when `include_self`): its ancestors' `BackgroundColor`s composited from the root
/// (`root` is the window background), `None` when an image sits in the chain.
fn resolved_background(
    world: &World,
    entity: Entity,
    root: Color,
    include_self: bool,
) -> Option<Color> {
    let mut chain = Vec::new();
    // The element's own fill counts too (a label with its own chip).
    let mut cur = if include_self {
        Some(entity)
    } else {
        world.get::<ChildOf>(entity).map(|c| c.parent())
    };
    while let Some(e) = cur {
        if world.get::<ImageNode>(e).is_some() {
            return None;
        }
        // The not-yet label is drawn above the disabled scrim of its control, so the scrim lies
        // between the control's fill and the label.
        if world.get::<crate::not_yet::NotYetLabel>(entity).is_some() {
            if let Some(link) = world.get::<crate::interaction::layer::StateLayerLink>(e) {
                if let Some(scrim) = world.get::<BackgroundColor>(link.scrim) {
                    if scrim.0.alpha() > 0.0 {
                        chain.push(scrim.0);
                    }
                }
            }
        }
        if let Some(bg) = world.get::<BackgroundColor>(e) {
            if bg.0.alpha() > 0.0 {
                chain.push(bg.0);
            }
        }
        cur = world.get::<ChildOf>(e).map(|c| c.parent());
    }
    Some(
        chain
            .into_iter()
            .rev()
            .fold(root, |below, above| composite_over(above, below)),
    )
}

/// WCAG contrast of every visible, enabled text against the colour behind it, and of every
/// visible control border (`BorderColor` on a focusable element) against its own surroundings.
///
/// `scheme` is the colour scheme under test: its `Surface` is the window behind the whole tree
/// and its kind selects the thresholds ([`crate::theme::accent::text_min`],
/// [`crate::theme::accent::control_min`]). Borders in `OutlineVariant` are decorative (design
/// language section 4) and exempt. Text of 24 px and
/// above counts as large text and needs 3:1 (4.5:1 in high contrast, where `text_min` is 7).
pub fn text_contrast(world: &mut World, scheme: &crate::theme::ColorScheme) -> ContrastReport {
    let root = scheme.get(crate::theme::ColorRole::Surface);
    let text_min = crate::theme::accent::text_min(scheme.kind);
    let control_min = crate::theme::accent::control_min(scheme.kind);
    let decorative = scheme.get(crate::theme::ColorRole::OutlineVariant);
    let mut report = ContrastReport::default();
    let mut q = world.query::<(Entity, &Text, &TextColor, &TextFont)>();
    let texts: Vec<(Entity, String, Color, f32)> = q
        .iter(world)
        .filter(|(_, t, ..)| !t.0.trim().is_empty())
        .map(|(e, t, c, f)| {
            let size = match f.font_size {
                FontSize::Px(p) => p,
                _ => 14.0,
            };
            (e, t.0.clone(), c.0, size)
        })
        .collect();
    for (e, text, fg, size) in texts {
        if hidden(world, e) || inactive(world, e) {
            continue;
        }
        let Some(bg) = resolved_background(world, e, root, true) else {
            report.over_image += 1;
            continue;
        };
        let fg = composite_over(fg, bg);
        let need = if size >= 24.0 {
            if text_min > 4.5 {
                4.5
            } else {
                3.0
            }
        } else {
            text_min
        };
        let ratio = contrast_ratio(fg, bg);
        report.checked += 1;
        if ratio + 1e-3 < need {
            let id = world.get::<UiId>(e).map(|u| u.0.to_string());
            report.violations.push(format!(
                "text {text:?} ({} px) {} is {ratio:.2}:1, needs {need} ({} on {})",
                size,
                id.unwrap_or_else(|| format!("{e:?}")),
                crate::theme::contrast::to_hex(fg),
                crate::theme::contrast::to_hex(bg)
            ));
        }
    }
    let mut q = world.query::<(
        Entity,
        &BorderColor,
        &Node,
        Has<crate::interaction::focus::Focusable>,
    )>();
    let borders: Vec<(Entity, Color)> = q
        .iter(world)
        .filter(|(_, b, n, focusable)| {
            *focusable
                && b.top.alpha() > 0.0
                && b.top != decorative
                && (n.border.left != Val::ZERO || n.border.top != Val::ZERO)
        })
        .map(|(e, b, ..)| (e, b.top))
        .collect();
    for (e, border) in borders {
        if hidden(world, e) || inactive(world, e) {
            continue;
        }
        let Some(bg) = resolved_background(world, e, root, false) else {
            continue;
        };
        let ratio = contrast_ratio(composite_over(border, bg), bg);
        report.checked += 1;
        if ratio + 1e-3 < control_min {
            report.violations.push(format!(
                "border of {} is {ratio:.2}:1, needs {control_min}",
                world
                    .get::<UiId>(e)
                    .map_or(format!("{e:?}"), |u| u.0.to_string())
            ));
        }
    }
    report
}
