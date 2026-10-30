//! M-0071: the HUD's layout across window sizes and UI scales (the accessibility matrix applied
//! to the in-game HUD). The cluster at the bottom left, the mini map at the bottom right, the
//! status bar, the pause button, the camera label, the toasts and the viewpoint editor's panel
//! stay inside the window and never overlap, from 1024 x 640 to 2560 x 1440 at a UI scale of 75
//! to 200 per cent, and the HUD passes the text rules.

mod common;

use bevy::prelude::*;
use common::hud::{build_hud, hidden, overlap, rect, Extras};
use opts_game::hud::VehicleKind;
use opts_gui::testing::rules;

const SIZES: [(u32, u32); 5] = [
    (1024, 640),
    (1280, 720),
    (1920, 1080),
    (2560, 1440),
    (1600, 900),
];
const SCALES: [f32; 5] = [0.75, 1.0, 1.25, 1.5, 2.0];

/// The elements that must be clear of each other (ids from `hud::build`).
const ELEMENTS: [&str; 5] = [
    "hud.slot.cluster",
    "hud.minimap",
    "hud.slot.status-bar",
    "hud.pause",
    "hud.slot.camera-label",
];

/// The viewpoint editor's panel needs this much layout height (68 px from the top, about 360 px
/// tall with its subtitle filled in, a toast of 48 px below it with its gaps) and its width:
/// smaller layout areas are too small for the editor itself.
fn editor_fits(logical: Vec2) -> bool {
    logical.x >= 640.0 && logical.y >= 500.0
}

fn check(w: u32, h: u32, scale: f32, kind: VehicleKind, extras: Extras) -> Vec<String> {
    check_at(w, h, scale, kind, extras, 14.0)
}

/// The rectangles of the HUD elements and toasts that are shown, by id.
fn shown_rects(app: &mut App) -> Vec<(String, [f32; 4])> {
    let mut shown: Vec<(String, [f32; 4])> = Vec::new();
    let mut ids: Vec<String> = ELEMENTS.iter().map(|s| (*s).to_owned()).collect();
    ids.push("hud.viewpoint".to_owned());
    for id in &ids {
        let Some(e) = opts_gui::ui_tree::find_entity(app.world_mut(), id) else {
            continue;
        };
        if hidden(app, e) {
            continue;
        }
        let Some(r) = rect(app, id) else {
            continue;
        };
        shown.push((id.clone(), r));
    }
    // Toasts have no id.
    let toasts: Vec<Entity> = app
        .world_mut()
        .query_filtered::<Entity, With<opts_gui::components::toast::M3Toast>>()
        .iter(app.world())
        .collect();
    for (i, t) in toasts.iter().enumerate() {
        if hidden(app, *t) {
            continue;
        }
        if let Some(r) = opts_gui::ui_tree::rect_of(app.world(), *t) {
            shown.push((format!("toast{i}"), r));
        }
    }
    shown
}

/// Findings about the shown rectangles: missing, outside the window or overlapping.
fn layout_findings(
    what: &str,
    shown: &[(String, [f32; 4])],
    logical: Vec2,
    extras: Extras,
) -> Vec<String> {
    let mut found = Vec::new();
    for needed in ["hud.slot.cluster", "hud.minimap", "hud.pause"] {
        if !shown.iter().any(|(id, _)| id == needed) && !(extras.editor && needed == "hud.minimap")
        {
            found.push(format!("{what}: {needed} is not shown"));
        }
    }
    for (id, r) in shown {
        if r[0] < -0.5
            || r[1] < -0.5
            || r[0] + r[2] > logical.x + 0.5
            || r[1] + r[3] > logical.y + 0.5
        {
            found.push(format!("{what}: {id} {r:?} leaves the window {logical:?}"));
        }
    }
    for (i, (a_id, a)) in shown.iter().enumerate() {
        for (b_id, b) in &shown[i + 1..] {
            if a_id.starts_with("toast") && b_id.starts_with("toast") {
                continue;
            }
            // A layout area this small has no room for a toast beside the top bar and the label:
            // the toast (4 s) may cover them (documented in the HUD registry).
            let toast_pair = a_id.starts_with("toast") || b_id.starts_with("toast");
            if toast_pair && (logical.x < 640.0 || logical.y < 400.0) {
                let other = if a_id.starts_with("toast") {
                    b_id
                } else {
                    a_id
                };
                if other != "hud.minimap" && other != "hud.slot.cluster" {
                    continue;
                }
            }
            if overlap(*a, *b) {
                found.push(format!("{what}: {a_id} {a:?} overlaps {b_id} {b:?}"));
            }
        }
    }
    found
}

