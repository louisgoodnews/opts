//! The `[smoke]` emitter of a vehicle `model.cfg`: position, direction and the optional variables.

use opts_omsi::formats::vehicles::model::ModelConfig;

fn smoke_of(values: &str) -> opts_omsi::formats::vehicles::model::ast::SmokeEmitter {
    let model = ModelConfig::parse(&format!("[smoke]\n{values}\n")).expect("parses");
    assert!(model.warnings.is_empty(), "{:?}", model.warnings);
    model.smokes.into_iter().next().expect("one emitter")
}

#[test]
fn reads_position_direction_and_variables() {
    let e = smoke_of("0\n-5.5\n1.2\n0\n1\n0\nmotor_on\n1\nexhaust_freq");
    assert_eq!(e.pos, glam::Vec3::new(0.0, -5.5, 1.2));
    assert_eq!(e.dir, glam::Vec3::new(0.0, 1.0, 0.0));
    assert_eq!(e.variable.as_deref(), Some("motor_on"));
    assert_eq!(e.freq_variable.as_deref(), Some("exhaust_freq"));
}

#[test]
fn optional_variables_may_be_absent() {
    let e = smoke_of("1\n2\n3\n0\n0\n1");
    assert_eq!(e.variable, None);
    assert_eq!(e.freq_variable, None);
}
