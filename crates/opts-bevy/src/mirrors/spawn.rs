//! Creating the mirror cameras and their surfaces for a vehicle.

use bevy::ecs::system::SystemParam;
use bevy::prelude::*;
use bevy::render::render_resource::TextureFormat;

use super::*;

pub(super) fn mirror_material(
    original: &StandardMaterial,
    picture: Handle<Image>,
    uv_min: Vec2,
    uv_max: Vec2,
    settings: &MirrorSettings,
) -> StandardMaterial {
    let mut m = original.clone();
    m.base_color_texture = Some(picture);
    m.base_color = Color::WHITE;
    m.unlit = true;
    m.alpha_mode = AlphaMode::Opaque;
    m.double_sided = true;
    m.cull_mode = None;
    m.uv_transform = pane_uv_transform(uv_min, uv_max, settings.flip_u, settings.flip_v);
    m
}

/// What a scan learned about one pane mesh.
pub(super) struct Found {
    pub(super) member: MirrorMember,
    pub(super) uv_min: Vec2,
    pub(super) uv_max: Vec2,
}

type CandidateMeshQuery<'w, 's> = Query<
    'w,
    's,
    (&'static MeshMaterial3d<StandardMaterial>, &'static Mesh3d),
    Without<MirrorSurface>,
>;

