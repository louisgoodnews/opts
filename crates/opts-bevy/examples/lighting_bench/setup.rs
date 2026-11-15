//! Scene setup.

use super::cone_material::cone_mesh;
use super::procedural::{hash, rgba_image, Rng};
use super::*;

pub(super) fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut images: ResMut<Assets<Image>>,
    win: Res<WindowModeFlag>,
) {
    let mut rng = Rng(0x1234_5678_9abc_def1);
    let cone_handle = meshes.add(cone_mesh());

    let facade = facade_materials(&mut materials, &mut images);
    spawn_boxes(&mut commands, &mut meshes, &facade, &mut rng);
    spawn_road(&mut commands, &mut meshes, &mut materials, &mut images);
    spawn_trees(
        &mut commands,
        &mut meshes,
        &mut materials,
        &mut images,
        &mut rng,
    );
    spawn_lights(&mut commands);
    let target = spawn_camera(&mut commands, &mut images, win.0);
    commands.insert_resource(Assets3 {
        cone: cone_handle,
        target,
        quad: meshes.add(Rectangle::new(1.0, 1.0)),
    });
}

mod lights;
mod materials;
mod scenery;

use lights::*;
use materials::*;
use scenery::*;

/// Camera; returns the offscreen render target, if any.
fn spawn_camera(
    commands: &mut Commands,
    images: &mut Assets<Image>,
    window_mode: bool,
) -> Option<Handle<Image>> {
    let mut cam = commands.spawn((
        Camera3d::default(),
        opts_bevy::MainCamera,
        Transform::from_xyz(0.0, 1.8, 6.0).looking_to(Vec3::NEG_Z, Vec3::Y),
        Tonemapping::default(),
        Msaa::Sample4,
        RenderLayers::from_layers(&[0, 1]),
        Cam,
    ));
    let mut target = None;
    if !window_mode {
        let mut image = Image::new_fill(
            Extent3d {
                width: W,
                height: H,
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
        cam.insert(RenderTarget::Image(handle.clone().into()));
        target = Some(handle);
    }
    let _ = RenderLayers::layer(0);
    target
}
