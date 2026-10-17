//! The cloud layer of `[clouds]`: one disc at the base height that follows the camera, textured
//! with the cloud type's texture from `clouds.cfg`, scrolling downwind. A scattered type
//! (`sct`) shows its texture's gaps, an overcast one (`ovc`) is closed. The disc fades out at its
//! rim; the texture is fixed to the world, so driving does not slide the clouds.

use std::collections::HashMap;

use bevy::asset::RenderAssetUsages;
use bevy::light::{NotShadowCaster, NotShadowReceiver};
use bevy::math::Affine2;
use bevy::mesh::{Indices, PrimitiveTopology};
use bevy::prelude::*;
use opts_bevy::weather::{cloud_drift, weather_look};
use opts_bevy::{SessionEnvironment, WeatherCatalogue, WeatherEffects};

use crate::in_game::types::{InGameCamera, InGameEntity};

/// Radius of the disc; inside the camera's far plane.
pub const RADIUS_M: f32 = 950.0;
const SEGMENTS: usize = 48;
const RINGS: usize = 6;
/// The reference height follows the camera's height with this rate (1/s), so hills do not pass
/// through the layer.
const HEIGHT_FOLLOW: f32 = 0.3;

/// Marks the cloud disc.
#[derive(Component, Debug, Clone, Copy)]
pub struct CloudPlane;

/// State of the cloud layer.
#[derive(Resource, Default)]
pub struct CloudState {
    /// Loaded textures by file name (`None`: could not be read, do not try again).
    textures: HashMap<String, Option<Handle<Image>>>,
    mesh: Option<Handle<Mesh>>,
    /// Height the layer is measured from.
    reference_y: Option<f32>,
    /// The texture the material has.
    shown: Option<String>,
    /// The tint written to the material last.
    tint: f32,
}

/// The disc: radial rings, vertex alpha falling to 0 at the rim, UVs in metres.
fn disc_mesh() -> Mesh {
    let mut positions: Vec<[f32; 3]> = vec![[0.0, 0.0, 0.0]];
    let mut colors: Vec<[f32; 4]> = vec![[1.0; 4]];
    for ring in 1..=RINGS {
        let r = RADIUS_M * ring as f32 / RINGS as f32;
        let alpha = ((RADIUS_M - r) / (0.4 * RADIUS_M)).clamp(0.0, 1.0);
        for s in 0..SEGMENTS {
            let a = std::f32::consts::TAU * s as f32 / SEGMENTS as f32;
            positions.push([r * a.cos(), 0.0, r * a.sin()]);
            colors.push([1.0, 1.0, 1.0, alpha]);
        }
    }
    let uvs: Vec<[f32; 2]> = positions.iter().map(|p| [p[0], p[2]]).collect();
    let normals = vec![[0.0, -1.0, 0.0]; positions.len()];
    let at = |ring: usize, s: usize| (1 + (ring - 1) * SEGMENTS + s % SEGMENTS) as u32;
    let mut indices: Vec<u32> = Vec::new();
    for s in 0..SEGMENTS {
        indices.extend([0, at(1, s), at(1, s + 1)]);
    }
    for ring in 1..RINGS {
        for s in 0..SEGMENTS {
            let (a, b) = (at(ring, s), at(ring, s + 1));
            let (c, d) = (at(ring + 1, s), at(ring + 1, s + 1));
            indices.extend([a, c, b, b, c, d]);
        }
    }
    Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::default(),
    )
    .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, positions)
    .with_inserted_attribute(Mesh::ATTRIBUTE_NORMAL, normals)
    .with_inserted_attribute(Mesh::ATTRIBUTE_UV_0, uvs)
    .with_inserted_attribute(Mesh::ATTRIBUTE_COLOR, colors)
    .with_inserted_indices(Indices::U32(indices))
}

/// UV transform of the disc: the texture is fixed to the world and carried along by the wind.
/// `camera_xz` is the disc's position, `drift` the cloud speed in m/s, `t` seconds, `size_m` the
/// texture repeat.
#[must_use]
pub fn uv_transform(camera_xz: Vec2, drift: Vec2, t: f64, size_m: f32) -> Affine2 {
    let size = f64::from(size_m.max(1.0));
    let off =
        |cam: f32, d: f32| ((f64::from(cam) - f64::from(d) * t) / size).rem_euclid(1.0) as f32;
    Affine2::from_scale_angle_translation(
        Vec2::splat(1.0 / size_m.max(1.0)),
        0.0,
        Vec2::new(off(camera_xz.x, drift.x), off(camera_xz.y, drift.y)),
    )
}

fn load_texture(weather: &WeatherCatalogue, file: &str) -> Option<Image> {
    let root = weather.root()?;
    let path = opts_bevy::install::OmsiInstall::new(root).find_file(&format!("Texture/{file}"))?;
    let bytes = std::fs::read(path).ok()?;
    let mut image = opts_bevy::texture::load_bevy_image(&bytes).ok()?;
    opts_bevy::texture_filter::prepare_object_texture(&mut image);
    Some(image)
}

