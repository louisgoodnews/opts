//! Mapping of the AI lamp and indicator variables (no game run needed).

use super::*;

/// The lamp variables `SD77` reads that the AI lamp mapping feeds.
const MODEL_LAMP_VARS: [&str; 10] = [
    "lights_abbl_bulb_1",
    "lights_abbl_bulb_2",
    "lights_fern",
    "lights_rueck",
    "lights_stand",
    "lights_blinkgeber",
    "lights_blinker_l_bulb_1",
    "lights_blinker_l_bulb_4",
    "lights_blinker_r_bulb_1",
    "lights_blinker_r_bulb_4",
];

fn lamps(dark: bool) -> opts_bevy::AiVariables {
    let mut vars = opts_bevy::AiVariables::default();
    set_lamp_variables(&mut vars, dark);
    vars
}

fn indicators(blinker: opts_traffic::Blinker, flash: bool) -> opts_bevy::AiVariables {
    let mut vars = opts_bevy::AiVariables::default();
    set_blinker_variables(&mut vars, blinker, flash);
    vars
}

#[test]
fn headlights_and_tail_lamps_follow_the_dark_state() {
    let night = lamps(true);
    for name in [
        "lights_abbl_bulb_1",
        "lights_abbl_bulb_2",
        "lights_rueck",
        "lights_stand",
    ] {
        assert_eq!(night.get(name), 1.0, "{name} at night");
    }
    assert_eq!(night.get("lights_fern"), 0.0, "no high beam for AI");
    let day = lamps(false);
    for name in MODEL_LAMP_VARS {
        assert_eq!(day.get(name), 0.0, "{name} by day");
    }
}

#[test]
fn a_lit_indicator_flashes_only_its_commanded_side() {
    let left_on = indicators(opts_traffic::Blinker::Left, true);
    assert_eq!(left_on.get("lights_blinker_l_bulb_1"), 1.0);
    assert_eq!(left_on.get("lights_blinker_l_bulb_4"), 1.0);
    assert_eq!(left_on.get("lights_blinker_r_bulb_1"), 0.0);
    assert_eq!(left_on.get("lights_blinkgeber"), 1.0);
    assert_eq!(left_on.get("ai_blinker_l"), 1.0);
    assert_eq!(left_on.get("ai_blinker_r"), 0.0);
    let left_off_phase = indicators(opts_traffic::Blinker::Left, false);
    assert_eq!(left_off_phase.get("lights_blinker_l_bulb_1"), 0.0);
    assert_eq!(left_off_phase.get("lights_blinkgeber"), 0.0);
    assert_eq!(left_off_phase.get("ai_blinker_l"), 1.0, "command stays on");
}

#[test]
fn no_indicator_command_keeps_every_flasher_dark() {
    let off = indicators(opts_traffic::Blinker::Off, true);
    for n in 1..=4 {
        assert_eq!(off.get(&format!("lights_blinker_l_bulb_{n}")), 0.0);
        assert_eq!(off.get(&format!("lights_blinker_r_bulb_{n}")), 0.0);
    }
    assert_eq!(off.get("ai"), 1.0);
}
