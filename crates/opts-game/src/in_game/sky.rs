//! Data-driven sky: dome texture, sun/moon discs and light colours from the installation's
//! `envir.cfg`, driven by the sun's elevation at the session's time of day.
//!
//! OMSI maps carry no geographic position, so the sun path assumes a fixed mid-latitude
//! (`LATITUDE_DEG`) and treats the clock time as local solar time.

use std::path::Path;

use bevy::asset::RenderAssetUsages;
use bevy::prelude::*;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};
use opts_bevy::ground::CpuImage;
use opts_core::formats::envir::EnvirConfig;

use crate::in_game::types::InGameEntity;
use crate::new_game_screen::types::NewGameSetup;

/// Assumed latitude of the simulated region (central Europe, where OMSI's maps lie).
const LATITUDE_DEG: f32 = 50.0;
/// Radius of the sky dome; stays inside the default 1000 m far plane.
const DOME_RADIUS: f32 = 900.0;
/// Seconds of game time between sky texture rebuilds.
const REFRESH_S: f32 = 0.5;
/// Peak illuminance of full white direct sunlight.
const SUN_ILLUMINANCE: f32 = 20_000.0;
/// Ambient brightness for full white ambient light.
const AMBIENT_BRIGHTNESS: f32 = 700.0;

impl Sky {
    /// Whether it is dark enough for vehicle headlights `elapsed_s` seconds into the session
    /// (sun lower than 3° above the horizon).
    #[must_use]
    pub fn lights_needed(&self, elapsed_s: f32) -> bool {
        sun_position(self.start_hour + elapsed_s / 3600.0, self.day_of_year).0 < 3.0
    }
}

/// Sky state for the running session.
#[derive(Resource)]
pub struct Sky {
    config: EnvirConfig,
    textures: [CpuImage; 3],
    image: Handle<Image>,
    start_hour: f32,
    day_of_year: f32,
    /// Multiplier for sun and ambient light under overcast presets.
    attenuation: f32,
    last_refresh: f32,
}

/// Marks the sky dome.
#[derive(Component)]
pub struct SkyDome;
/// Marks the sun light and disc.
#[derive(Component)]
pub struct SunLight;
/// Marks the sun disc quad.
#[derive(Component)]
pub struct SunDisc;
/// Marks the moon disc quad.
#[derive(Component)]
pub struct MoonDisc;

fn load_texture(root: &Path, rel: &str) -> Result<Image, String> {
    let path = rel.replace('\\', "/");
    let full = opts_bevy::install::OmsiInstall::new(root)
        .find_file(&path)
        .ok_or_else(|| format!("{rel} not found"))?;
    let bytes = std::fs::read(&full).map_err(|e| format!("{}: {e}", full.display()))?;
    opts_bevy::texture::load_bevy_image(&bytes).map_err(|e| format!("{rel}: {e}"))
}

fn day_of_year(month: u32, day: u32) -> f32 {
    const CUM: [u32; 12] = [0, 31, 59, 90, 120, 151, 181, 212, 243, 273, 304, 334];
    (CUM[(month.clamp(1, 12) - 1) as usize] + day) as f32
}

/// Sun elevation and azimuth (degrees; azimuth clockwise from north) for the local solar hour.
#[must_use]
pub fn sun_position(hour: f32, day_of_year: f32) -> (f32, f32) {
    let lat = LATITUDE_DEG.to_radians();
    let decl =
        (-23.44f32).to_radians() * (std::f32::consts::TAU * (day_of_year + 10.0) / 365.0).cos();
    let h = (15.0 * (hour - 12.0)).to_radians();
    let sin_el = lat.sin() * decl.sin() + lat.cos() * decl.cos() * h.cos();
    let az = h.sin().atan2(h.cos() * lat.sin() - decl.tan() * lat.cos()) + std::f32::consts::PI;
    (
        sin_el.clamp(-1.0, 1.0).asin().to_degrees(),
        az.to_degrees().rem_euclid(360.0),
    )
}

