use super::*;

fn engine(names: &[&str]) -> ScriptEngine {
    let (mut e, _) = ScriptEngine::from_sources(&[]);
    let names: Vec<String> = names.iter().map(|n| (*n).to_owned()).collect();
    e.declare_variables(&names, false);
    e
}

fn cable(front: bool, index: u32, receive: &str, send: &str, coupling: &str) -> Cable {
    Cable {
        front,
        kind: 'C',
        index,
        receive: receive.into(),
        send: send.into(),
        coupling: coupling.into(),
    }
}

fn car_cables() -> Vec<Cable> {
    vec![
        cable(true, 2, "bat_r", "bat_w", "line_v"),
        cable(false, 2, "bat_r", "bat_w", "line_h"),
    ]
}

fn car() -> ScriptEngine {
    let mut e = engine(&["bat_r", "bat_w", "line_v", "line_h"]);
    e.set("line_v", 1.0);
    e.set("line_h", 1.0);
    e
}

#[test]
fn a_value_one_car_sends_reaches_every_car_of_the_run_and_the_sender() {
    let cables = car_cables();
    let (mut a, mut b, mut c) = (car(), car(), car());
    a.set("bat_w", 1.0);
    let n = bridge(&mut [
        Unit {
            engine: Some(&mut a),
            cables: &cables,
            vehicle: 0,
        },
        Unit {
            engine: Some(&mut b),
            cables: &cables,
            vehicle: 0,
        },
        Unit {
            engine: Some(&mut c),
            cables: &cables,
            vehicle: 0,
        },
    ]);
    assert_eq!(n, 3);
    assert_eq!(
        (a.get("bat_r"), b.get("bat_r"), c.get("bat_r")),
        (1.0, 1.0, 1.0)
    );
}

#[test]
fn a_lone_car_gets_its_own_value_back() {
    let cables = car_cables();
    let mut a = car();
    a.set("bat_w", 1.0);
    bridge(&mut [Unit {
        engine: Some(&mut a),
        cables: &cables,
        vehicle: 0,
    }]);
    assert_eq!(a.get("bat_r"), 1.0);
}

#[test]
fn a_broken_coupling_variable_cuts_the_line() {
    let cables = car_cables();
    let (mut a, mut b) = (car(), car());
    a.set("bat_w", 1.0);
    a.set("line_h", 0.0);
    bridge(&mut [
        Unit {
            engine: Some(&mut a),
            cables: &cables,
            vehicle: 0,
        },
        Unit {
            engine: Some(&mut b),
            cables: &cables,
            vehicle: 0,
        },
    ]);
    assert_eq!((a.get("bat_r"), b.get("bat_r")), (1.0, 0.0));
}

#[test]
fn a_unit_without_the_cable_ends_the_run_and_one_without_scripts_passes_it_on() {
    let cables = car_cables();
    let other = [cable(true, 9, "x_r", "x_w", "line_v")];
    let (mut a, mut c) = (car(), car());
    a.set("bat_w", 1.0);
    bridge(&mut [
        Unit {
            engine: Some(&mut a),
            cables: &cables,
            vehicle: 0,
        },
        Unit {
            engine: None,
            cables: &cables,
            vehicle: 0,
        },
        Unit {
            engine: Some(&mut c),
            cables: &cables,
            vehicle: 0,
        },
    ]);
    assert_eq!(c.get("bat_r"), 1.0);
    let mut d = car();
    bridge(&mut [
        Unit {
            engine: Some(&mut a),
            cables: &cables,
            vehicle: 0,
        },
        Unit {
            engine: None,
            cables: &other,
            vehicle: 0,
        },
        Unit {
            engine: Some(&mut d),
            cables: &cables,
            vehicle: 0,
        },
    ]);
    assert_eq!(d.get("bat_r"), 0.0);
}

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
            vehicle: 0,
        },
        Unit {
            engine: Some(&mut b),
            cables: &cables,
            vehicle: 0,
        },
    ]);
    assert_eq!(a.get("bat_r"), -0.8);
}

/// The inactive flags of a cab stay within their vehicle: the passive trams of a traction write
/// their heads' "inactive" without the driven tram reading it, while other lines (battery) still
/// run through the whole formation.
#[test]
fn a_cabs_inactive_flag_does_not_cross_into_another_vehicle_but_other_lines_do() {
    let cables: Vec<Cable> = ["inaktiv_r", "bat_r"]
        .iter()
        .enumerate()
        .flat_map(|(i, receive)| {
            let send = if i == 0 { "inaktiv_w" } else { "bat_w" };
            [true, false].map(|front| Cable {
                front,
                kind: 'C',
                index: i as u32,
                receive: (*receive).to_owned(),
                send: send.to_owned(),
                coupling: if front { "line_v" } else { "line_h" }.to_owned(),
            })
        })
        .collect();
    let wired = || {
        let mut e = engine(&[
            "inaktiv_r",
            "inaktiv_w",
            "bat_r",
            "bat_w",
            "line_v",
            "line_h",
        ]);
        e.set("line_v", 1.0);
        e.set("line_h", 1.0);
        e
    };
    let (mut driven, mut passive) = (wired(), wired());
    driven.set("bat_w", 1.0);
    passive.set("inaktiv_w", 1.0);
    bridge(&mut [
        Unit {
            engine: Some(&mut driven),
            cables: &cables,
            vehicle: 0,
        },
        Unit {
            engine: Some(&mut passive),
            cables: &cables,
            vehicle: 1,
        },
    ]);
    assert_eq!(driven.get("inaktiv_r"), 0.0, "the other vehicle's flag");
    assert_eq!(passive.get("inaktiv_r"), 1.0, "its own flag");
    assert_eq!(passive.get("bat_r"), 1.0, "the battery line crosses");
}
