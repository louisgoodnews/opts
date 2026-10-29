use bevy::prelude::*;
use bevy::render::render_resource::TextureFormat;

use crate::mirrors::*;

use super::{AmbientCamera, AmbientPane, AMBIENT_PICTURE};

pub(super) fn restore_pane_material(
    pane: Entity,
    panes: &Query<
        (
            Entity,
            &AmbientPane,
            &GlobalTransform,
            Option<&InheritedVisibility>,
        ),
        Without<MirrorSurface>,
    >,
    pane_materials: &mut Query<&mut MeshMaterial3d<StandardMaterial>>,
) {
    if let (Ok((_, info, ..)), Ok(mut m)) = (panes.get(pane), pane_materials.get_mut(pane)) {
        if m.0 != info.original {
            m.0 = info.original.clone();
        }
    }
}

pub(super) fn sync_pool_capacity(
    budget: usize,
    cameras: &Query<(
        Entity,
        &mut AmbientCamera,
        &mut Transform,
        &mut Camera,
        &mut Projection,
    )>,
    panes: &Query<
        (
            Entity,
            &AmbientPane,
            &GlobalTransform,
            Option<&InheritedVisibility>,
        ),
        Without<MirrorSurface>,
    >,
    pane_materials: &mut Query<&mut MeshMaterial3d<StandardMaterial>>,
    images: &mut Assets<Image>,
    materials: &mut Assets<StandardMaterial>,
    commands: &mut Commands,
) {
    let existing: Vec<Entity> = cameras.iter().map(|c| c.0).collect();
    for extra in existing.iter().skip(budget) {
        if let Ok((_, info, ..)) = cameras.get(*extra) {
            if let Some(p) = info.pane {
                restore_pane_material(p, panes, pane_materials);
            }
        }
        commands.entity(*extra).despawn();
    }
    for _ in existing.len()..budget {
        let mut image = Image::new_target_texture(
            AMBIENT_PICTURE,
            AMBIENT_PICTURE,
            TextureFormat::Bgra8UnormSrgb,
            None,
        );
        image.sampler =
            bevy::image::ImageSampler::Descriptor(bevy::image::ImageSamplerDescriptor {
                address_mode_u: bevy::image::ImageAddressMode::Repeat,
                address_mode_v: bevy::image::ImageAddressMode::Repeat,
                mag_filter: bevy::image::ImageFilterMode::Linear,
                min_filter: bevy::image::ImageFilterMode::Linear,
                ..default()
            });
        let picture = images.add(image);
        let dynamic = materials.add(StandardMaterial::default());
        commands.spawn((
            Camera3d::default(),
            Camera {
                order: MIRROR_ORDER,
                is_active: false,
                ..default()
            },
            RenderTarget::Image(picture.clone().into()),
            Msaa::Off,
            Hdr,
            Tonemapping::None,
            Projection::Perspective(PerspectiveProjection {
                fov: 40f32.to_radians(),
                near: 0.2,
                ..default()
            }),
            Transform::default(),
            Name::new("Ambient_Mirror_Camera"),
            AmbientCamera {
                picture,
                dynamic,
                pane: None,
            },
        ));
    }
}
