//! Data-driven sky: dome texture, sun/moon discs and light colours from the installation's
//! `envir.cfg`, driven by the sun of the [`SessionEnvironment`] (date, time, weather).
//!
//! OMSI maps carry no geographic position, so the sun path assumes a fixed mid-latitude
//! (`opts_bevy::environment::LATITUDE_DEG`) and treats the clock time as local solar time.

use std::path::Path;

use bevy::asset::RenderAssetUsages;
use bevy::prelude::*;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};
use opts_bevy::ground::CpuImage;
use opts_bevy::SessionEnvironment;
use opts_core::formats::envir::EnvirConfig;

use crate::in_game::types::InGameEntity;
/// Radius of the sky dome; stays inside the default 1000 m far plane.
pub(crate) const DOME_RADIUS: f32 = 900.0;
/// Clock seconds between sky texture rebuilds (a jump of the environment rebuilds at once).
const REFRESH_S: f64 = 0.5;
/// Peak illuminance of full white direct sunlight.
const SUN_ILLUMINANCE: f32 = 20_000.0;
/// Ambient brightness for full white ambient light.
const AMBIENT_BRIGHTNESS: f32 = 700.0;

/// Sky state for the running session.
#[derive(Resource)]
pub struct Sky {
    config: EnvirConfig,
    textures: [CpuImage; 3],
    image: Handle<Image>,
    /// Clock seconds of the last rebuild.
    last_refresh: f64,
    /// Environment revision of the last rebuild (`None` before the first).
    last_revision: Option<u64>,
    /// Whether the last rebuild wrote the clear colour for an HDR camera.
    last_hdr: Option<bool>,
    /// The ground-fog sky clearing of the last rebuild, in 1/32 steps (the sky is rebuilt when
    /// the camera's height moves it by a step).
    last_fog_clear: u8,
}

/// The sky's colour at the horizon (linear RGB `0..=1`), refreshed with the sky texture; the fog
/// takes its colour from it (`weather::fog`).
#[derive(Resource, Debug, Clone, Copy, PartialEq)]
pub struct SkyHorizon(pub [f32; 3]);

/// Marks the sky dome.
#[derive(Component)]
pub struct SkyDome;
/// Marks the sun light and disc.
#[derive(Component)]
pub struct SunLight;
/// Marks the sun disc quad.
#[derive(Component)]
pub struct SunDisc;
/// Marks the moon disc quad; `in_game::moon` gives it its mesh and material.
#[derive(Component)]
pub struct MoonDisc;
/// The moon photograph (`Texture/mond.bmp`) waiting for the disc's material.
#[derive(Component)]
pub struct MoonDiscTexture(pub Option<Handle<Image>>);

fn load_texture(root: &Path, rel: &str) -> Result<Image, String> {
    let path = rel.replace('\\', "/");
    let full = opts_bevy::install::OmsiInstall::new(root)
        .find_file(&path)
        .ok_or_else(|| format!("{rel} not found"))?;
    let bytes = std::fs::read(&full).map_err(|e| format!("{}: {e}", full.display()))?;
    opts_bevy::texture::load_bevy_image(&bytes).map_err(|e| format!("{rel}: {e}"))
}

/// Sun elevation and azimuth (degrees; azimuth clockwise from north) for the local solar hour:
/// [`opts_bevy::environment::sun_position`] at the sky's latitude.
#[must_use]
pub fn sun_position(hour: f32, day_of_year: f32) -> (f32, f32) {
    opts_bevy::environment::sun_position(hour, day_of_year, opts_bevy::environment::LATITUDE_DEG)
}