/// Spawns, moves, textures and removes the cloud disc.
#[allow(clippy::too_many_arguments)]
pub fn update_clouds(
    time: Res<Time<Real>>,
    env: Res<SessionEnvironment>,
    effects: Res<WeatherEffects>,
    weather: Res<WeatherCatalogue>,
    mut state: ResMut<CloudState>,
    mut images: ResMut<Assets<Image>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    cameras: Query<&GlobalTransform, With<InGameCamera>>,
    mut planes: Query<
        (Entity, &mut Transform, &MeshMaterial3d<StandardMaterial>),
        With<CloudPlane>,
    >,
    mut commands: Commands,
) {
    let conditions = env.conditions();
    let layer = conditions
        .cloud
        .as_ref()
        .filter(|_| effects.clouds)
        .and_then(|c| c.texture.as_ref().map(|t| (c, t.clone())));
    let Some((cloud, file)) = layer else {
        for (e, ..) in &planes {
            commands.entity(e).despawn();
        }
        return;
    };
    let Some(cam) = cameras.iter().next().map(GlobalTransform::translation) else {
        return;
    };
    let handle = state
        .textures
        .entry(file.clone())
        .or_insert_with(|| load_texture(&weather, &file).map(|i| images.add(i)))
        .clone();
    let Some(handle) = handle else {
        for (e, ..) in &planes {
            commands.entity(e).despawn();
        }
        return;
    };
    let dt = time.delta_secs();
    let reference = state.reference_y.get_or_insert(cam.y);
    *reference += (cam.y - *reference) * (1.0 - (-dt * HEIGHT_FOLLOW).exp());
    let y = *reference + cloud.base_m;
    let tint = weather_look(conditions, [0.0; 3], env.sun_alt_deg()).cloud;
    let colour = Color::linear_rgb(tint, tint, tint);
    let transform = uv_transform(
        Vec2::new(cam.x, cam.z),
        cloud_drift(conditions),
        time.elapsed_secs_f64(),
        cloud.size_m,
    );

    let Some((_, mut t, material)) = planes.iter_mut().next() else {
        let mesh = state
            .mesh
            .get_or_insert_with(|| meshes.add(disc_mesh()))
            .clone();
        let material = materials.add(StandardMaterial {
            base_color: colour,
            base_color_texture: Some(handle),
            unlit: true,
            alpha_mode: AlphaMode::Blend,
            cull_mode: None,
            uv_transform: transform,
            ..default()
        });
        state.shown = Some(file);
        state.tint = tint;
        commands.spawn((
            Mesh3d(mesh),
            MeshMaterial3d(material),
            Transform::from_xyz(cam.x, y, cam.z),
            NotShadowCaster,
            NotShadowReceiver,
            CloudPlane,
            InGameEntity,
            Name::new("Cloud_Layer"),
        ));
        return;
    };
    t.translation = Vec3::new(cam.x, y, cam.z);
    if let Some(mut m) = materials.get_mut(&material.0) {
        m.uv_transform = transform;
        if state.shown.as_deref() != Some(file.as_str()) {
            m.base_color_texture = Some(handle);
            state.shown = Some(file);
        }
        if (state.tint - tint).abs() > 0.004 {
            m.base_color = colour;
            state.tint = tint;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_texture_is_fixed_to_the_world_and_drifts_downwind() {
        let size = 2000.0;
        // Driving 100 m east moves the disc with the camera, so the texture offset follows.
        let a = uv_transform(Vec2::new(0.0, 0.0), Vec2::ZERO, 0.0, size);
        let b = uv_transform(Vec2::new(100.0, 0.0), Vec2::ZERO, 0.0, size);
        assert!((b.translation.x - a.translation.x - 100.0 / size).abs() < 1e-6);
        // Wind carrying the clouds east (+x) at 6 m/s: after 100 s the pattern sits 600 m further
        // east, so the offset (texture coordinate at the disc centre) has moved the other way.
        let c = uv_transform(Vec2::ZERO, Vec2::new(6.0, 0.0), 100.0, size);
        assert!((c.translation.x - (-600.0f32 / size).rem_euclid(1.0)).abs() < 1e-6);
        // The offset wraps and stays precise far from the origin.
        let far = uv_transform(Vec2::new(1.0e6, 0.0), Vec2::new(1.0, 0.0), 1.0e5, size);
        assert!((0.0..1.0).contains(&far.translation.x));
    }

    #[test]
    fn the_disc_has_a_closed_fan_and_fades_at_its_rim() {
        let mesh = disc_mesh();
        let count = mesh.count_vertices();
        assert_eq!(count, 1 + RINGS * SEGMENTS);
        let Some(Indices::U32(idx)) = mesh.indices() else {
            panic!("indices");
        };
        assert_eq!(idx.len(), 3 * SEGMENTS + 6 * SEGMENTS * (RINGS - 1));
        assert!(idx.iter().all(|i| (*i as usize) < count));
        let bevy::mesh::VertexAttributeValues::Float32x4(colors) =
            mesh.attribute(Mesh::ATTRIBUTE_COLOR).unwrap()
        else {
            panic!("colours");
        };
        assert_eq!(colors[0][3], 1.0);
        assert_eq!(colors[count - 1][3], 0.0);
    }
}
