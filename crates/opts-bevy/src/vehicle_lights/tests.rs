//! Unit tests for `super` (`vehicle_lights.rs`).

use super::*;

fn light(switch: LightSwitch) -> ScriptLight {
    ScriptLight {
        owner: Entity::PLACEHOLDER,
        switch,
        base_intensity: 100.0,
    }
}

#[test]
fn interior_light_follows_its_variable_clamped() {
    let l = light(LightSwitch::Variable("lights_cab".into()));
    assert_eq!(l.factor(|_| 0.0), 0.0);
    assert_eq!(l.factor(|_| 0.5), 0.5);
    assert_eq!(l.factor(|_| 7.0), MAX_BRIGHTNESS);
    assert_eq!(l.factor(|_| -1.0), 0.0);
}

#[test]
fn only_the_selected_spotlight_shines() {
    let high = light(LightSwitch::Spot(0));
    let dipped = light(LightSwitch::Spot(1));
    let select = |v: f32| move |n: &str| if n == "spot_select" { v } else { 0.0 };
    assert_eq!(
        (high.factor(select(0.0)), dipped.factor(select(0.0))),
        (1.0, 0.0)
    );
    assert_eq!(
        (high.factor(select(1.0)), dipped.factor(select(1.0))),
        (0.0, 1.0)
    );
    assert_eq!(
        (high.factor(select(-1.0)), dipped.factor(select(-1.0))),
        (0.0, 0.0)
    );
}
