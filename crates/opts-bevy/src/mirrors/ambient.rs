//! Live mirrors on the vehicles around the player (parked cars, AI traffic).
//!
//! Only the player's own vehicle gets a camera per mirror ([`super::spawn`]). Every other
//! vehicle would keep the static `reflexion<N>.bmp` of its pane, a fixed road scene. This module
//! gives the **nearest** other mirrors a live picture, with the geometric reflection of the
//! viewer's ray ([`mirror_camera_pose`]):
//!
//! * **Panes.** [`register_ambient_panes`] marks every mesh that gets a mirror material
//!   ([`SurfaceKind::Mirror`]) with an [`AmbientPane`] (its geometry, UV rectangle and own static
//!   material), once, when the mesh appears. The player's own panes carry the mark too and are
//!   skipped when choosing (they have [`MirrorSurface`] or a [`PlayerVehicle`] ancestor).
//! * **Budget.** [`LightQuality::mirror_count`] cameras (the same budget as the player's mirrors:
//!   2 up to level 3, none above) serve the panes that are visible, within
//!   [`AMBIENT_RANGE_M`], in front of the main camera and facing it, nearest first. A pane keeps
//!   its camera while it stays among the nearest, so the picture does not jump around.
//! * **Cost.** Small pictures ([`AMBIENT_PICTURE`] pixels square) rendered every
//!   [`AMBIENT_EVERY_FACTOR`] times as seldom as the player's mirrors, staggered by rank. A pane
//!   that loses its camera gets its own static material back.

use super::spawn::mirror_material;
use super::*;

/// Farthest distance (m) at which another vehicle's mirror is live.
pub const AMBIENT_RANGE_M: f32 = 30.0;
/// Side of the picture (pixels).
pub const AMBIENT_PICTURE: u32 = 192;
/// How many times less often than the player's mirrors an ambient mirror renders.
pub const AMBIENT_EVERY_FACTOR: u8 = 2;
/// Smallest `|cos|` between the pane's normal and the direction to the viewer: a pane seen
/// edge-on is not worth a camera.
pub const AMBIENT_MIN_FACING: f32 = 0.15;

/// A mirror pane of any vehicle (set once, when the mesh appears).
#[derive(Component, Debug, Clone)]
pub struct AmbientPane {
    /// Centre of the pane in the mesh's local space.
    pub centre: Vec3,
    /// Mean normal of the pane in the mesh's local space.
    pub normal: Vec3,
    /// Extent along the pane's up direction (local metres, before scale).
    pub height: f32,
    /// Width over height.
    pub aspect: f32,
    /// The pane's UV rectangle.
    pub uv_min: Vec2,
    /// The pane's UV rectangle.
    pub uv_max: Vec2,
    /// The pane's own static material.
    pub original: Handle<StandardMaterial>,
}

/// A camera of the pool, and the pane it serves.
#[derive(Component, Debug, Clone)]
pub struct AmbientCamera {
    /// The picture it renders into.
    pub picture: Handle<Image>,
    /// The material that shows the picture on the pane.
    pub dynamic: Handle<StandardMaterial>,
    /// The pane it serves now.
    pub pane: Option<Entity>,
}

/// Marks new mirror meshes.
#[allow(clippy::type_complexity)]
pub fn register_ambient_panes(
    mut commands: Commands,
    meshes: Res<Assets<Mesh>>,
    new: Query<
        (Entity, &MeshMaterial3d<StandardMaterial>, &Mesh3d),
        (
            Added<MeshMaterial3d<StandardMaterial>>,
            Without<AmbientPane>,
        ),
    >,
) {
    for (entity, material, mesh) in &new {
        if surface_kind(material.0.id()) != Some(SurfaceKind::Mirror) {
            continue;
        }
        let Some(mesh) = meshes.get(&mesh.0) else {
            continue;
        };
        let (Some(pos), Some(nor)) = (
            mesh.attribute(Mesh::ATTRIBUTE_POSITION)
                .and_then(|a| a.as_float3()),
            mesh.attribute(Mesh::ATTRIBUTE_NORMAL)
                .and_then(|a| a.as_float3()),
        ) else {
            continue;
        };
        let Some((centre, normal, height, aspect)) = pane_geometry(pos, nor) else {
            continue;
        };
        let (uv_min, uv_max) = mesh
            .attribute(Mesh::ATTRIBUTE_UV_0)
            .and_then(|a| match a {
                bevy::mesh::VertexAttributeValues::Float32x2(v) => uv_bounds(v),
                _ => None,
            })
            .unwrap_or((Vec2::ZERO, Vec2::ONE));
        commands.entity(entity).insert(AmbientPane {
            centre,
            normal,
            height,
            aspect,
            uv_min,
            uv_max,
            original: material.0.clone(),
        });
    }
}

/// A pane that is a candidate this frame.
struct Candidate {
    pane: Entity,
    distance: f32,
    centre: Vec3,
    normal: Vec3,
    height: f32,
    aspect: f32,
}

