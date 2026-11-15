//! Unit tests of the consist: fixtures here, the runs and the flags in their own files.

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

mod flags;
mod runs;
