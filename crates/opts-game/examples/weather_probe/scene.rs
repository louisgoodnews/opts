use super::*;

pub(super) fn solid(r: u8, g: u8, b: u8) -> CpuImage {
    CpuImage {
        width: 4,
        height: 4,
        rgba: [r, g, b, 255].repeat(16),
    }
}

/// The headless app with the ground, environment and weather systems (no window).
pub(super) fn build_app() -> App {
    let mut app = App::new();
    app.add_plugins(
        DefaultPlugins
            .set(WindowPlugin {
                primary_window: None,
                exit_condition: ExitCondition::DontExit,
                ..default()
            })
            .set(RenderPlugin {
                synchronous_pipeline_compilation: true,
                ..default()
            })
            .disable::<WinitPlugin>(),
    )
    .add_plugins((GroundMaterialPlugin, EnvironmentPlugin))
    .init_resource::<opts_bevy::reflections::Wetness>()
    .init_resource::<opts_bevy::weather::GroundFogView>()
    .init_resource::<clouds::CloudState>()
    .init_resource::<precip::PrecipState>()
    .add_systems(
        Update,
        (
            fog::measure_fog_view,
            fog::update_fog.after(fog::measure_fog_view),
            clouds::update_clouds,
            precip::update_precipitation,
            (surfaces::drive_wetness, surfaces::update_surfaces).chain(),
        )
            .after(EnvironmentSet),
    );
    if let Some(root) = std::env::var_os("OMSI_PATH") {
        if let Some(c) = WeatherCatalogue::read(std::path::Path::new(&root)) {
            app.insert_resource(c);
        }
    }
    app
}

/// Spawns the image target, the drive camera and the sun; returns the target.
pub(super) fn spawn_camera_and_sun(world: &mut World) -> Handle<Image> {
    let mut img = Image::new_target_texture(W, H, TextureFormat::Rgba8UnormSrgb, None);
    img.asset_usage = RenderAssetUsages::RENDER_WORLD;
    img.texture_descriptor.usage |= TextureUsages::COPY_SRC;
    let target = world.resource_mut::<Assets<Image>>().add(img);
    world.insert_resource(ClearColor(Color::srgb(0.55, 0.7, 0.9)));
    world.spawn((
        Camera3d::default(),
        opts_bevy::MainCamera,
        RenderTarget::from(target.clone()),
        Transform::from_xyz(0.0, 2.0, 0.0).looking_at(Vec3::new(0.0, 6.0, -60.0), Vec3::Y),
        InGameCamera,
    ));
    world.spawn((
        DirectionalLight {
            illuminance: 15_000.0,
            ..default()
        },
        Transform::from_xyz(-3.0, 8.0, 4.0).looking_at(Vec3::ZERO, Vec3::Y),
    ));
    target
}

/// Spawns the layered ground, a few boxes and their materials.
pub(super) fn spawn_ground(world: &mut World) {
    // Layered ground (one grey-green layer) so the ground shader, with its wet and snow code,
    // is the one that runs.
    let layers = {
        let l = solid(90, 110, 70);
        world
            .resource_mut::<Assets<Image>>()
            .add(layer_array(&[&l], 4))
    };
    let details = {
        let l = solid(255, 255, 255);
        world
            .resource_mut::<Assets<Image>>()
            .add(layer_array(&[&l], 4))
    };
    let masks = world
        .resource_mut::<Assets<Image>>()
        .add(mask_array(&[None], 4));
    let ground = world
        .resource_mut::<Assets<GroundMaterial>>()
        .add(GroundMaterial {
            base: StandardMaterial {
                perceptual_roughness: 0.9,
                ..default()
            },
            extension: ground_extension(
                layers,
                details,
                masks,
                &[GroundSlot {
                    layer: 0,
                    repeats: 40.0,
                    detail_repeats: 1.0,
                }],
            ),
        });
    let plane = world
        .resource_mut::<Assets<Mesh>>()
        .add(Rectangle::new(2000.0, 2000.0));
    world.spawn((
        Mesh3d(plane),
        MeshMaterial3d(ground),
        Transform::from_rotation(Quat::from_rotation_x(-std::f32::consts::FRAC_PI_2)),
    ));
    let cube = world
        .resource_mut::<Assets<Mesh>>()
        .add(Cuboid::new(6.0, 10.0, 6.0));
    let wall = world
        .resource_mut::<Assets<StandardMaterial>>()
        .add(Color::srgb(0.75, 0.55, 0.45));
    for (i, z) in [-20.0, -50.0, -100.0, -200.0, -400.0]
        .into_iter()
        .enumerate()
    {
        let x = if i % 2 == 0 { -8.0 } else { 8.0 };
        world.spawn((
            Mesh3d(cube.clone()),
            MeshMaterial3d(wall.clone()),
            Transform::from_xyz(x, 5.0, z),
        ));
    }
}

pub(super) fn step(apps: &mut SubApps) {
    apps.update();
    let device = apps.main.world().resource::<RenderDevice>();
    let _ = device.wgpu_device().poll(PollType::Wait {
        submission_index: None,
        timeout: None,
    });
}