/// Parameters for [`spawn_mirror_cameras`].
#[derive(SystemParam)]
pub struct MirrorSpawnParams<'w, 's> {
    pub time: Res<'w, Time<Real>>,
    pub quality: Res<'w, LightQuality>,
    pub settings: Res<'w, MirrorSettings>,
    pub images: ResMut<'w, Assets<Image>>,
    pub materials: ResMut<'w, Assets<StandardMaterial>>,
    pub virtual_mirrors: Res<'w, VirtualMirrors>,
    pub meshes: Res<'w, Assets<Mesh>>,
    pub vehicles: Query<'w, 's, Entity, With<PlayerVehicle>>,
    pub buses: Query<'w, 's, &'static BusVehicle>,
    pub children: Query<'w, 's, &'static Children>,
    pub cameras: Query<'w, 's, (Entity, &'static mut MirrorCamera)>,
    pub candidates: CandidateMeshQuery<'w, 's>,
}

fn scan_vehicle_panes(
    vehicle: Entity,
    children: &Query<&Children>,
    candidates: &CandidateMeshQuery,
    meshes: &Assets<Mesh>,
) -> HashMap<GroupKey, (Handle<StandardMaterial>, Vec<Found>)> {
    let mut groups: HashMap<GroupKey, (Handle<StandardMaterial>, Vec<Found>)> = HashMap::new();
    for entity in children.iter_descendants(vehicle) {
        let Ok((material, mesh)) = candidates.get(entity) else {
            continue;
        };
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
        groups
            .entry(GroupKey::of(material.0.id()))
            .or_insert_with(|| (material.0.clone(), Vec::new()))
            .1
            .push(Found {
                member: MirrorMember {
                    entity,
                    centre,
                    normal,
                    height,
                    aspect,
                    original: material.0.clone(),
                },
                uv_min,
                uv_max,
            });
    }
    groups
}

fn join_existing_group(
    camera: Entity,
    found: Vec<Found>,
    cameras: &mut Query<(Entity, &mut MirrorCamera)>,
    materials: &mut Assets<StandardMaterial>,
    settings: &MirrorSettings,
    commands: &mut Commands,
) {
    if let Ok((_, mut info)) = cameras.get_mut(camera) {
        for f in found {
            info.uv_min = info.uv_min.min(f.uv_min);
            info.uv_max = info.uv_max.max(f.uv_max);
            commands
                .entity(f.member.entity)
                .insert(MirrorSurface { camera });
            info.members.push(f.member);
        }
        let (dynamic, lo, hi) = (info.dynamic.clone(), info.uv_min, info.uv_max);
        if let Some(mut m) = materials.get_mut(&dynamic) {
            m.uv_transform = if info.tag.is_some() {
                pane_uv_transform(Vec2::ZERO, Vec2::ONE, settings.flip_u, settings.flip_v)
            } else {
                pane_uv_transform(lo, hi, settings.flip_u, settings.flip_v)
            };
        }
    }
}

fn create_mirror_image(size: UVec2) -> Image {
    let mut image = Image::new_target_texture(size.x, size.y, TextureFormat::Bgra8UnormSrgb, None);
    image.sampler = bevy::image::ImageSampler::Descriptor(bevy::image::ImageSamplerDescriptor {
        address_mode_u: bevy::image::ImageAddressMode::Repeat,
        address_mode_v: bevy::image::ImageAddressMode::Repeat,
        mag_filter: bevy::image::ImageFilterMode::Linear,
        min_filter: bevy::image::ImageFilterMode::Linear,
        ..default()
    });
    image
}

fn spawn_camera_entity(
    picture: Handle<Image>,
    aspect_ratio: f32,
    commands: &mut Commands,
) -> Entity {
    commands
        .spawn((
            Camera3d::default(),
            Camera {
                order: MIRROR_ORDER,
                is_active: false,
                ..default()
            },
            RenderTarget::Image(picture.into()),
            Msaa::Off,
            Hdr,
            Tonemapping::None,
            Projection::Perspective(PerspectiveProjection {
                fov: 40f32.to_radians(),
                near: 0.2,
                aspect_ratio,
                ..default()
            }),
            Transform::default(),
            Name::new("Mirror_Camera"),
        ))
        .id()
}

fn create_mirror_camera(
    vehicle: Entity,
    key: GroupKey,
    original: Handle<StandardMaterial>,
    found: Vec<Found>,
    tag: Option<BusMirrorCamera>,
    params: &mut MirrorSpawnParams,
    commands: &mut Commands,
) -> Option<Entity> {
    let base = params.materials.get(&original)?.clone();
    let size = if tag.is_some() {
        let side = ((params.settings.size.x * params.settings.size.y) as f32)
            .sqrt()
            .round() as u32;
        UVec2::splat(side.clamp(32, 1024))
    } else {
        params.settings.size
    };
    let picture = params.images.add(create_mirror_image(size));
    let (uv_min, uv_max) = found.iter().fold(
        (Vec2::splat(f32::MAX), Vec2::splat(f32::MIN)),
        |(lo, hi), f| (lo.min(f.uv_min), hi.max(f.uv_max)),
    );
    let (pic_min, pic_max) = if tag.is_some() {
        (Vec2::ZERO, Vec2::ONE)
    } else {
        (uv_min, uv_max)
    };
    let dynamic = params.materials.add(mirror_material(
        &base,
        picture.clone(),
        pic_min,
        pic_max,
        &params.settings,
    ));
    let aspect = found.iter().map(|f| f.member.aspect).sum::<f32>() / found.len() as f32;
    let aspect_ratio = if tag.is_some() {
        1.0
    } else {
        aspect.clamp(MIRROR_ASPECT_RANGE.0, MIRROR_ASPECT_RANGE.1)
    };
    let members: Vec<MirrorMember> = found.into_iter().map(|f| f.member).collect();
    let camera = spawn_camera_entity(picture.clone(), aspect_ratio, commands);
    for m in &members {
        commands.entity(m.entity).insert(MirrorSurface { camera });
    }
    commands.entity(camera).insert(MirrorCamera {
        vehicle,
        key,
        original,
        dynamic,
        members,
        uv_min,
        uv_max,
        picture,
        base_size: size,
        enlarged: false,
        tag,
        last_render: 0,
    });
    Some(camera)
}

/// Finds the mirror panes of the player vehicle and gives each group of panes a camera.
pub fn spawn_mirror_cameras(
    mut params: MirrorSpawnParams,
    mut last_scan: Local<Option<f64>>,
    mut commands: Commands,
) {
    if params.quality.mirror_count == 0 && !params.virtual_mirrors.needs_cameras() {
        return;
    }
    let now = params.time.elapsed_secs_f64();
    if last_scan.is_some_and(|t| now - t < params.settings.scan_interval_s) {
        return;
    }
    *last_scan = Some(now);

    let vehicles: Vec<Entity> = params.vehicles.iter().collect();
    for vehicle in vehicles {
        let groups = scan_vehicle_panes(
            vehicle,
            &params.children,
            &params.candidates,
            &params.meshes,
        );
        let mut existing: HashMap<GroupKey, Entity> = params
            .cameras
            .iter()
            .filter(|(_, c)| c.vehicle == vehicle)
            .map(|(e, c)| (c.key, e))
            .collect();
        for (key, (original, found)) in groups {
            if let Some(camera) = existing.get(&key).copied() {
                join_existing_group(
                    camera,
                    found,
                    &mut params.cameras,
                    &mut params.materials,
                    &params.settings,
                    &mut commands,
                );
                continue;
            }
            if existing.len() >= MAX_CAMERAS {
                continue;
            }
            let tag = if params.settings.use_tags {
                match key {
                    GroupKey::Index(n) => Some(n),
                    GroupKey::Material(id) => mirror_index(id),
                }
                .and_then(|n| {
                    params
                        .buses
                        .get(vehicle)
                        .ok()
                        .and_then(|b| b.mirror_cameras.get(n as usize).cloned())
                })
            } else {
                None
            };
            if let Some(camera) = create_mirror_camera(
                vehicle,
                key,
                original,
                found,
                tag,
                &mut params,
                &mut commands,
            ) {
                existing.insert(key, camera);
            }
        }
    }
}
