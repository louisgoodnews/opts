//! The checks of the script-effect tests: the passing objects, the seeded fault and the broken scripts.

use opts_benchmark::scenery::worker::Harness;
use opts_benchmark::scenery::{object, object_key, ObjectEntry, Record, Verdict};

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
pub(crate) fn check_toggle(h: &mut Harness) {
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
pub(crate) fn check_slider_head_and_lamp(h: &mut Harness) {
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
pub(crate) fn check_faults(h: &mut Harness) {
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