/// Chooses the panes, keeps the pool at the budget and poses the cameras.
#[allow(clippy::too_many_arguments, clippy::type_complexity)]
pub fn update_ambient_mirrors(
    mut commands: Commands,
    quality: Res<LightQuality>,
    settings: Res<MirrorSettings>,
    frame: Res<MirrorFrame>,
    mut stats: ResMut<MirrorStats>,
    mut images: ResMut<Assets<Image>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    main: Query<
        &GlobalTransform,
        (
            With<MainCamera>,
            Without<AmbientCamera>,
            Without<MirrorCamera>,
        ),
    >,
    players: Query<(), With<PlayerVehicle>>,
    parents: Query<&ChildOf>,
    panes: Query<
        (
            Entity,
            &AmbientPane,
            &GlobalTransform,
            Option<&InheritedVisibility>,
        ),
        Without<MirrorSurface>,
    >,
    mut pane_materials: Query<&mut MeshMaterial3d<StandardMaterial>>,
    mut cameras: Query<(
        Entity,
        &mut AmbientCamera,
        &mut Transform,
        &mut Camera,
        &mut Projection,
    )>,
) {
    let budget = usize::from(quality.mirror_count);
    let restore =
        |pane: Entity, pane_materials: &mut Query<&mut MeshMaterial3d<StandardMaterial>>| {
            if let (Ok((_, info, ..)), Ok(mut m)) = (panes.get(pane), pane_materials.get_mut(pane))
            {
                if m.0 != info.original {
                    m.0 = info.original.clone();
                }
            }
        };
    // The pool follows the budget.
    let existing: Vec<Entity> = cameras.iter().map(|c| c.0).collect();
    for extra in existing.iter().skip(budget) {
        if let Ok((_, info, ..)) = cameras.get(*extra) {
            if let Some(p) = info.pane {
                restore(p, &mut pane_materials);
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
    stats.ambient = 0;
    let Some(view) = main.iter().next() else {
        for (_, _, _, mut cam, _) in &mut cameras {
            if cam.is_active {
                cam.is_active = false;
            }
        }
        return;
    };
    if budget == 0 {
        return;
    }
    let (viewer, forward) = (view.translation(), view.forward().as_vec3());

    // The panes worth a camera, nearest first.
    let mut candidates: Vec<Candidate> = Vec::new();
    for (entity, pane, gt, visible) in &panes {
        if visible.is_some_and(|v| !v.get()) {
            continue;
        }
        let centre = gt.transform_point(pane.centre);
        let distance = centre.distance(viewer);
        if distance > AMBIENT_RANGE_M || !mirror_in_front(viewer, forward, centre) {
            continue;
        }
        let normal = gt
            .affine()
            .matrix3
            .mul_vec3(pane.normal)
            .normalize_or_zero();
        let to_viewer = (viewer - centre).normalize_or_zero();
        if normal.dot(to_viewer).abs() < AMBIENT_MIN_FACING {
            continue;
        }
        if parents.iter_ancestors(entity).any(|a| players.contains(a)) {
            continue;
        }
        let (scale, _, _) = gt.to_scale_rotation_translation();
        candidates.push(Candidate {
            pane: entity,
            distance,
            centre,
            normal,
            height: pane.height * scale.max_element(),
            aspect: pane.aspect,
        });
    }
    candidates.sort_by(|a, b| a.distance.total_cmp(&b.distance).then(a.pane.cmp(&b.pane)));
    candidates.truncate(budget);

    // A camera keeps its pane while that is still among the nearest; the others take the rest.
    let mut taken: Vec<Entity> = Vec::new();
    let mut free: Vec<Entity> = Vec::new();
    for (entity, mut info, ..) in &mut cameras {
        match info.pane {
            Some(p) if candidates.iter().any(|c| c.pane == p) => taken.push(p),
            Some(p) => {
                restore(p, &mut pane_materials);
                info.pane = None;
                free.push(entity);
            }
            None => free.push(entity),
        }
    }
    let mut waiting = candidates.iter().filter(|c| !taken.contains(&c.pane));
    for camera in free {
        let Some(c) = waiting.next() else { break };
        let Ok((_, mut info, ..)) = cameras.get_mut(camera) else {
            continue;
        };
        if let Ok((_, pane, ..)) = panes.get(c.pane) {
            if let Some(base) = materials.get(&pane.original).cloned() {
                let new = mirror_material(
                    &base,
                    info.picture.clone(),
                    pane.uv_min,
                    pane.uv_max,
                    &settings,
                );
                if let Some(mut m) = materials.get_mut(&info.dynamic) {
                    *m = new;
                }
            }
        }
        info.pane = Some(c.pane);
    }

    let every = quality.mirror_every.saturating_mul(AMBIENT_EVERY_FACTOR);
    let mut rank = 0;
    for (_, info, mut t, mut cam, mut projection) in &mut cameras {
        let Some(pane) = info.pane else {
            if cam.is_active {
                cam.is_active = false;
            }
            continue;
        };
        let Some(c) = candidates.iter().find(|c| c.pane == pane) else {
            continue;
        };
        if let Ok(mut m) = pane_materials.get_mut(pane) {
            if m.0 != info.dynamic {
                m.0 = info.dynamic.clone();
            }
        }
        if let Some((position, direction)) = mirror_camera_pose(viewer, c.centre, c.normal) {
            t.translation = position;
            t.look_to(direction, Vec3::Y);
        }
        if let Projection::Perspective(persp) = &mut *projection {
            persp.fov = mirror_fov(c.height, c.distance);
            persp.aspect_ratio = c.aspect.clamp(MIRROR_ASPECT_RANGE.0, MIRROR_ASPECT_RANGE.1);
        }
        let render = is_render_frame(frame.0, rank, every);
        if cam.is_active != render {
            cam.is_active = render;
        }
        stats.ambient += 1;
        rank += 1;
    }
}
