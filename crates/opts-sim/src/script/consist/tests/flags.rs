//! The larger value wins, and the cab-inactive flag of a traction.

use super::*;

#[test]
fn the_larger_magnitude_wins() {
    let cables = car_cables();
    let (mut a, mut b) = (car(), car());
    a.set("bat_w", 0.3);
    b.set("bat_w", -0.8);
    bridge(&mut [
        Unit {
            engine: Some(&mut a),
            cables: &cables,
        },
        Unit {
            engine: Some(&mut b),
            cables: &cables,
        },
    ]);
    assert_eq!(a.get("bat_r"), -0.8);
}

/// A cab-inactive flag is 1 only while every cab that writes it is inactive: the passive cars of
/// a traction write 1, the driven cab writes 0 and the line reads 0 everywhere; the other lines
/// keep the largest value.
#[test]
fn an_occupied_cab_clears_the_inactive_flag_for_the_whole_train_but_other_lines_take_the_largest() {
    let cable_pair = |receive: &str, send: &str, index: u32| {
        [true, false].map(|front| Cable {
            front,
            kind: 'C',
            index,
            receive: receive.to_owned(),
            send: send.to_owned(),
            coupling: if front { "line_v" } else { "line_h" }.to_owned(),
        })
    };
    let cables: Vec<Cable> = cable_pair("inaktiv_r", "inaktiv_w", 0)
        .into_iter()
        .chain(cable_pair("bat_r", "bat_w", 1))
        .collect();
    // A unit whose scripts write the flag: `from_sources` with a store to the variable.
    let writer = |flag: f32| {
        let src = format!("{{frame}}\n{flag} (S.L.inaktiv_w)\n{{end}}");
        let (mut e, _) = ScriptEngine::from_sources(&[("t.osc", src.as_str())]);
        let names: Vec<String> = [
            "inaktiv_r",
            "inaktiv_w",
            "bat_r",
            "bat_w",
            "line_v",
            "line_h",
        ]
        .iter()
        .map(|n| (*n).to_owned())
        .collect();
        e.declare_variables(&names, false);
        e.set("line_v", 1.0);
        e.set("line_h", 1.0);
        e
    };
    let (mut driven, mut passive) = (writer(0.0), writer(1.0));
    driven.set("bat_w", 1.0);
    bridge(&mut [
        Unit {
            engine: Some(&mut driven),
            cables: &cables,
        },
        Unit {
            engine: Some(&mut passive),
            cables: &cables,
        },
    ]);
    assert_eq!(driven.get("inaktiv_r"), 0.0);
    assert_eq!(passive.get("inaktiv_r"), 0.0, "an occupied cab clears it");
    assert_eq!(
        passive.get("bat_r"),
        1.0,
        "the battery line takes the largest"
    );
}
