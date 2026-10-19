//! Applying a plan item's keys to the scene.

use super::cone_material::ConeMaterial;
use super::config::Cfg;
use super::procedural::{sky_cube, Rng};
use super::*;

#[allow(clippy::too_many_arguments, clippy::type_complexity)]
pub(super) fn apply(
    cfg: &Cfg,
    commands: &mut Commands,
    assets: &Assets3,
    cones: &mut Assets<ConeMaterial>,
    shadow_map: &mut DirectionalLightShadowMap,
    cam: Option<Entity>,
    images: &mut Assets<Image>,
    every: &mut MirrorEvery,
    stale_q: &Query<Entity, Or<(With<Cone>, With<Fog>, With<Mirror>)>>,
    sun: &mut Query<(Entity, &mut Transform, &mut Visibility), (With<Sun>, Without<Cam>)>,
    lamps: &mut Query<(&NightLamp, &mut Visibility), (Without<Sun>, Without<HeadSpot>)>,
    spots: &mut Query<
        (Entity, &HeadSpot, &mut SpotLight, &mut Visibility),
        (Without<Sun>, Without<NightLamp>),
    >,
    ambient: &mut GlobalAmbientLight,
) {
    let Some(cam) = cam else { return };
    let night = cfg.s("scene", "sun") == "night";
    shadow_map.size = cfg.u("shadow", 2048) as usize;
    commands.insert_resource(ClearColor(if night {
        Color::srgb(0.01, 0.015, 0.03)
    } else {
        Color::srgb(0.52, 0.70, 0.95)
    }));

    // Sun.
    let elev = cfg.f("elev", 10.0).to_radians();
    let toward = cfg.s("look", "toward") == "toward";
    let moon = cfg.u("moon", 0) == 1;
    let az = 35f32.to_radians();
    let sz = if toward { -az.cos() } else { az.cos() };
    let sun_dir = Vec3::new(az.sin() * elev.cos(), elev.sin(), sz * elev.cos()).normalize();
    for (e, mut t, mut vis) in sun {
        let casc = CascadeShadowConfigBuilder {
            num_cascades: cfg.u("casc", 4) as usize,
            maximum_distance: cfg.f("cmax", 150.0),
            ..default()
        }
        .build();
        commands.entity(e).insert(casc);
        if let Ok(mut l) = commands.get_entity(e) {
            l.insert(sun_light(cfg, moon));
        }
        if cfg.u("fog", 0) == 1 && (!night || moon) {
            commands.entity(e).insert(VolumetricLight);
        } else {
            commands.entity(e).remove::<VolumetricLight>();
        }
        *t = Transform::default().looking_to(-sun_dir, Vec3::Y);
        *vis = if night && !moon {
            Visibility::Hidden
        } else {
            Visibility::Inherited
        };
    }
    ambient.brightness = if night { 20.0 } else { 700.0 };
    let lamp_n = cfg.u("lamps", 48) as usize;
    for (l, mut v) in lamps {
        *v = if night && l.0 < lamp_n {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        };
    }
    let spot_shadows = cfg.u("spotsh", 0) as usize;
    let vspots = cfg.u("vspots", 0) as usize;
    for (e, h, mut s, mut v) in spots {
        *v = if night && h.0 < cfg.u("spots", 6).max(vspots as u32) as usize {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        };
        s.shadow_maps_enabled = h.0 < spot_shadows.max(vspots);
        if h.0 < vspots {
            commands.entity(e).insert(VolumetricLight);
        } else {
            commands.entity(e).remove::<VolumetricLight>();
        }
    }

    // Camera.
    let az_cam = if toward { 0.0 } else { std::f32::consts::PI };
    // "toward" looks down -Z (toward the sun); "away" turns around and looks down +Z, the camera
    // sits at the far end of the street so the same canyon fills the view.
    let pos = if toward {
        Vec3::new(0.0, 1.8, 6.0)
    } else {
        Vec3::new(0.0, 1.8, -140.0)
    };
    let _ = az_cam;
    let look_dir = if toward { Vec3::NEG_Z } else { Vec3::Z };
    commands
        .entity(cam)
        .insert(Transform::from_translation(pos).looking_to(look_dir, Vec3::Y));
    if cfg.u("msaa", 4) == 0 {
        commands.entity(cam).insert(Msaa::Off);
    } else if cfg.u("msaa", 4) == 2 {
        commands.entity(cam).insert(Msaa::Sample2);
    } else {
        commands.entity(cam).insert(Msaa::Sample4);
    }
    if cfg.u("hdr", 0) == 1 {
        commands.entity(cam).insert(Hdr);
    } else {
        commands.entity(cam).remove::<Hdr>();
    }
    if cfg.u("bloom", 0) == 1 {
        let mut b = Bloom::NATURAL;
        b.intensity = cfg.f("bloomi", 0.05);
        commands.entity(cam).insert(b);
    } else {
        commands.entity(cam).remove::<Bloom>();
    }
    if cfg.u("ibl", 0) == 1 {
        commands.entity(cam).insert(GeneratedEnvironmentMapLight {
            environment_map: images.add(sky_cube(64)),
            intensity: 500.0,
            ..default()
        });
    } else {
        commands
            .entity(cam)
            .remove::<GeneratedEnvironmentMapLight>();
    }
    // Mirror cameras (render to a small 8-bit image; every `mevery`-th frame).
    for e in stale_q {
        commands.entity(e).despawn();
    }
    every.0 = cfg.u("mevery", 1);
    let mw = cfg.u("msize", 256);
    for i in 0..cfg.u("mirrors", 0) as usize {
        let mut image = Image::new_fill(
            Extent3d {
                width: mw,
                height: mw / 2,
                depth_or_array_layers: 1,
            },
            TextureDimension::D2,
            &[0, 0, 0, 255],
            TextureFormat::Rgba8UnormSrgb,
            RenderAssetUsages::default(),
        );
        image.texture_descriptor.usage = TextureUsages::TEXTURE_BINDING
            | TextureUsages::COPY_SRC
            | TextureUsages::RENDER_ATTACHMENT;
        let handle = images.add(image);
        let mut m = commands.spawn((
            Camera3d::default(),
            Camera {
                order: -1 - i as isize,
                ..default()
            },
            RenderTarget::Image(handle.into()),
            Msaa::Off,
            Transform::from_translation(pos + Vec3::new(if i == 0 { 2.5 } else { -2.5 }, 0.3, 1.0))
                .looking_to(look_dir, Vec3::Y),
            Mirror,
        ));
        if cfg.u("mhdr", 0) == 1 {
            m.insert(Hdr);
        }
        if cfg.u("mibl", 0) == 1 {
            m.insert(GeneratedEnvironmentMapLight {
                environment_map: images.add(sky_cube(64)),
                intensity: 500.0,
                ..default()
            });
        }
    }
    let ballast = cfg.u("ballast", 0);
    if ballast > 0 {
        let mat = cones.add(ConeMaterial {
            params: Vec4::ZERO,
            params2: Vec4::new(ballast as f32, 0.0, 0.0, 0.0),
            march: false,
            ballast: true,
        });
        commands.spawn((
            Mesh3d(assets.quad.clone()),
            MeshMaterial3d(mat),
            Transform::from_translation(pos + look_dir * 1.0)
                .looking_to(look_dir, Vec3::Y)
                .with_scale(Vec3::new(1.7, 1.0, 1.0)),
            NotShadowCaster,
            NotShadowReceiver,
            RenderLayers::layer(1),
            bevy::camera::visibility::NoFrustumCulling,
            Cone,
        ));
    }
    let tech = cfg.s("tech", "none");
    let need_prepass = cfg.u("prepass", 0) == 1 || cfg.u("fade", 0) == 1;
    if need_prepass {
        commands.entity(cam).insert(DepthPrepass);
    } else {
        commands.entity(cam).remove::<DepthPrepass>();
    }
    if cfg.u("fog", 0) == 1 || vspots > 0 {
        commands.entity(cam).insert(VolumetricFog {
            step_count: cfg.u("steps", 64),
            jitter: cfg.f("jitter", 0.0),
            ambient_intensity: cfg.f("ambient", 0.0),
            ambient_color: Color::WHITE,
        });
    } else {
        commands.entity(cam).remove::<VolumetricFog>();
    }

    // Fog volume.
    if cfg.u("fog", 0) == 1 || vspots > 0 {
        let side = cfg.f("vol", 200.0);
        commands.spawn((
            FogVolume {
                density_factor: cfg.f("density", 0.002),
                scattering: 0.3,
                absorption: 0.1,
                scattering_asymmetry: cfg.f("g", 0.6),
                ..default()
            },
            Transform::from_translation(pos).with_scale(Vec3::splat(side)),
            Fog,
        ));
    }

    // Cones.
    let n = cfg.u("n", 0) as usize;
    if n > 0 && (tech == "mesh" || tech == "march") {
        let march = tech == "march";
        let fade = cfg.u("fade", 0) == 1;
        let mat_l = cones.add(ConeMaterial {
            params: Vec4::new(
                if march {
                    cfg.f("inten", 3.0)
                } else {
                    cfg.f("inten", 0.45)
                },
                cfg.f("edge", 2.0),
                if fade { 1.5 } else { 0.0 },
                cfg.f("axial", 1.5),
            ),
            params2: Vec4::new(cfg.u("msteps", 8) as f32, 0.35, 0.0, cfg.f("dbg", 0.0)),
            march,
            ballast: false,
        });
        if cfg.u("inside", 0) == 1 {
            // A big cone around the camera: the view starts inside its volume.
            let (len, half) = (40.0f32, 22f32.to_radians());
            commands.spawn((
                Mesh3d(assets.cone.clone()),
                MeshMaterial3d(mat_l.clone()),
                Transform::from_translation(pos - look_dir * 3.0 + Vec3::new(0.0, 0.3, 0.0))
                    .looking_to(look_dir, Vec3::Y)
                    .with_scale(Vec3::new(len * half.tan(), len * half.tan(), len)),
                MeshTag::new(255 | (240 << 8) | (200 << 16)),
                NotShadowCaster,
                NotShadowReceiver,
                Cone,
            ));
        }
        let mut rng = Rng(99);
        for i in 0..n {
            let lantern = i % 5 < 3;
            let side = if i % 2 == 0 { -1.0 } else { 1.0 };
            let zf = (i as f32 + 0.5) / n as f32;
            let z = -8.0 - zf * 120.0;
            let (apex, dir, len, half) = if lantern {
                (
                    Vec3::new(side * 5.5, 6.5, z),
                    Vec3::new(-side * 0.25, -1.0, 0.0).normalize(),
                    8.0,
                    30f32.to_radians(),
                )
            } else {
                let lane = if rng.next() < 0.5 { -2.0 } else { 2.0 };
                (
                    Vec3::new(lane + rng.range(-0.5, 0.5), 0.8, z),
                    if lane < 0.0 { Vec3::NEG_Z } else { Vec3::Z },
                    35.0,
                    14f32.to_radians(),
                )
            };
            let col = if lantern {
                [255u32, 215, 150]
            } else {
                [255, 245, 215]
            };
            let tag = col[0] | (col[1] << 8) | (col[2] << 16);
            commands.spawn((
                Mesh3d(assets.cone.clone()),
                MeshMaterial3d(mat_l.clone()),
                Transform::from_translation(apex)
                    .looking_to(dir, Vec3::Y)
                    .with_scale(Vec3::new(len * half.tan(), len * half.tan(), len)),
                MeshTag::new(tag),
                NotShadowCaster,
                NotShadowReceiver,
                Cone,
            ));
        }
    }
}

fn sun_light(cfg: &Cfg, moon: bool) -> DirectionalLight {
    DirectionalLight {
        shadow_maps_enabled: cfg.u("sunshadow", 1) == 1,
        illuminance: if moon {
            cfg.f("lux", 400.0)
        } else {
            cfg.f("lux", 20_000.0)
        },
        ..default()
    }
}

pub(super) fn mirror_toggle(
    bench: Res<Bench>,
    every: Res<MirrorEvery>,
    mut q: Query<&mut Camera, With<Mirror>>,
) {
    let on = every.0 <= 1 || bench.frame.is_multiple_of(every.0);
    for mut c in &mut q {
        if c.is_active != on {
            c.is_active = on;
        }
    }
}