/// Direction towards a point at the given elevation/azimuth, in Bevy space (north = −Z).
pub(crate) fn direction(elevation_deg: f32, azimuth_deg: f32) -> Vec3 {
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

/// The sky texture for the blend weights of the sun's height. `veil` gives, per row (`0` zenith …
/// `1` horizon), how far the row is pulled towards `veil_color` (the fog colour, sRGB `0..=1`);
/// a veil of 0 leaves the texture as it is.
fn blended_sky(
    sky: &Sky,
    weights: [f32; 3],
    veil: impl Fn(f32) -> f32,
    veil_color: [f32; 3],
) -> Vec<u8> {
    let (w, h) = (sky.textures[0].width, sky.textures[0].height);
    let mut out = vec![255u8; w * h * 4];
    for y in 0..h {
        let v = veil(y as f32 / (h - 1).max(1) as f32).clamp(0.0, 1.0);
        for x in 0..w {
            for c in 0..3 {
                let mut acc = 0.0;
                for (t, wt) in sky.textures.iter().zip(weights) {
                    if wt > 0.0 {
                        let (tx, ty) = (x * t.width / w, y * t.height / h);
                        acc += f32::from(t.rgba[(ty * t.width + tx) * 4 + c]) * wt;
                    }
                }
                if v > 0.0 {
                    acc += (veil_color[c] * 255.0 - acc) * v;
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

    // The texture is filled by `update_sky` on its first run.
    let sky = Sky {
        config,
        textures,
        image: image.clone(),
        last_refresh: f64::NEG_INFINITY,
        last_revision: None,
        last_hdr: None,
        last_fog_clear: 0,
    };

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
        opts_bevy::sun_rays::LightRole::Sun,
        InGameEntity,
        Name::new("Sun_Light"),
    ));
    let sun_tex = load_texture(root, "Texture/sonne.bmp").ok();
    let moon_tex = load_texture(root, "Texture/mond.bmp").ok();
    let (mesh, material) = disc(meshes, materials, sun_tex, images, 160.0);
    commands.spawn((
        (Mesh3d(mesh), MeshMaterial3d(material)),
        bevy::light::NotShadowCaster,
        bevy::light::NotShadowReceiver,
        InGameEntity,
        SunDisc,
        Name::new("Sun_Disc"),
    ));
    // The moon disc gets its phase material from `in_game::moon`.
    commands.spawn((
        MoonDisc,
        MoonDiscTexture(moon_tex.map(|t| images.add(t))),
        bevy::light::NotShadowCaster,
        bevy::light::NotShadowReceiver,
        InGameEntity,
        Name::new("Moon_Disc"),
    ));
    commands.insert_resource(sky);
    Ok(())
}

/// Moves the sky dome to the [`MainCamera`](opts_bevy::MainCamera) and turns it so that texture
/// `u = 0` faces the sun (the dusk texture's glow is authored around it).
pub fn place_sky_dome(
    env: Res<SessionEnvironment>,
    cameras: Query<&GlobalTransform, (With<opts_bevy::MainCamera>, Without<SkyDome>)>,
    mut dome: Query<&mut Transform, With<SkyDome>>,
) {
    let Some(cam) = cameras.iter().next().map(GlobalTransform::translation) else {
        return;
    };
    for mut t in &mut dome {
        t.translation = cam;
        t.rotation = Quat::from_rotation_y(-env.sun_az_deg().to_radians());
    }
}

/// Updates sky colours, sun/moon placement and lighting.
#[allow(clippy::type_complexity, clippy::too_many_arguments)]
pub fn update_sky(
    env: Res<SessionEnvironment>,
    effects: Option<Res<opts_bevy::WeatherEffects>>,
    fog_view: Option<Res<opts_bevy::weather::GroundFogView>>,
    mut commands: Commands,
    sky: Option<ResMut<Sky>>,
    mut images: ResMut<Assets<Image>>,
    mut clear: Option<ResMut<ClearColor>>,
    mut ambient: Option<ResMut<GlobalAmbientLight>>,
    look: Option<Res<crate::look::ActiveLook>>,
    settings: Option<Res<crate::settings::GameSettings>>,
    cameras: Query<&GlobalTransform, (With<opts_bevy::MainCamera>, Without<SkyDome>)>,
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
) {
    let Some(mut sky) = sky else { return };
    let Some(cam) = cameras.iter().next().map(GlobalTransform::translation) else {
        return;
    };
    // The session environment (not the app's virtual time, which keeps running between
    // sessions), so a restored save and a changed time show their sky at once.
    let now = env.seconds_of_day();
    let (elevation, azimuth) = (env.sun_alt_deg(), env.sun_az_deg());
    let sun_dir = direction(elevation, azimuth);

    // A new look changes the ambient scale: refresh at once instead of at the next interval.
    let look_changed = look.as_ref().is_some_and(|l| l.is_changed());
    let ambient_scale = look.as_ref().map_or(1.0, |l| {
        crate::look::effective_ambient_scale(l.0.params().ambient_scale, elevation)
    });
    // Switching `graphics.hdr` changes how the clear colour must be written.
    let hdr = settings
        .as_ref()
        .is_some_and(|s| crate::settings_registry::live::hdr_enabled(s));
    let hdr_changed = sky.last_hdr != Some(hdr);
    let jumped = sky.last_revision != Some(env.revision());
    let effects_changed = effects.as_ref().is_some_and(|e| e.is_changed());
    // Above a ground-fog layer the sky is not veiled (`ground_fog`); quantised so the sky is not
    // rebuilt every frame while the camera climbs.
    let fog_clear = fog_view
        .as_ref()
        .map_or(0, |v| (v.sky_clear * 32.0).round() as u8);
    let fog_clear_changed = sky.last_fog_clear != fog_clear;
    if look_changed
        || fog_clear_changed
        || hdr_changed
        || jumped
        || effects_changed
        || (now - sky.last_refresh).abs() >= REFRESH_S
    {
        sky.last_refresh = now;
        sky.last_hdr = Some(hdr);
        sky.last_fog_clear = fog_clear;
        sky.last_revision = Some(env.revision());
        let weights = sky.config.sky_weights(elevation);
        // Colour of the sky at the horizon fills anything the dome does not cover; the fog
        // takes it too, greyed in thick fog.
        let h = sky.textures[0].height - 1;
        let mut horizon = [0.0; 3];
        for (ch, out) in horizon.iter_mut().enumerate() {
            for (t, wt) in sky.textures.iter().zip(weights) {
                *out += f32::from(t.rgba[(h * t.width) * 4 + ch]) * wt / 255.0;
            }
        }
        let conditions = env.conditions();
        let fog_on = effects.as_ref().is_none_or(|e| e.fog);
        let veil_color = opts_bevy::weather::fog_color(horizon, conditions);
        let data = blended_sky(
            &sky,
            weights,
            |v| {
                if fog_on {
                    opts_bevy::weather::sky_veil_clear(conditions, v, f32::from(fog_clear) / 32.0)
                } else {
                    0.0
                }
            },
            veil_color,
        );
        if let Some(mut img) = images.get_mut(&sky.image) {
            img.data = Some(data);
        }
        let l = |c: [f32; 3]| (c[0] + c[1] + c[2]) / 3.0;
        let a = sky.config.light_at(&sky.config.light_a, elevation);
        let b = sky.config.light_at(&sky.config.light_b, elevation);
        let c = sky.config.light_at(&sky.config.light_c, elevation);
        // The weather scales the light through `opts_bevy::weather::light_scales`.
        let la = env.light_attenuation();
        let (att, ambient_att) = (la.direct, la.ambient);
        for (_, mut light) in &mut sun {
            light.color = Color::srgb(a[0].max(0.001), a[1].max(0.001), a[2].max(0.001));
            light.illuminance = SUN_ILLUMINANCE * l(a) * att;
        }
        let amb = [c[0] + b[0] * 0.5, c[1] + b[1] * 0.5, c[2] + b[2] * 0.5];
        let peak = amb.iter().copied().fold(0.001_f32, f32::max);
        if let Some(ambient) = ambient.as_mut() {
            ambient.color = Color::srgb(amb[0] / peak, amb[1] / peak, amb[2] / peak);
            ambient.brightness = AMBIENT_BRIGHTNESS * ambient_scale * peak.min(1.0) * ambient_att;
        }
        // The clear colour fills what the dome does not cover: the fog colour, towards the
        // horizon colour as the camera leaves the ground-fog layer.
        let lift = f32::from(fog_clear) / 32.0;
        let clear_color = if fog_on {
            [0, 1, 2].map(|c| veil_color[c] + (horizon[c] - veil_color[c]) * lift)
        } else {
            horizon
        };
        if let Some(clear) = clear.as_mut() {
            let display = Color::srgb(clear_color[0], clear_color[1], clear_color[2]).to_linear();
            clear.0 = if hdr {
                // The HDR camera tonemaps its clear colour too; write what shows `display` after it.
                let tm = look
                    .as_ref()
                    .map_or_else(Default::default, |l| l.0.params().tonemapping);
                let x =
                    crate::look::hdr_clear_color(tm, [display.red, display.green, display.blue]);
                Color::linear_rgb(x[0], x[1], x[2])
            } else {
                display.into()
            };
        }
        commands.insert_resource(SkyHorizon(horizon));
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
}

#[cfg(test)]
mod tests;
