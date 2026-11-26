//! The HUD layout at the smallest window (every vehicle kind) and at the largest base text
//! sizes. Helpers come from the parent `hud_matrix` module.

use super::*;

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