/// Direction towards a point at the given elevation/azimuth, in Bevy space (north = −Z).
fn direction(elevation_deg: f32, azimuth_deg: f32) -> Vec3 {
    let (el, az) = (elevation_deg.to_radians(), azimuth_deg.to_radians());
    Vec3::new(el.cos() * az.sin(), el.sin(), -el.cos() * az.cos())
}

/// Builds the dome: the UV-sphere's `u` is kept, `v` is rewritten to run 0 (zenith) → 1 (horizon).
fn dome_mesh() -> Mesh {
    let mut mesh = Sphere::new(DOME_RADIUS).mesh().uv(48, 24);
    let uvs: Option<Vec<[f32; 2]>> = {
        use bevy::mesh::VertexAttributeValues as V;
        match (
            mesh.attribute(Mesh::ATTRIBUTE_POSITION),
            mesh.attribute(Mesh::ATTRIBUTE_UV_0),
        ) {
            (Some(V::Float32x3(p)), Some(V::Float32x2(uv))) => Some(
                p.iter()
                    .zip(uv)
                    .map(|(p, uv)| {
                        let el = (p[1] / DOME_RADIUS).clamp(-1.0, 1.0).asin();
                        [
                            uv[0],
                            (1.0 - el / std::f32::consts::FRAC_PI_2).clamp(0.0, 1.0),
                        ]
                    })
                    .collect(),
            ),
            _ => None,
        }
    };
    if let Some(uvs) = uvs {
        mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, uvs);
    }
    mesh
}

fn blended_sky(sky: &Sky, weights: [f32; 3]) -> Vec<u8> {
    let (w, h) = (sky.textures[0].width, sky.textures[0].height);
    let mut out = vec![255u8; w * h * 4];
    for y in 0..h {
        for x in 0..w {
            for c in 0..3 {
                let mut acc = 0.0;
                for (t, wt) in sky.textures.iter().zip(weights) {
                    if wt > 0.0 {
                        let (tx, ty) = (x * t.width / w, y * t.height / h);
                        acc += f32::from(t.rgba[(ty * t.width + tx) * 4 + c]) * wt;
                    }
                }
                out[(y * w + x) * 4 + c] = acc.round().clamp(0.0, 255.0) as u8;
            }
        }
    }
    out
}

fn disc(
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    texture: Option<Image>,
    images: &mut Assets<Image>,
    size: f32,
) -> (Handle<Mesh>, Handle<StandardMaterial>) {
    let handle = texture.map(|t| images.add(t));
    (
        meshes.add(Rectangle::new(size, size)),
        materials.add(StandardMaterial {
            base_color_texture: handle,
            unlit: true,
            alpha_mode: AlphaMode::Add,
            cull_mode: None,
            fog_enabled: false,
            ..default()
        }),
    )
}

