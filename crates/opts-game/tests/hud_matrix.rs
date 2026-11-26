//! M-0071: the HUD's layout across window sizes and UI scales (the accessibility matrix applied
//! to the in-game HUD). The cluster at the bottom left, the mini map at the bottom right, the
//! status bar, the pause button, the camera label, the toasts and the viewpoint editor's panel
//! stay inside the window and never overlap, from 1024 x 640 to 2560 x 1440 at a UI scale of 75
//! to 200 per cent, and the HUD passes the text rules.

#[path = "common/hud/mod.rs"]
mod hud;
#[path = "hud_matrix/geometry.rs"]
mod geometry;
#[path = "common/layout.rs"]
mod layout;

use bevy::prelude::*;
use geometry::{hidden, overlap, rect};
use hud::{build_hud, Extras};
use opts_game::hud::VehicleKind;
use opts_gui::testing::rules;

impl Extras {
    /// Everything at once.
    pub const ALL: Self = Self {
        alert: true,
        toast: true,
        camera_label: true,
        editor: true,
    };
}

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

#[path = "hud_matrix/findings.rs"]
mod findings;
use findings::{layout_findings, shown_rects};

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
