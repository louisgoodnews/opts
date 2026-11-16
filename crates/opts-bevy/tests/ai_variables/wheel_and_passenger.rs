//! A wheel turns with its variable and a visible rule follows it.

use super::*;

#[test]
fn a_wheel_turns_with_its_variable_and_a_visible_rule_follows_it() {
    let mut app = app();
    let owner = app.world_mut().spawn(AiVariables::default()).id();
    let wheel = app
        .world_mut()
        .spawn((
            MeshAnimation {
                owner,
                steps: vec![AnimStep {
                    frame: Mat4::IDENTITY,
                    variable: "Wheel_Rotation_0_L".into(),
                    factor: 57.295_78,
                    translate: false,
                    maxspeed: None,
                    delay: None,
                }],
                visible: Vec::new(),
                shown: Vec::new(),
            },
            Transform::IDENTITY,
            Visibility::Inherited,
        ))
        .id();
    let passenger = app
        .world_mut()
        .spawn((
            MeshAnimation {
                owner,
                steps: Vec::new(),
                visible: vec![("ai_pax2".into(), 1.0)],
                shown: Vec::new(),
            },
            Transform::IDENTITY,
            Visibility::Inherited,
        ))
        .id();
    app.update();
    assert_eq!(
        *app.world().get::<Visibility>(passenger).unwrap(),
        Visibility::Hidden
    );
    assert!(
        app.world()
            .get::<Transform>(wheel)
            .unwrap()
            .rotation
            .angle_between(Quat::IDENTITY)
            < 1e-6
    );

    let mut vars = app.world_mut().get_mut::<AiVariables>(owner).unwrap();
    vars.set("wheel_rotation_0_l", 1.0);
    vars.set("ai_pax2", 1.0);
    app.update();
    let angle = app
        .world()
        .get::<Transform>(wheel)
        .unwrap()
        .rotation
        .angle_between(Quat::IDENTITY);
    assert!((angle - 1.0).abs() < 1e-4, "wheel turned {angle} rad");
    assert_eq!(
        *app.world().get::<Visibility>(passenger).unwrap(),
        Visibility::Inherited
    );
}
