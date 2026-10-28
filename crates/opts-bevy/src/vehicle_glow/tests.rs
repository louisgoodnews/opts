//! Unit tests for `super` (`vehicle_glow.rs`).

use super::*;

fn e(i: u32) -> Entity {
    Entity::from_raw_u32(i).expect("valid index")
}

#[test]
fn the_pool_takes_the_nearest_and_keeps_them_under_small_moves() {
    let cands: Vec<(Entity, f32)> = (1..=10).map(|i| (e(i), i as f32 * 10.0)).collect();
    let first = choose_headlights(&cands, &[], 6, HEADLIGHT_HYSTERESIS);
    assert_eq!(first, (1..=6).map(e).collect::<Vec<_>>());
    // Vehicle 7 comes a little nearer than vehicle 6: no swap (within the hysteresis).
    let mut moved = cands.clone();
    moved[6].1 = 58.0;
    let second = choose_headlights(&moved, &first, 6, HEADLIGHT_HYSTERESIS);
    assert!(second.contains(&e(6)) && !second.contains(&e(7)));
    // Much nearer: it takes over.
    moved[6].1 = 5.0;
    let third = choose_headlights(&moved, &second, 6, HEADLIGHT_HYSTERESIS);
    assert!(third.contains(&e(7)) && third.len() == 6);
    // Fewer candidates than lights.
    assert_eq!(choose_headlights(&cands[..2], &[], 6, 1.25).len(), 2);
}

#[test]
fn brightness_approaches_with_the_time_constant() {
    let glow = VehicleGlow {
        owner: Entity::PLACEHOLDER,
        variable: "ai_light".into(),
        factor: 1.0,
        time_const_s: 0.2,
        color: [255.0; 3],
        headlight: true,
    };
    let after = glow.approach(0.0, 1.0, 0.2);
    assert!((after - (1.0 - (-1.0f32).exp())).abs() < 1e-5);
    let instant = VehicleGlow {
        time_const_s: 0.0,
        ..glow
    };
    assert_eq!(instant.approach(0.0, 1.0, 0.01), 1.0);
}

#[test]
fn glows_sit_under_the_vehicle_root_and_follow_its_variables() {
    let text = "[LOD]\n0.1\n\n[mesh]\nbody.o3d\n\n[light_enh_2]\n0\n2\n0.5\n0\n1\n0\n0\n0\n1\n0\n0\n255\n255\n255\n0.3\n40\n80\nAI_Light\n1\n0\n0\n0\n0\n\n[LOD]\n0.02\n\n[mesh]\nlow.o3d\n\n[light_enh_2]\n0\n2\n0.5\n0\n1\n0\n0\n0\n1\n0\n0\n255\n255\n255\n0.3\n40\n80\nAI_Light\n1\n0\n0\n0\n0\n";
    let model = ModelConfig::parse(text).expect("model");
    assert_eq!(model.lights.len(), 2);
    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    app.add_systems(Update, vehicle_glow_system);
    let root = app.world_mut().spawn(AiVariables::default()).id();
    let glows = {
        let mut commands = app.world_mut().commands();
        spawn_vehicle_glows(&mut commands, root, root, &model)
    };
    app.world_mut().flush();
    assert_eq!(glows.len(), 1, "only the detail LOD's light");
    assert_eq!(
        app.world().get::<ChildOf>(glows[0]).map(ChildOf::parent),
        Some(root)
    );
    assert!(app.world().get::<VehicleGlow>(glows[0]).unwrap().headlight);
    app.world_mut()
        .get_mut::<AiVariables>(root)
        .unwrap()
        .set("ai_light", 1.0);
    app.update();
    assert_eq!(
        app.world().get::<SceneryLight>(glows[0]).unwrap().level,
        1.0
    );
}
