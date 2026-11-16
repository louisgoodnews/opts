//! Contrast of every text and control border of the tree, checked against the colour behind it.

use super::*;

pub(super) fn check_texts_contrast(
    world: &mut World,
    root: Color,
    text_min: f32,
    report: &mut ContrastReport,
) {
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
}

pub(super) fn check_borders_contrast(
    world: &mut World,
    root: Color,
    control_min: f32,
    decorative: Color,
    report: &mut ContrastReport,
) {
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
}
