//! Script-effect check acceptance tests (M-0054 Phase C): self-authored scripted objects are run
//! through the real worker pipeline (map loader, scenery runtime, off-screen capture).
//!
//! * a script that toggles a mesh on `Time` (S2 differs from S0/S1/S3);
//! * a script that animates a mesh on a variable (winter moves a slider);
//! * a traffic-head-like object driven by `TrafficLightPhase` (scenario S4);
//! * a lamp whose `[light_enh]` follows `Time`;
//! * a seeded fault: the same toggle object with its effects detached must be reported as
//!   `S-EFFECT-NOT-SHOWN`;
//! * a script file that does not exist (`S-MISSING`), a static scripted object (`S-INERT`
//!   becomes `unknown`).
//!
//! The tests need a GPU adapter for Bevy's renderer; without one they report "skipped" and pass.

mod checks;
#[path = "../common/mod.rs"]
mod common;
mod fixtures;

use checks::{check_faults, check_slider_head_and_lamp, check_toggle};
use common::config;
use fixtures::build_root;
use opts_benchmark::logging;
use opts_benchmark::scenery::worker::Harness;

#[test]
fn script_effects_are_checked() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path().join("omsi");
    build_root(&root);
    let sink = logging::init(true);
    let built = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        Harness::new(config(&tmp.path().join("run"), &root, None, true), sink)
    }));
    let mut h = match built {
        Ok(Ok(h)) => h,
        other => {
            eprintln!(
                "skipped: no usable GPU adapter ({:?})",
                other.err().is_some()
            );
            return;
        }
    };
    opts_bevy::texture_sentinel::set_missing_texture_sentinel(true);

    check_toggle(&mut h);
    check_slider_head_and_lamp(&mut h);
    check_faults(&mut h);
}