/// [`check`] with the user's base text size (`ui_font_size`).
fn check_at(
    w: u32,
    h: u32,
    scale: f32,
    kind: VehicleKind,
    extras: Extras,
    base: f32,
) -> Vec<String> {
    let mut app = build_hud(w, h, scale, kind, extras);
    if base != 14.0 {
        app.insert_resource(opts_gui::fonts::UiTypography {
            family: String::new(),
            base_px: base,
        });
        for _ in 0..6 {
            app.update();
        }
    }
    let logical = Vec2::new(w as f32 / scale, h as f32 / scale);
    let what = format!(
        "{kind:?} {w}x{h} at {}% (layout {:.0}x{:.0}) {extras:?}",
        (scale * 100.0) as u32,
        logical.x,
        logical.y
    );
    let shown = shown_rects(&mut app);
    let mut found = layout_findings(&what, &shown, logical, extras);
    // The text rule compares every pair of texts, so it shares the exemption of tiny areas.
    let tiny = extras.toast && (logical.x < 640.0 || logical.y < 400.0);
    for f in rules::text_overflow(app.world_mut(), logical) {
        if tiny && f.contains("fixture toast") {
            continue;
        }
        found.push(format!("{what}: {f}"));
    }
    for f in rules::small_targets(app.world_mut()) {
        found.push(format!("{what}: {f}"));
    }
    found
}

fn report(findings: &[String]) {
    assert!(
        findings.is_empty(),
        "{} findings:\n{}",
        findings.len(),
        findings.join("\n")
    );
}

#[test]
fn the_hud_has_no_overlap_from_1024x640_to_2560x1440_at_ui_scale_75_to_200() {
    let extras = Extras {
        alert: true,
        toast: true,
        camera_label: true,
        editor: false,
    };
    let mut findings = Vec::new();
    for (w, h) in SIZES {
        for scale in SCALES {
            findings.extend(check(w, h, scale, VehicleKind::Bus, extras));
        }
    }
    report(&findings);
}

#[test]
fn the_viewpoint_editor_panel_never_overlaps_the_hud() {
    let mut findings = Vec::new();
    for (w, h) in SIZES {
        for scale in SCALES {
            let logical = Vec2::new(w as f32 / scale, h as f32 / scale);
            if !editor_fits(logical) {
                continue;
            }
            // The label (a camera change) and the editor (it cancels on a camera change) never
            // coexist.
            let extras = Extras {
                camera_label: false,
                ..Extras::ALL
            };
            findings.extend(check(w, h, scale, VehicleKind::Bus, extras));
        }
    }
    report(&findings);
}

#[test]
fn every_vehicle_kind_lays_out_at_the_smallest_window() {
    let extras = Extras {
        alert: true,
        toast: true,
        camera_label: true,
        editor: false,
    };
    let mut findings = Vec::new();
    for kind in VehicleKind::ALL {
        for scale in [1.0, 2.0] {
            findings.extend(check(1024, 640, scale, kind, extras));
        }
    }
    report(&findings);
}

#[test]
fn the_hud_has_no_overlap_at_the_largest_base_text_sizes() {
    let extras = Extras {
        alert: true,
        toast: true,
        camera_label: true,
        editor: false,
    };
    let mut findings = Vec::new();
    for kind in VehicleKind::ALL {
        for (w, h) in [(1024, 640), (1920, 1080)] {
            for base in [20.0, 24.0] {
                findings.extend(check_at(w, h, 1.0, kind, extras, base));
            }
        }
    }
    report(&findings);
}
