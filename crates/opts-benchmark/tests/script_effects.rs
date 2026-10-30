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

mod common;

use std::path::Path;

use common::{add_anchor, add_box, add_sco, add_script, config};
use opts_benchmark::logging;
use opts_benchmark::scenery::worker::Harness;
use opts_benchmark::scenery::{object, object_key, ObjectEntry, Record, Verdict};

const HEAD: &str = "[groups]\n1\nTest\n\n[friendlyname]\nt\n\n";

/// The anchor and the boxes the scripted objects are built from.
fn add_boxes(root: &Path) {
    add_anchor(root);
    add_box(
        root,
        "base",
        [-4.0, 0.0, -4.0],
        [4.0, 3.0, 4.0],
        [150, 150, 150],
    );
    add_box(
        root,
        "flag",
        [-1.0, 3.0, -1.0],
        [1.0, 6.0, 1.0],
        [220, 40, 40],
    );
    add_box(
        root,
        "pole",
        [-0.5, 0.0, -3.0],
        [0.5, 8.0, -2.0],
        [90, 90, 90],
    );
    add_box(
        root,
        "red",
        [-0.8, 6.0, -1.0],
        [0.8, 7.5, 1.0],
        [255, 30, 30],
    );
    add_box(
        root,
        "yellow",
        [-0.8, 4.0, -1.0],
        [0.8, 5.5, 1.0],
        [250, 220, 30],
    );
    add_box(
        root,
        "green",
        [-0.8, 2.0, -1.0],
        [0.8, 3.5, 1.0],
        [30, 220, 60],
    );
}

/// The toggle and slider objects with their scripts.
fn add_toggle_and_slider(root: &Path) {
    add_sco(
        root,
        "toggle",
        &format!(
            "{HEAD}[mesh]\nbase.x\n\n[mesh]\nflag.x\n\n[visible]\nshow\n1\n\n[script]\n1\nscript\\toggle.osc\n"
        ),
    );
    add_script(
        root,
        "toggle.osc",
        "{frame}\n (L.S.Time) 64800 >= (S.L.show)\n{end}\n",
    );
    add_sco(
        root,
        "slider",
        &format!(
            "{HEAD}[mesh]\nbase.x\n\n[mesh]\nflag.x\n\n[newanim]\norigin_trans\n0\n0\n0\nanim_trans\nslide\n4\n\n[script]\n1\nscript\\slide.osc\n"
        ),
    );
    add_script(
        root,
        "slide.osc",
        "{frame}\n (L.S.Month) 3 < (S.L.slide)\n{end}\n",
    );
}

/// The traffic head and the lamp with their scripts.
fn add_head_and_lamp(root: &Path) {
    add_sco(
        root,
        "head",
        &format!(
            "{HEAD}[trafficlight]\n\n[mesh]\npole.x\n\n[mesh]\nred.x\n\n[visible]\nred\n1\n\n\
             [mesh]\nyellow.x\n\n[visible]\nyellow\n1\n\n[mesh]\ngreen.x\n\n[visible]\ngreen\n1\n\n\
             [script]\n1\nscript\\head.osc\n"
        ),
    );
    add_script(
        root,
        "head.osc",
        "{frame}\n (L.L.TrafficLightPhase) 0 = (S.L.red)\n (L.L.TrafficLightPhase) 3 = (S.L.yellow)\n \
         (L.L.TrafficLightPhase) 6 = (S.L.green)\n{end}\n",
    );
    add_sco(
        root,
        "lamp",
        &format!(
            "{HEAD}[mesh]\nbase.x\n\n[light_enh]\n0\n0\n6\n255\n200\n100\n2\nlamp_on\n0\n0\n0\n0\n\n\
             [script]\n1\nscript\\lamp.osc\n"
        ),
    );
    add_script(
        root,
        "lamp.osc",
        "{frame}\n (L.S.Time) 64800 >= (L.S.Time) 21600 < || (S.L.lamp_on)\n{end}\n",
    );
}

