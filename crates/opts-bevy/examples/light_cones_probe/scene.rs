//! The probe scene: the lantern, the scenario setup that places the lamps and the camera.

use super::*;

fn lantern() -> ScoLight {
    // `neonlight_M_whip_S.sco`: an arm over the road, pointing down and slightly inwards.
    ScoLight {
        position: Vec3::new(-2.965, 0.0, 7.17),
        direction: Vec3::new(-0.3817, 0.0, -1.5344).normalize(),
        color: [230.0, 230.0, 255.0],
        size_m: 2.0,
        variable: "NightlightA".into(),
        factor: 0.8,
        omni: false,
        cone_inner_deg: 120.0,
        cone_outer_deg: 200.0,
        time_const_s: 0.2,
        volumetric_cone: true,
        bitmap: String::new(),
        after_mesh: None,
        lod_threshold: None,
    }
}
pub(super) fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let ground = materials.add(StandardMaterial {
        base_color: Color::srgb(0.35, 0.35, 0.37),
        perceptual_roughness: 0.9,
        ..default()
    });
    commands.spawn((
        Mesh3d(meshes.add(Plane3d::default().mesh().size(400.0, 800.0))),
        MeshMaterial3d(ground.clone()),
        Transform::from_xyz(0.0, 0.0, -300.0),
    ));
    // Facades on both sides of the road: where the depth fade matters.
    for side in [-1.0f32, 1.0] {
        commands.spawn((
            Mesh3d(meshes.add(Cuboid::new(1.0, 12.0, 700.0))),
            MeshMaterial3d(ground.clone()),
            Transform::from_xyz(side * 9.0, 6.0, -300.0),
        ));
    }
    commands.spawn((
        Camera3d::default(),
        opts_bevy::MainCamera,
        Tonemapping::None,
        Transform::from_xyz(0.0, 1.7, 5.0).looking_to(Vec3::NEG_Z, Vec3::Y),
    ));
}

/// Sets the current scenario up: despawns the old lamps, applies the quality settings, spawns the
/// new lamps and places the camera.
pub(super) fn setup_scenario(
    commands: &mut Commands,
    probe: &mut Probe,
    lamps: &Query<Entity, With<Lamp>>,
    camera: &mut Query<&mut Transform, With<opts_bevy::MainCamera>>,
) {
    let idx = probe.idx;
    for e in lamps.iter() {
        commands.entity(e).despawn();
    }
    let (layout, fade, eye) = {
        let s = &probe.scenarios[idx];
        (s.layout, s.fade, s.eye)
    };
    let mut q = ladder_preset(0);
    q.cone_depth_fade = fade;
    q.headlight_cones = false;
    commands.insert_resource(LightQualityInput {
        mode: LightMode::Custom,
        custom: q,
    });
    let shape = lantern_cone_shape(&lantern()).expect("lantern cone");
    let (spacing, count) = match layout {
        Layout::None => (0.0, 0),
        Layout::Road => (20.0, 64),
        Layout::Dense => (2.5, 64),
    };
    for i in 0..count {
        let root = commands
            .spawn((
                Transform::from_xyz(4.0, 0.0, -10.0 - spacing * i as f32),
                Visibility::Inherited,
                Lamp,
            ))
            .id();
        spawn_light_cone(
            commands,
            root,
            LightCone {
                kind: ConeKind::Lantern,
                shape,
                level: ConeLevel::Fixed(0.8),
            },
        );
    }
    if let Ok(mut t) = camera.single_mut() {
        let apex = Vec3::new(4.0, 0.0, -10.0) + shape.apex;
        *t = match eye {
            Eye::Street => Transform::from_xyz(0.0, 1.7, 5.0).looking_to(Vec3::NEG_Z, Vec3::Y),
            // Just below the lamp head, inside the cone.
            Eye::InsideAlong => Transform::from_translation(apex + Vec3::new(-0.2, -1.0, 0.0))
                .looking_to(Vec3::NEG_Z, Vec3::Y),
            Eye::InsideDown => Transform::from_translation(apex + Vec3::new(-0.2, -1.0, 0.0))
                .looking_to(Vec3::new(0.0, -1.0, -0.3), Vec3::NEG_Z),
        };
    }
}
