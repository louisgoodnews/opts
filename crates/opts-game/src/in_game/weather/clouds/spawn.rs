//! Creating the cloud layer: the disc, its material and the inputs of the update.

use bevy::prelude::*;

use super::*;

pub(super) fn load_texture(weather: &WeatherCatalogue, file: &str) -> Option<Image> {
    let root = weather.root()?;
    let path = opts_bevy::install::OmsiInstall::new(root).find_file(&format!("Texture/{file}"))?;
    let bytes = std::fs::read(path).ok()?;
    let mut image = opts_bevy::texture::load_bevy_image(&bytes).ok()?;
    opts_bevy::texture_filter::prepare_object_texture(&mut image);
    Some(image)
}

/// Creates the cloud layer: the disc, an unlit blended material with the cloud texture, at
/// `position`; remembers which texture and tint it shows.
pub(super) fn spawn_plane(
    commands: &mut Commands,
    (meshes, materials): (&mut Assets<Mesh>, &mut Assets<StandardMaterial>),
    state: &mut CloudState,
    (handle, file): (Handle<Image>, String),
    (colour, tint, transform): (Color, f32, bevy::math::Affine2),
    position: Vec3,
) {
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
        Transform::from_translation(position),
        NotShadowCaster,
        NotShadowReceiver,
        CloudPlane,
        InGameEntity,
        Name::new("Cloud_Layer"),
    ));
}

/// What [`update_clouds`] reads and moves besides the weather: the catalogue, camera and planes.
#[derive(bevy::ecs::system::SystemParam)]
pub struct CloudInputs<'w, 's> {
    pub(super) weather: Res<'w, WeatherCatalogue>,
    pub(super) cameras: Query<'w, 's, &'static GlobalTransform, With<InGameCamera>>,
    pub(super) planes: Query<
        'w,
        's,
        (
            Entity,
            &'static mut Transform,
            &'static MeshMaterial3d<StandardMaterial>,
        ),
        With<CloudPlane>,
    >,
}