/// The objects without a working script: a missing script file and an inert one.
fn add_broken_scripts(root: &Path) {
    add_sco(
        root,
        "missing",
        &format!("{HEAD}[mesh]\nbase.x\n\n[script]\n1\nscript\\nothere.osc\n"),
    );
    add_sco(
        root,
        "inert",
        &format!(
            "{HEAD}[mesh]\nbase.x\n\n[mesh]\nflag.x\n\n[visible]\nnever\n1\n\n[script]\n1\nscript\\idle.osc\n"
        ),
    );
    add_script(root, "idle.osc", "{frame}\n 0 (S.L.other)\n{end}\n");
}

fn build_root(root: &Path) {
    add_boxes(root);
    add_toggle_and_slider(root);
    add_head_and_lamp(root);
    add_broken_scripts(root);
}

fn run(h: &mut Harness, name: &str, fault: Option<&str>) -> Record {
    h.config.seed_fault = fault.map(String::from);
    let sco = format!("Sceneryobjects/Test/{name}.sco");
    let entry = ObjectEntry {
        key: object_key(&sco),
        sco,
        tier: "unreferenced".into(),
    };
    object::process(h, &entry).0
}

fn live_cats(r: &Record) -> Vec<&str> {
    r.categories.iter().map(String::as_str).collect()
}

/// The toggle object passes, and the same object twice gives the same result.
fn check_toggle(h: &mut Harness) {
    let toggle = run(h, "toggle", None);
    let check = toggle.script_check.clone().unwrap();
    assert_eq!(
        toggle.verdict,
        Verdict::Pass,
        "{:?} {check:?}",
        live_cats(&toggle)
    );
    assert_eq!(check.status, "ok", "{check:?}");
    assert_eq!(check.states, ["s0", "s1", "s2", "s3", "s0b"]);
    assert_eq!((check.stimuli, check.live), (1, 1), "{check:?}");
    assert!(check.changed_vars.contains(&"show".to_string()));
    assert_eq!(check.nondeterministic, Some(0.0));

    // The same object twice gives the same result (the harness is deterministic).
    let again = run(h, "toggle", None).script_check.unwrap();
    assert_eq!(
        (
            again.stimuli,
            again.live,
            again.states.clone(),
            again.nondeterministic
        ),
        (
            check.stimuli,
            check.live,
            check.states.clone(),
            check.nondeterministic
        )
    );
}

/// The slider, traffic head and lamp objects pass.
fn check_slider_head_and_lamp(h: &mut Harness) {
    let slider = run(h, "slider", None);
    let check = slider.script_check.clone().unwrap();
    assert_eq!(
        slider.verdict,
        Verdict::Pass,
        "{:?} {check:?}",
        live_cats(&slider)
    );
    assert_eq!((check.stimuli, check.live), (1, 1), "{check:?}");
    assert_eq!(check.channels, ["anim"]);

    let head = run(h, "head", None);
    let check = head.script_check.clone().unwrap();
    assert_eq!(
        head.verdict,
        Verdict::Pass,
        "{:?} {check:?}",
        live_cats(&head)
    );
    assert!(check.states.contains(&"s4_12".to_string()), "{check:?}");
    assert_eq!((check.stimuli, check.live), (3, 3), "{check:?}");

    let lamp = run(h, "lamp", None);
    let check = lamp.script_check.clone().unwrap();
    assert_eq!(
        lamp.verdict,
        Verdict::Pass,
        "{:?} {check:?}",
        live_cats(&lamp)
    );
    assert_eq!((check.light_stimuli, check.light_live), (1, 1), "{check:?}");
}

/// The seeded fault, the missing script and the inert script are reported.
fn check_faults(h: &mut Harness) {
    // Seeded fault: the script runs and its variable changes, but nothing is applied.
    let faulty = run(h, "toggle", Some("script-effect-off"));
    assert_eq!(faulty.verdict, Verdict::Fail, "{:?}", faulty.script_check);
    assert!(
        faulty.categories.iter().any(|c| c == "S-EFFECT-NOT-SHOWN"),
        "{:?} {:?}",
        faulty.categories,
        faulty.script_check
    );

    let missing = run(h, "missing", None);
    assert!(
        missing.categories.iter().any(|c| c == "S-MISSING"),
        "{:?}",
        missing.categories
    );

    let inert = run(h, "inert", None);
    assert_eq!(inert.verdict, Verdict::Unknown, "{:?}", inert.script_check);
    assert_eq!(inert.tags.script.as_deref(), Some("S-INERT"));
}

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
