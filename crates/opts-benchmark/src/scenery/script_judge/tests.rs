//! Unit tests for `super` (`script_judge.rs`).

use super::*;
use crate::scenery::script_run::{Probe, State};

fn state(vars: &[(&str, f32)]) -> State {
    State {
        name: "s".into(),
        pixels: Vec::new(),
        probe: Probe {
            vars: vars.iter().map(|(k, v)| ((*k).to_string(), *v)).collect(),
            ..Probe::default()
        },
    }
}

#[test]
fn variables_are_looked_up_without_case_and_default_to_zero() {
    let s = state(&[("green", 1.0)]);
    assert_eq!(var(&s, "GREEN"), 1.0);
    assert_eq!(var(&s, "unknown"), 0.0);
}

#[test]
fn mesh_rules_follow_the_top_lod_and_the_variables() {
    let sco = opts_core::formats::scenery::SceneryObject::from_text(
        "[mesh]\na.x\n[LOD]\n0.2\n[mesh]\nb.x\n[visible]\nflag\n1\n[LOD]\n0\n[mesh]\nc.x\n",
    )
    .unwrap();
    let infos = mesh_infos(&sco);
    assert_eq!(
        infos.iter().map(|m| m.top).collect::<Vec<_>>(),
        [true, true, false]
    );
    assert_eq!(infos[1].visible, Some(("flag".to_string(), 1.0)));
}
