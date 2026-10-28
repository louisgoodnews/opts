//! Rule checking for text overflow, clipping, container escape and overlap.

use bevy::prelude::*;

use crate::ui_id::UiId;
use crate::ui_tree::rect_of;

use super::contrast::{hidden, in_tooltip};

/// Text that does not fit: wider than its container, cut by a clipping ancestor, outside the
/// window, or overlapping another text. Ellipsis with a tooltip is the only allowed truncation
/// (design language section 5 and expandability section 7.4), so a text that already carries
/// [`Tooltip`](crate::interaction::tooltip::Tooltip) and a single line is exempt from the
/// "wider than container" check.
///
/// Meant for the pseudo-locale (`en-XA`, 40 % longer) and long German strings: the same screen is
/// laid out in the locale under test and checked at the window size given.
pub fn text_overflow(world: &mut World, window: Vec2) -> Vec<String> {
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
