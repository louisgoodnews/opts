//! Creating the mirror cameras and their surfaces for a vehicle.

use super::*;

fn mirror_material(
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

/// Finds the mirror panes of the player vehicle and gives each group of panes a camera.
#[allow(clippy::too_many_arguments, clippy::type_complexity)]
pub fn spawn_mirror_cameras(
    time: Res<Time<Real>>,
    quality: Res<LightQuality>,
    settings: Res<MirrorSettings>,
    mut last_scan: Local<Option<f64>>,
    mut commands: Commands,
    mut images: ResMut<Assets<Image>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    meshes: Res<Assets<Mesh>>,
    vehicles: Query<Entity, With<PlayerVehicle>>,
    buses: Query<&BusVehicle>,
    children: Query<&Children>,
    mut cameras: Query<(Entity, &mut MirrorCamera)>,
    candidates: Query<(&MeshMaterial3d<StandardMaterial>, &Mesh3d), Without<MirrorSurface>>,
) {
    if quality.mirror_count == 0 {
        return;
    }
    let now = time.elapsed_secs_f64();
    if last_scan.is_some_and(|t| now - t < settings.scan_interval_s) {
        return;
    }
    *last_scan = Some(now);
    for vehicle in &vehicles {
        // New panes of this vehicle, grouped by their (pooled) material.
        let mut groups: HashMap<AssetId<StandardMaterial>, (Handle<StandardMaterial>, Vec<Found>)> =
            HashMap::new();
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
                .entry(material.0.id())
                .or_insert_with(|| (material.0.clone(), Vec::new()))
                .1
                .push(Found {
                    member: MirrorMember {
                        entity,
                        centre,
                        normal,
                        height,
                        aspect,
                    },
                    uv_min,
                    uv_max,
                });
        }
        let mut existing: HashMap<AssetId<StandardMaterial>, Entity> = cameras
            .iter()
            .filter(|(_, c)| c.vehicle == vehicle)
            .map(|(e, c)| (c.material, e))
            .collect();
        for (id, (original, found)) in groups {
            if let Some(camera) = existing.get(&id).copied() {
                // Late panes join the camera of their group.
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
                            pane_uv_transform(
                                Vec2::ZERO,
                                Vec2::ONE,
                                settings.flip_u,
                                settings.flip_v,
                            )
                        } else {
                            pane_uv_transform(lo, hi, settings.flip_u, settings.flip_v)
                        };
                    }
                }
                continue;
            }
            if existing.len() >= MAX_CAMERAS {
                continue;
            }
            let Some(base) = materials.get(&original).cloned() else {
                continue;
            };
            let tag = if settings.use_tags {
                mirror_index(id).and_then(|n| {
                    buses
                        .get(vehicle)
                        .ok()
                        .and_then(|b| b.mirror_cameras.get(n as usize).cloned())
                })
            } else {
                None
            };
            // A tag camera fills the whole (square) texture, as in OMSI; the pane shows the part
            // its UVs select. Same pixel count as the pane picture.
            let size = if tag.is_some() {
                let side = ((settings.size.x * settings.size.y) as f32).sqrt().round() as u32;
                UVec2::splat(side.clamp(32, 1024))
            } else {
                settings.size
            };
            let mut image =
                Image::new_target_texture(size.x, size.y, TextureFormat::Bgra8UnormSrgb, None);
            image.sampler = bevy::image::ImageSampler::linear();
            let picture = images.add(image);
            let (uv_min, uv_max) = found.iter().fold(
                (Vec2::splat(f32::MAX), Vec2::splat(f32::MIN)),
                |(lo, hi), f| (lo.min(f.uv_min), hi.max(f.uv_max)),
            );
            let (pic_min, pic_max) = if tag.is_some() {
                (Vec2::ZERO, Vec2::ONE)
            } else {
                (uv_min, uv_max)
            };
            let dynamic = materials.add(mirror_material(
                &base,
                picture.clone(),
                pic_min,
                pic_max,
                &settings,
            ));
            let aspect = found.iter().map(|f| f.member.aspect).sum::<f32>() / found.len() as f32;
            let members: Vec<MirrorMember> = found.into_iter().map(|f| f.member).collect();
            let camera = commands
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
                        aspect_ratio: if tag.is_some() {
                            1.0
                        } else {
                            aspect.clamp(MIRROR_ASPECT_RANGE.0, MIRROR_ASPECT_RANGE.1)
                        },
                        ..default()
                    }),
                    Transform::default(),
                    Name::new("Mirror_Camera"),
                ))
                .id();
            for m in &members {
                commands.entity(m.entity).insert(MirrorSurface { camera });
            }
            commands.entity(camera).insert(MirrorCamera {
                vehicle,
                material: id,
                original,
                dynamic,
                members,
                uv_min,
                uv_max,
                tag,
            });
            existing.insert(id, camera);
        }
    }
}
