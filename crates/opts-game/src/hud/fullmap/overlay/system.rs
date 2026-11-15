//! The overlay system: builds it when the map opens and keeps it in step with the window and zoom.

use super::*;

/// Builds the overlay when the map opens and removes it when it closes; keeps the texture node,
/// the caption and the scale bar in step with the window and the zoom.
pub fn overlay(
    mut commands: Commands,
    state: Res<FullMapState>,
    image: Res<FullMapImage>,
    roots: Query<Entity, With<FullMapRoot>>,
    nodes: super::params::OverlayNodes,
    env: super::params::OverlayEnv,
) {
    let super::params::OverlayNodes {
        mut surface,
        mut caption,
        mut scale_text,
        mut scale_bar,
    } = nodes;
    let super::params::OverlayEnv {
        session,
        overview,
        loc,
        variant,
        theme,
    } = env;
    let existing = roots.iter().next();
    if !state.open {
        if let Some(root) = existing {
            commands.entity(root).try_despawn();
        }
        return;
    }
    let palette = *theme
        .as_ref()
        .map_or(&opts_gui::theme::InstrumentTokens::default().day, |t| {
            t.instrument(*variant)
        });
    if existing.is_none() {
        let ground = palette.panel;
        spawn_overlay(&mut commands, &palette, ground);
        return;
    }
    let screen = state.screen;
    // The texture covers the square around the screen; the root clips it.
    let side = screen.max_element();
    for (mut node, mut img) in &mut surface {
        let (w, h) = (Val::Px(side), Val::Px(side));
        let (left, top) = (
            Val::Px((screen.x - side) / 2.0),
            Val::Px((screen.y - side) / 2.0),
        );
        if node.width != w || node.height != h || node.left != left || node.top != top {
            node.width = w;
            node.height = h;
            node.left = left;
            node.top = top;
        }
        if let Some(h) = &image.0 {
            if img.image != *h {
                img.image = h.clone();
            }
        }
    }
    let ppm = state.viewport.px_per_m(screen);
    let bar_m = scale_bar_m(SCALE_BAR_MAX_PX / ppm);
    let map = session.as_ref().map_or("", |s| s.map_name.as_str());
    let (title, bar_text) = caption_texts(&loc, &overview, map, bar_m);
    for mut t in &mut caption {
        if t.0 != title {
            t.0.clone_from(&title);
        }
    }
    for mut t in &mut scale_text {
        if t.0 != bar_text {
            t.0.clone_from(&bar_text);
        }
    }
    for mut node in &mut scale_bar {
        let w = Val::Px((bar_m * ppm) as f32);
        if node.width != w {
            node.width = w;
        }
    }
}