/// Loads `envir.cfg` and spawns the dome, sun light, sun and moon discs.
///
/// # Errors
/// Returns a message when the configuration or a sky texture cannot be read; the caller falls
/// back to the plain default light so a broken install still renders.
pub fn spawn_sky(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    images: &mut Assets<Image>,
    root: &Path,
    setup: &NewGameSetup,
) -> Result<(), String> {
    let path = opts_bevy::install::OmsiInstall::new(root)
        .find_file("envir.cfg")
        .ok_or("envir.cfg not found")?;
    let config = EnvirConfig::parse(&std::fs::read(&path).map_err(|e| e.to_string())?)?;
    let mut cpu = Vec::new();
    for rel in config.sky_textures.iter().take(3) {
        let img = load_texture(root, rel)?;
        cpu.push(
            CpuImage::from_image(&img).ok_or_else(|| format!("{rel}: unsupported pixel format"))?,
        );
    }
    let textures: [CpuImage; 3] = cpu.try_into().map_err(|_| "need three sky textures")?;
    let (w, h) = (textures[0].width as u32, textures[0].height as u32);
    let mut sky_image = Image::new_fill(
        Extent3d {
            width: w,
            height: h,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        &[0, 0, 0, 255],
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::default(),
    );
    sky_image.sampler =
        bevy::image::ImageSampler::Descriptor(bevy::image::ImageSamplerDescriptor {
            address_mode_u: bevy::image::ImageAddressMode::Repeat,
            address_mode_v: bevy::image::ImageAddressMode::ClampToEdge,
            mag_filter: bevy::image::ImageFilterMode::Linear,
            min_filter: bevy::image::ImageFilterMode::Linear,
            ..default()
        });
    let image = images.add(sky_image);

    let weather = setup.weather_preset.to_lowercase();
    let overcast = ["rain", "fog", "snow", "overcast"]
        .iter()
        .any(|k| weather.contains(k));
    let sky = Sky {
        config,
        textures,
        image: image.clone(),
        start_hour: setup.hour as f32 + setup.minute as f32 / 60.0,
        day_of_year: day_of_year(setup.month, setup.day),
        attenuation: if overcast { 0.55 } else { 1.0 },
        last_refresh: f32::NEG_INFINITY,
    };

    {
        let (el, _) = sun_position(sky.start_hour, sky.day_of_year);
        let data = blended_sky(&sky, sky.config.sky_weights(el));
        if let Some(mut img) = images.get_mut(&sky.image) {
            img.data = Some(data);
        }
    }
    let image = sky.image.clone();
    commands.spawn((
        (
            Mesh3d(meshes.add(dome_mesh())),
            MeshMaterial3d(materials.add(StandardMaterial {
                base_color_texture: Some(image),
                unlit: true,
                cull_mode: Some(bevy::render::render_resource::Face::Front),
                fog_enabled: false,
                ..default()
            })),
        ),
        bevy::light::NotShadowCaster,
        bevy::light::NotShadowReceiver,
        SkyDome,
        InGameEntity,
        Name::new("Sky_Dome"),
    ));
    commands.spawn((
        (DirectionalLight {
            shadow_maps_enabled: true,
            ..default()
        }),
        SunLight,
        InGameEntity,
        Name::new("Sun_Light"),
    ));
    let sun_tex = load_texture(root, "Texture/sonne.bmp").ok();
    let moon_tex = load_texture(root, "Texture/mond.bmp").ok();
    for (is_sun, tex, size) in [(true, sun_tex, 160.0), (false, moon_tex, 120.0)] {
        let (mesh, material) = disc(meshes, materials, tex, images, size);
        let mut e = commands.spawn((
            (Mesh3d(mesh), MeshMaterial3d(material)),
            bevy::light::NotShadowCaster,
            bevy::light::NotShadowReceiver,
            InGameEntity,
        ));
        if is_sun {
            e.insert((SunDisc, Name::new("Sun_Disc")));
        } else {
            e.insert((MoonDisc, Name::new("Moon_Disc")));
        }
    }
    commands.insert_resource(sky);
    Ok(())
}

/// Moves the dome with the camera and updates sky colours, sun/moon placement and lighting.
#[allow(clippy::type_complexity, clippy::too_many_arguments)]
pub fn update_sky(
    time: Res<Time<Virtual>>,
    sky: Option<ResMut<Sky>>,
    mut images: ResMut<Assets<Image>>,
    mut clear: Option<ResMut<ClearColor>>,
    mut ambient: Option<ResMut<GlobalAmbientLight>>,
    look: Option<Res<crate::look::ActiveLook>>,
    cameras: Query<&GlobalTransform, (With<Camera3d>, Without<SkyDome>)>,
    mut dome: Query<
        &mut Transform,
        (
            With<SkyDome>,
            Without<SunLight>,
            Without<SunDisc>,
            Without<MoonDisc>,
        ),
    >,
    mut sun: Query<(&mut Transform, &mut DirectionalLight), (With<SunLight>, Without<SkyDome>)>,
    mut sun_disc: Query<
        &mut Transform,
        (
            With<SunDisc>,
            Without<SkyDome>,
            Without<SunLight>,
            Without<MoonDisc>,
        ),
    >,
    mut moon_disc: Query<
        &mut Transform,
        (
            With<MoonDisc>,
            Without<SkyDome>,
            Without<SunLight>,
            Without<SunDisc>,
        ),
    >,
) {
    let Some(mut sky) = sky else { return };
    let Some(cam) = cameras.iter().next().map(GlobalTransform::translation) else {
        return;
    };
    let now = time.elapsed_secs();
    let hour = sky.start_hour + now / 3600.0;
    let (elevation, azimuth) = sun_position(hour, sky.day_of_year);
    let sun_dir = direction(elevation, azimuth);

    for mut t in &mut dome {
        // Rotate so texture u = 0 faces the sun (the dusk texture's glow is authored around it).
        t.translation = cam;
        t.rotation = Quat::from_rotation_y(-azimuth.to_radians());
    }
    // A new look changes the ambient scale: refresh at once instead of at the next interval.
    let look_changed = look.as_ref().is_some_and(|l| l.is_changed());
    let ambient_scale = look.as_ref().map_or(1.0, |l| l.0.params().ambient_scale);
    if look_changed || now - sky.last_refresh >= REFRESH_S {
        sky.last_refresh = now;
        let weights = sky.config.sky_weights(elevation);
        let data = blended_sky(&sky, weights);
        if let Some(mut img) = images.get_mut(&sky.image) {
            img.data = Some(data);
        }
        let l = |c: [f32; 3]| (c[0] + c[1] + c[2]) / 3.0;
        let a = sky.config.light_at(&sky.config.light_a, elevation);
        let b = sky.config.light_at(&sky.config.light_b, elevation);
        let c = sky.config.light_at(&sky.config.light_c, elevation);
        let att = sky.attenuation;
        for (_, mut light) in &mut sun {
            light.color = Color::srgb(a[0].max(0.001), a[1].max(0.001), a[2].max(0.001));
            light.illuminance = SUN_ILLUMINANCE * l(a) * att;
        }
        let amb = [c[0] + b[0] * 0.5, c[1] + b[1] * 0.5, c[2] + b[2] * 0.5];
        let peak = amb.iter().copied().fold(0.001_f32, f32::max);
        if let Some(ambient) = ambient.as_mut() {
            ambient.color = Color::srgb(amb[0] / peak, amb[1] / peak, amb[2] / peak);
            ambient.brightness = AMBIENT_BRIGHTNESS * ambient_scale * peak.min(1.0) * att.max(0.8);
        }
        // Colour of the sky at the horizon fills anything the dome does not cover.
        let h = sky.textures[0].height - 1;
        let w = weights;
        let mut horizon = [0.0; 3];
        for (ch, out) in horizon.iter_mut().enumerate() {
            for (t, wt) in sky.textures.iter().zip(w) {
                *out += f32::from(t.rgba[(h * t.width) * 4 + ch]) * wt / 255.0;
            }
        }
        if let Some(clear) = clear.as_mut() {
            clear.0 = Color::srgb(horizon[0], horizon[1], horizon[2]);
        }
    }
    for (mut t, _) in &mut sun {
        *t = Transform::from_translation(cam).looking_to(-sun_dir, Vec3::Y);
    }
    let place = |dir: Vec3| {
        let mut t = Transform::from_translation(cam + dir * (DOME_RADIUS - 10.0));
        t.look_at(cam, Vec3::Y);
        if dir.y <= -0.05 {
            t.scale = Vec3::ZERO;
        }
        t
    };
    for mut t in &mut sun_disc {
        *t = place(sun_dir);
    }
    for mut t in &mut moon_disc {
        *t = place(-sun_dir);
    }
}
