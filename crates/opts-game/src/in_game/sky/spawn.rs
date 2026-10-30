//! Spawning of the data-driven sky dome, lighting and discs.

use std::path::Path;

use bevy::asset::RenderAssetUsages;
use bevy::prelude::*;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};
use opts_bevy::ground::CpuImage;
use opts_core::formats::envir::EnvirConfig;

use crate::in_game::types::InGameEntity;

use super::{MoonDisc, MoonDiscTexture, Sky, SkyDome, SunDisc, SunLight, DOME_RADIUS};

fn load_texture(root: &Path, rel: &str) -> Result<Image, String> {
    let path = rel.replace('\\', "/");
    let full = opts_bevy::install::OmsiInstall::new(root)
        .find_file(&path)
        .ok_or_else(|| format!("{rel} not found"))?;
    let bytes = std::fs::read(&full).map_err(|e| format!("{}: {e}", full.display()))?;
    opts_bevy::texture::load_bevy_image(&bytes).map_err(|e| format!("{rel}: {e}"))
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
pub(super) fn blended_sky(
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

/// The empty, repeating sky texture `update_sky` fills.
fn sky_texture(w: u32, h: u32) -> Image {
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
    sky_image
}

/// The dome with the sky texture and the directional sunlight.
fn spawn_dome_and_sun(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    image: Handle<Image>,
) {
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
    let sky_image = sky_texture(w, h);
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

    spawn_dome_and_sun(commands, meshes, materials, sky.image.clone());
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
