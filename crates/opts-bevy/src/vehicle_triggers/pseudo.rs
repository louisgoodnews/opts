//! The pseudo triggers of keys that need more than one script trigger (`@door_side_toggle`,
//! `@cabin_light_toggle`).

/// Switches a tram's door side between right and left with the scripts' own toggles: `kw_s_1`
/// flips the right side, `kw_s_2` the left one (`Tuervorwahl_rechts_*`, `Tuervorwahl_links_*`).
/// Nothing selected: the right; one selected: the other; both: the right only.
pub(super) fn toggle_door_side(engine: &mut opts_sim::script::ScriptEngine) -> bool {
    let (mut right, mut left) = (false, false);
    for (name, value) in engine.variables() {
        if value > 0.5 {
            right |= name.starts_with("tuervorwahl_rechts");
            left |= name.starts_with("tuervorwahl_links");
        }
    }
    let triggers: &[&str] = match (right, left) {
        (false, false) => &["kw_s_1"],
        (true, false) => &["kw_s_1", "kw_s_2"],
        (false, true) => &["kw_s_2", "kw_s_1"],
        (true, true) => &["kw_s_2"],
    };
    triggers
        .iter()
        .fold(false, |any, t| engine.trigger(t) | any)
}

/// Switches the cab light on or off. A bus's switch (`cp_fahrerlicht_sw`) has three positions and
/// the script's toggle steps through them (off, auto, on, off): the key steps until the switch
/// is *on* (2), or off when it is on. Vehicles without that variable get the plain toggle.
pub(super) fn toggle_cabin_light(engine: &mut opts_sim::script::ScriptEngine) -> bool {
    const SWITCH: &str = "cp_fahrerlicht_sw";
    let has_switch = engine.variables().iter().any(|(n, _)| n == SWITCH);
    if !has_switch {
        return engine.trigger("cp_fahrerlicht_toggle");
    }
    let target = if engine.get(SWITCH) > 1.5 { 0.0 } else { 2.0 };
    let mut handled = false;
    for _ in 0..3 {
        handled |= engine.trigger("cp_fahrerlicht_toggle");
        if (engine.get(SWITCH) - target).abs() < 0.5 {
            break;
        }
    }
    handled
}
