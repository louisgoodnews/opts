//! Scenery script loading and per-instance engines (`docs/src/architecture/scenery-runtime.md`).
//! The scripts are self-written fixtures, not OMSI files.

use std::path::{Path, PathBuf};

use opts_core::formats::scenery::SceneryObject;
use opts_sim::script::{load_scenery_scripts, Severity};

/// A lamp that is lit between 18:00 and 06:00 (`Time`) and a hand that follows `knob`.
const LAMP_OSC: &str = "\
'self-written test script
{init}
    0 (S.L.lamp_on)
{end}
{frame}
    (L.S.Time) 64800 >= (L.S.Time) 21600 < || (S.L.lamp_on)
    (L.L.knob) 90 * (S.L.hand_angle)
    (L.S.Timegap) (L.L.ticks) + (S.L.ticks)
{end}
";

const VARLIST: &str = "lamp_on\nhand_angle\nknob\nticks\n";
const STRVARLIST: &str = "label\nsubtitle\n";

const SCO: &str = "\
[script]
1
script\\lamp.osc

[varnamelist]
1
script\\lamp_varlist.txt

[stringvarnamelist]
1
script\\lamp_stringvarlist.txt
";

fn fixture_dir(tag: &str) -> PathBuf {
    let dir =
        std::env::temp_dir().join(format!("opts-scenery-scripts-{}-{tag}", std::process::id()));
    let script = dir.join("script");
    std::fs::create_dir_all(&script).expect("temp dir");
    std::fs::write(script.join("lamp.osc"), LAMP_OSC).expect("write");
    std::fs::write(script.join("lamp_varlist.txt"), VARLIST).expect("write");
    std::fs::write(script.join("lamp_stringvarlist.txt"), STRVARLIST).expect("write");
    dir
}

fn resolver(dir: &Path) -> impl FnMut(&str) -> Option<PathBuf> + '_ {
    move |rel: &str| {
        let p = dir.join(rel.replace('\\', "/"));
        p.exists().then_some(p)
    }
}

#[test]
fn a_scenery_script_switches_on_time_and_animates_on_a_variable() {
    let dir = fixture_dir("time");
    let sco = SceneryObject::from_text(SCO).expect("sco parses");
    let set = load_scenery_scripts(&sco, resolver(&dir));
    assert!(
        set.diagnostics
            .iter()
            .all(|d| d.severity != Severity::Error),
        "{:?}",
        set.diagnostics
    );
    assert_eq!(set.string_names, ["label", "subtitle"]);
    let mut e = set.engine;
    e.run_init();
    e.set_system("Time", 12.0 * 3600.0);
    e.run_frame(1.0 / 60.0, false);
    assert_eq!(e.get("lamp_on"), 0.0);
    e.set_system("Time", 23.0 * 3600.0);
    e.set("knob", 0.5);
    e.run_frame(1.0 / 60.0, false);
    assert_eq!(e.get("lamp_on"), 1.0);
    assert_eq!(e.get("hand_angle"), 45.0);
    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn forked_engines_share_the_program_but_not_the_variables() {
    let dir = fixture_dir("fork");
    let sco = SceneryObject::from_text(SCO).expect("sco parses");
    let template = load_scenery_scripts(&sco, resolver(&dir)).engine;
    let mut a = template.fork();
    let mut b = template.fork();
    let knob = a.variable_id("knob");
    a.set_id(knob, 1.0);
    a.run_frame(0.5, false);
    b.run_frame(0.25, false);
    assert_eq!(a.get("hand_angle"), 90.0);
    assert_eq!(b.get("hand_angle"), 0.0);
    assert_eq!((a.get("ticks"), b.get("ticks")), (0.5, 0.25));
    // Fixed ids address the same variable as names.
    let time = b.system_id("Time");
    b.set_system_id(time, 20.0 * 3600.0);
    b.run_frame(0.0, false);
    assert_eq!(b.get("lamp_on"), 1.0);
    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn missing_files_are_warnings() {
    let sco = SceneryObject::from_text(SCO).expect("sco parses");
    let set = load_scenery_scripts(&sco, |_| None);
    assert_eq!(set.diagnostics.len(), 3, "{:?}", set.diagnostics);
    assert!(set
        .diagnostics
        .iter()
        .all(|d| d.severity == Severity::Warning));
}
