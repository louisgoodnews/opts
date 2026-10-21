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
        },
        Unit {
            engine: Some(&mut b),
            cables: &cables,
        },
        Unit {
            engine: Some(&mut c),
            cables: &cables,
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
        },
        Unit {
            engine: Some(&mut b),
            cables: &cables,
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
        },
        Unit {
            engine: None,
            cables: &cables,
        },
        Unit {
            engine: Some(&mut c),
            cables: &cables,
        },
    ]);
    assert_eq!(c.get("bat_r"), 1.0);
    let mut d = car();
    bridge(&mut [
        Unit {
            engine: Some(&mut a),
            cables: &cables,
        },
        Unit {
            engine: None,
            cables: &other,
        },
        Unit {
            engine: Some(&mut d),
            cables: &cables,
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
        },
        Unit {
            engine: Some(&mut b),
            cables: &cables,
        },
    ]);
    assert_eq!(a.get("bat_r"), -0.8);
}
