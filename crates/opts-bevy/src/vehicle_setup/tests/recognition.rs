//! Recognition of the start recipes from the scripts' own names, with the step counts read from
//! the recipe table (hand-computed: each stand-in script carries one marker of a family).

use super::*;

#[test]
fn the_stepped_citybus_key_is_recognised_by_its_counter() {
    // Electrics, drive position, starter held: three steps, the last one until the engine runs.
    let e = engine("{init}\n    0 (S.L.cp_keyboard_e_m)\n{end}\n");
    let r = recipe_for(&e);
    assert_eq!(r.name, "stepped ignition key");
    assert_eq!(r.steps.len(), 3);
    assert_eq!(r.steps[0].from, ELECTRICS);
    assert_eq!(r.steps[1].from, IGNITION);
    assert!(r.steps[2].until_engine);
}

#[test]
fn the_luzern_key_is_recognised_by_its_key_mode() {
    let e = engine("{init}\n    0 (S.L.cp_keymode)\n{end}\n");
    let r = recipe_for(&e);
    assert_eq!(r.name, "three-step key");
    assert_eq!(r.steps.len(), 3);
    assert_eq!(r.steps[0].from, ELECTRICS);
    assert!(r.steps[2].until_engine);
}

#[test]
fn the_four_step_key_needs_all_three_markers() {
    let all = "{init}\n    0 (S.L.key_mode)\n    0 (S.L.actia_dash_on)\n{end}\n\
               {trigger:actia_move_drag}\n    1 (S.L.actia_seen)\n{end}\n";
    let e = engine(all);
    let r = recipe_for(&e);
    assert_eq!(r.name, "four-step key");
    assert_eq!(r.steps.len(), 4);
    assert_eq!(r.steps[2].from, IGNITION);
    assert!(r.steps[3].until_engine);

    // Without the drag trigger the two shared markers fall through to the generic recipe, which
    // takes three steps.
    let partial = engine("{init}\n    0 (S.L.key_mode)\n    0 (S.L.actia_dash_on)\n{end}\n");
    let fallback = recipe_for(&partial);
    assert_eq!(fallback.name, "battery switch and start key");
    assert_eq!(fallback.steps.len(), 3);
}

#[test]
fn the_polygon_master_switch_takes_four_presses_and_a_second_starter_try() {
    let e = engine("{trigger:enginestartswitch}\n    1 (S.L.startmode)\n{end}\n");
    let r = recipe_for(&e);
    assert_eq!(r.name, "three-step key (master switch)");
    assert_eq!(r.steps.len(), 4);
    assert_eq!(r.steps[1].from, IGNITION);
    assert!(r.steps[3].only_if_off, "a second try only while the engine is off");
}
