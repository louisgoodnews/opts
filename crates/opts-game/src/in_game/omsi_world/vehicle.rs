//! Player vehicle, trailers and driver loading.

use std::path::{Path, PathBuf};

use bevy::prelude::*;
use opts_bevy::coords::{
    camera_preset_to_bevy_transform, omsi_euler_to_bevy_quat, omsi_to_bevy_pos,
};
use opts_bevy::{
    attach_model_parts, spawn_vehicle_model, ContentLoader, PlayerVehicle, TextureOverride,
    VehicleController,
};
use opts_core::formats::vehicles::model::ModelConfig;
use opts_core::formats::vehicles::BusManifest;

use crate::in_game::types::InGameEntity;

/// Asset stores the loading systems write to (grouped to stay within Bevy's parameter limit).
use super::{CameraSetup, WorldLoadJob, WorldStats};

/// Reads the first vehicle listed in the map's `ailists.cfg` (fallback when no `--vehicle-path`).
pub(super) fn first_ai_list_vehicle(job: &WorldLoadJob) -> Option<PathBuf> {
    let cfg = job.install.find_in(&job.config.map_dir, "ailists.cfg")?;
    let text = opts_core::formats::text::decode_bytes(&std::fs::read(cfg).ok()?);
    text.lines()
        .map(str::trim)
        .filter(|l| l.to_lowercase().contains(".bus"))
        .find_map(|l| {
            let rel = l.split('\t').next()?.trim();
            job.install.find_file(rel)
        })
}

/// Expected mesh box of a vehicle from its `.bus` `[boundingbox]` (restricts the scrambled-mesh
/// decoder; see `docs/src/formats/binary/o3d.md`, Addendum 9).
fn manifest_bounds(
    manifest: &BusManifest,
) -> Option<opts_core::formats::o3d::descramble::ExpectedBounds> {
    manifest.bounding_box.map(|(half, centre)| {
        opts_core::formats::o3d::descramble::ExpectedBounds::from_omsi_box(half, centre)
    })
}

/// Reads the repaints of the player's vehicle once per session (they come from the `.cti` files of
/// its `Colorscheme` folders, `docs/src/formats/text/repaints.md`).
fn ensure_repaints(job: &mut WorldLoadJob, bus_path: &Path, manifest: &BusManifest) {
    if job.repaint.is_none() {
        let default = manifest
            .friendly_name
            .as_ref()
            .map(|f| f.livery.clone())
            .unwrap_or_default();
        job.repaint = Some(crate::new_game_screen::repaints::discover_at(
            &job.config.root,
            bus_path,
            &default,
        ));
    }
}

/// Texture replacements of the chosen repaint for the model of one segment: each `[CTCTexture]`
/// slot's default file is swapped for the repaint's file of that slot.
fn repaint_map(
    job: &WorldLoadJob,
    model: &ModelConfig,
) -> std::collections::HashMap<String, PathBuf> {
    let Some(r) = &job.repaint else {
        return Default::default();
    };
    let name = if job.config.livery.is_empty() {
        r.default_name.as_str()
    } else {
        job.config.livery.as_str()
    };
    r.replacements(name, &model.ctc_textures)
        .into_iter()
        .collect()
}

/// Loads the visible lod0 meshes of one vehicle segment as children of `root`.
/// Returns `(mesh_count, vertex_count, bounds_min, bounds_max)` (Bevy space, body meshes only).
#[allow(clippy::too_many_arguments)]
pub(super) fn load_segment_meshes(
    job: &mut WorldLoadJob,
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    images: &mut Assets<Image>,
    root: Entity,
    lod0: &opts_core::formats::vehicles::model::ModelLod,
    model_dir: &Path,
    tex_dirs: &[PathBuf],
) -> (usize, usize, Vec3, Vec3) {
    load_segment_meshes_for(
        job, commands, meshes, materials, images, root, root, lod0, model_dir, tex_dirs,
    )
}

/// [`load_segment_meshes`] for a segment whose animations, light maps and hot spots read the
/// variables of `owner` (the leading unit: trailers share its scripts, `[scriptshare]`).
#[allow(clippy::too_many_arguments)]
fn load_segment_meshes_for(
    job: &mut WorldLoadJob,
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    images: &mut Assets<Image>,
    root: Entity,
    owner: Entity,
    lod0: &opts_core::formats::vehicles::model::ModelLod,
    model_dir: &Path,
    tex_dirs: &[PathBuf],
) -> (usize, usize, Vec3, Vec3) {
    let mut mesh_count = 0usize;
    let mut vertex_count = 0usize;
    let (mut min, mut max) = (Vec3::splat(f32::MAX), Vec3::splat(f32::MIN));
    // Dummy meshes (`…Dum_Gelenk_A`) that carry a bone's animation, and the skinned meshes
    // (`[smoothskin]` + `[setbone]`: the articulation bellows) bound to them.
    let mut dummies: Vec<(String, Entity)> = Vec::new();
    let mut skins: Vec<(
        Vec<Entity>,
        Vec<opts_core::formats::vehicles::model::MeshBone>,
    )> = Vec::new();
    for mesh_def in &lod0.meshes {
        // Shadow-only and AI-only meshes are not part of the visible player model.
        let vp = mesh_def.viewpoint;
        if mesh_def.isshadow || !(vp.is_outside() || vp.is_inside()) {
            continue;
        }
        // Developer aid: `OPTS_HIDE_MESH=<substring>[,<substring>]` skips matching vehicle meshes
        // (used to bisect rendering problems).
        if std::env::var("OPTS_HIDE_MESH").is_ok_and(|v| {
            v.split(',')
                .filter(|t| !t.is_empty())
                .any(|t| mesh_def.path.to_lowercase().contains(&t.to_lowercase()))
        }) {
            continue;
        }
        // A dummy mesh only carries the animation of a bone. Its geometry is often protected
        // (scrambled) or empty, so the joint is an entity of its own.
        let stem = Path::new(&mesh_def.path.replace('\\', "/"))
            .file_stem()
            .map(|s| s.to_string_lossy().to_lowercase())
            .unwrap_or_default();
        if stem.contains("dum") && mesh_def.anims.iter().any(|a| a.driver_variable().is_some()) {
            let mut joint = commands.spawn((
                Name::new(format!("joint_{stem}")),
                Transform::IDENTITY,
                Visibility::Inherited,
            ));
            if let Some(anim) = opts_bevy::MeshAnimation::from_mesh(owner, mesh_def) {
                joint.insert(anim);
            }
            let id = joint.id();
            commands.entity(root).add_child(id);
            dummies.push((stem.clone(), id));
        }
        let Some(o3d_path) = job.install.find_in(model_dir, &mesh_def.path) else {
            tracing::error!(at = %opts_bevy::position_tag(), mesh = %mesh_def.path, "vehicle o3d not found");
            job.cache
                .stats
                .failures
                .push(format!("vehicle o3d not found: {}", mesh_def.path));
            continue;
        };
        let overrides: Vec<TextureOverride> = mesh_def
            .materials
            .iter()
            .map(|m| TextureOverride {
                slot: m.slot_index as usize,
                texture: m.texture.clone(),
                alpha: m.alpha_mode,
                hidden_by_variable: m.alpha_scale_var.is_some(),
                trans_map: m.trans_map.clone().filter(|t| !t.trim().is_empty()),
                image: None,
                light_map: m.light_map.clone(),
            })
            .collect();
        let mut loader = ContentLoader {
            install: &job.install,
            cache: &mut job.cache,
            meshes,
            materials,
            images,
        };
        let skinned = mesh_def.smoothskin && !mesh_def.bones.is_empty();
        let parts = if skinned {
            loader.o3d_skinned_parts(&o3d_path, tex_dirs, &overrides)
        } else {
            loader.o3d_parts(&o3d_path, tex_dirs, &overrides)
        };
        for part in &parts {
            if let Some(mesh) = meshes.get(&part.mesh) {
                if let Some(bevy::mesh::VertexAttributeValues::Float32x3(pos)) =
                    mesh.attribute(Mesh::ATTRIBUTE_POSITION)
                {
                    let (mut lo, mut hi) = (Vec3::splat(f32::MAX), Vec3::splat(f32::MIN));
                    for p in pos {
                        lo = lo.min(Vec3::from(*p));
                        hi = hi.max(Vec3::from(*p));
                    }
                    // Camera framing uses the vehicle body only: meshes that are huge or far
                    // from the origin (shadow volumes, sky/helper geometry) would skew it.
                    if (hi - lo).max_element() < 16.0 && ((hi + lo) * 0.5).abs().max_element() < 8.0
                    {
                        min = min.min(lo);
                        max = max.max(hi);
                    }
                }
            }
            vertex_count += part.vertices;
            mesh_count += 1;
        }
        let spawned = attach_model_parts(commands, root, &parts, &mesh_def.path);
        // Script variables move and show the mesh (`[newanim]`, `[visible]`) and switch its light
        // maps; a click on it raises its `[mouseevent]` trigger.
        let anim = opts_bevy::MeshAnimation::from_mesh(owner, mesh_def);
        let hot_spot = mesh_def
            .mouseevent
            .as_deref()
            .map(str::trim)
            .filter(|t| !t.is_empty());
        if skinned {
            skins.push((spawned.clone(), mesh_def.bones.clone()));
        }
        for (e, part) in spawned.iter().copied().zip(&parts) {
            let mut entity = commands.entity(e);
            if let Some(anim) = &anim {
                entity.insert(anim.clone());
            }
            if let Some(var) = &part.light_variable {
                entity.insert(opts_bevy::LightMapBinding::new(owner, var.clone()));
            }
            if let Some(trigger) = hot_spot {
                entity.insert(opts_bevy::MouseTrigger {
                    owner,
                    trigger: trigger.to_owned(),
                });
            }
        }
    }
    bind_skins(commands, root, &dummies, skins);
    (mesh_count, vertex_count, min, max)
}

/// Binds skinned meshes of a vehicle to the dummy meshes that carry their bones' animation.
///
/// OMSI names a bone in `[setbone] <name> <index>`; the dummy mesh of that bone is the one whose
/// file name contains the bone name (`GN_Dum_Gelenk_A.o3d` for `Gelenk_A`). The bone matrix is
/// the dummy's animation matrix (the vertices are in vehicle space), so the inverse bind poses
/// are the identity. A bone without a dummy stays on the vehicle root (not animated).
fn bind_skins(
    commands: &mut Commands,
    root: Entity,
    dummies: &[(String, Entity)],
    skins: Vec<(
        Vec<Entity>,
        Vec<opts_core::formats::vehicles::model::MeshBone>,
    )>,
) {
    for (entities, bones) in skins {
        let n = bones
            .iter()
            .map(|b| usize::try_from(b.index).map_or(0, |i| i + 1))
            .max()
            .unwrap_or(0);
        if n == 0 {
            continue;
        }
        let mut joints = vec![root; n];
        for b in &bones {
            let Ok(i) = usize::try_from(b.index) else {
                continue;
            };
            let name = b.name.to_lowercase();
            match dummies.iter().find(|(stem, _)| stem.contains(&name)) {
                Some((_, e)) => joints[i] = *e,
                None => {
                    tracing::warn!(bone = %b.name, "no dummy mesh for bone; it stays on the root")
                }
            }
        }
        commands.queue(move |world: &mut World| {
            let handle = world
                .resource_mut::<Assets<bevy::mesh::skinning::SkinnedMeshInverseBindposes>>()
                .add(bevy::mesh::skinning::SkinnedMeshInverseBindposes::from(
                    vec![Mat4::IDENTITY; joints.len()],
                ));
            for e in entities {
                if let Ok(mut entity) = world.get_entity_mut(e) {
                    entity.insert((
                        bevy::mesh::skinning::SkinnedMesh {
                            inverse_bindposes: handle.clone(),
                            joints: joints.clone(),
                        },
                        bevy::camera::visibility::NoFrustumCulling,
                    ));
                }
            }
        });
    }
}

/// Spawns the trailers named by `[couple_back]` (recursively) behind `root`. Returns the
/// passenger camera presets of all trailers with the segment entity they are defined in.
#[allow(clippy::too_many_arguments)]
pub(super) fn spawn_trailer_chain(
    job: &mut WorldLoadJob,
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    images: &mut Assets<Image>,
    root: Entity,
    bus_dir: &Path,
    tractor: &BusManifest,
) -> Vec<(Entity, opts_core::formats::vehicles::bus::BusCamera)> {
    const MAX_SEGMENTS: usize = 4;
    let mut cams = Vec::new();
    let mut chain = Vec::new();
    let (mut parent_dir, mut parent_manifest) = (bus_dir.to_path_buf(), tractor.clone());
    for _ in 0..MAX_SEGMENTS {
        let Some(name) = parent_manifest.couple_back.clone() else {
            break;
        };
        let Some(path) = job.install.find_in(&parent_dir, &name) else {
            tracing::error!(at = %opts_bevy::position_tag(), trailer = %name, "trailer .bus not found; vehicle stays without it");
            break;
        };
        let loaded = (|| -> Result<_, String> {
            let manifest =
                BusManifest::parse_bytes(&std::fs::read(&path).map_err(|e| e.to_string())?)
                    .map_err(|e| e.to_string())?;
            let dir = path.parent().map(Path::to_path_buf).unwrap_or_default();
            let cfg_rel = manifest.model_cfg.clone().ok_or("no [model] entry")?;
            let cfg = job
                .install
                .find_in(&dir, &cfg_rel)
                .ok_or_else(|| format!("model config not found: {cfg_rel}"))?;
            let model = ModelConfig::parse_bytes(&std::fs::read(&cfg).map_err(|e| e.to_string())?)
                .map_err(|e| e.to_string())?;
            Ok((manifest, dir, cfg, model))
        })();
        let (manifest, dir, cfg, model) = match loaded {
            Ok(v) => v,
            Err(e) => {
                tracing::error!(at = %opts_bevy::position_tag(), trailer = %path.display(), "cannot load trailer: {e}");
                break;
            }
        };
        let Some(lod0) = model.lods.first() else {
            break;
        };
        // Both coupling points are in their own vehicle frame; the segment follows its leader
        // with its front joint on the leader's rear joint (`opts_bevy::articulation`).
        let (Some(b), Some(f)) = (parent_manifest.coupling_back, manifest.coupling_front) else {
            tracing::error!(at = %opts_bevy::position_tag(), trailer = %path.display(), "coupling points missing; trailer not attached");
            break;
        };
        let rear_axle_y = manifest
            .axles
            .iter()
            .map(|a| a.longitudinal_m)
            .fold(f32::INFINITY, f32::min);
        let rear_axle_y = if rear_axle_y.is_finite() {
            rear_axle_y
        } else {
            f.y - 6.0
        };
        let max_alpha = manifest
            .coupling_front_limits
            .map_or(52.5, |l| l.max_alpha_deg);
        let seg = commands
            .spawn((
                Name::new(name.clone()),
                Transform::default(),
                Visibility::default(),
                opts_bevy::ArticulatedSegment::new(
                    Vec2::new(b.x, b.y),
                    Vec2::new(f.x, f.y),
                    rear_axle_y,
                    max_alpha,
                ),
                InGameEntity,
            ))
            .id();
        chain.push(seg);
        let model_dir = cfg.parent().map(Path::to_path_buf).unwrap_or_default();
        let tex_dirs = vec![
            job.install
                .find_in(&dir, "texture")
                .unwrap_or_else(|| dir.join("texture")),
            model_dir.join("texture"),
        ];
        job.cache.expected_bounds = manifest_bounds(&manifest);
        job.cache.texture_replacements = repaint_map(job, &model);
        let (count, verts, _, _) = load_segment_meshes_for(
            job, commands, meshes, materials, images, seg, root, lod0, &model_dir, &tex_dirs,
        );
        job.cache.expected_bounds = None;
        job.cache.texture_replacements.clear();
        tracing::info!(trailer = %name, meshes = count, vertices = verts, "trailer segment attached");
        cams.extend(manifest.pax_cameras.iter().cloned().map(|c| (seg, c)));
        (parent_dir, parent_manifest) = (dir, manifest);
    }
    if !chain.is_empty() {
        commands
            .entity(root)
            .insert(opts_bevy::ArticulatedChain { segments: chain });
    }
    cams
}

/// Queues the background decoding of the vehicle's scrambled meshes and returns their keys.
///
/// The vehicle must not be built before they are done (it would come up without a body), so the
/// loading stage waits until none of the returned keys is pending, then calls
/// [`load_player_vehicle`]. The scratch meshes built here are dropped.
pub fn prewarm_vehicle_meshes(
    job: &mut WorldLoadJob,
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    images: &mut Assets<Image>,
) -> Vec<opts_bevy::descramble_cache::Key> {
    let Some(bus_path) = job
        .config
        .vehicle_path
        .clone()
        .or_else(|| first_ai_list_vehicle(job))
    else {
        return Vec::new();
    };
    let loaded = (|| -> Option<(PathBuf, PathBuf, ModelConfig, Option<_>, BusManifest)> {
        let manifest = BusManifest::parse_bytes(&std::fs::read(&bus_path).ok()?).ok()?;
        let bus_dir = bus_path.parent()?.to_path_buf();
        let cfg = job
            .install
            .find_in(&bus_dir, manifest.model_cfg.as_deref()?)?;
        let model = ModelConfig::parse_bytes(&std::fs::read(&cfg).ok()?).ok()?;
        let bounds = manifest_bounds(&manifest);
        Some((bus_dir, cfg, model, bounds, manifest))
    })();
    let Some((bus_dir, cfg, model, bounds, manifest)) = loaded else {
        return Vec::new();
    };
    let Some(lod0) = model.lods.first() else {
        return Vec::new();
    };
    let model_dir = cfg.parent().map(Path::to_path_buf).unwrap_or_default();
    let tex_dirs = vec![
        job.install
            .find_in(&bus_dir, "texture")
            .unwrap_or_else(|| bus_dir.join("texture")),
        model_dir.join("texture"),
    ];
    job.cache.take_deferred();
    let scratch = commands.spawn_empty().id();
    ensure_repaints(job, &bus_path, &manifest);
    job.cache.expected_bounds = bounds;
    job.cache.texture_replacements = repaint_map(job, &model);
    load_segment_meshes(
        job, commands, meshes, materials, images, scratch, lod0, &model_dir, &tex_dirs,
    );
    job.cache.expected_bounds = None;
    job.cache.texture_replacements.clear();
    commands.entity(scratch).despawn();
    job.cache.take_deferred().into_iter().collect()
}

/// Number of the given decode keys still being worked on.
#[must_use]
pub fn decodes_pending(job: &WorldLoadJob, keys: &[opts_bevy::descramble_cache::Key]) -> usize {
    job.cache
        .decode_pool
        .as_ref()
        .map_or(0, |p| keys.iter().filter(|k| p.is_pending(**k)).count())
}

/// Loads the player's vehicle from its `.bus` file and spawns it at the map's start point.
pub fn load_player_vehicle(
    job: &mut WorldLoadJob,
    stats: &mut WorldStats,
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    images: &mut Assets<Image>,
    bindposes: &mut Assets<bevy::mesh::skinning::SkinnedMeshInverseBindposes>,
) -> Result<(), String> {
    let bus_path = match job.config.vehicle_path.clone() {
        Some(p) => p,
        None => first_ai_list_vehicle(job)
            .ok_or("no --vehicle-path given and the map's ailists.cfg lists no vehicle")?,
    };
    let bus_bytes = std::fs::read(&bus_path).map_err(|e| format!("{}: {e}", bus_path.display()))?;
    let manifest =
        BusManifest::parse_bytes(&bus_bytes).map_err(|e| format!("{}: {e}", bus_path.display()))?;
    tracing::info!(path = %bus_path.display(), mass_t = ?manifest.mass_tonnes, axles = manifest.axles.len(), "vehicle .bus parsed");

    let bus_dir = bus_path.parent().map(Path::to_path_buf).unwrap_or_default();
    let cfg_rel = manifest
        .model_cfg
        .clone()
        .ok_or("the .bus file has no [model] entry")?;
    let cfg_path = job
        .install
        .find_in(&bus_dir, &cfg_rel)
        .ok_or_else(|| format!("model config not found: {cfg_rel}"))?;
    let cfg_bytes = std::fs::read(&cfg_path).map_err(|e| format!("{}: {e}", cfg_path.display()))?;
    let model =
        ModelConfig::parse_bytes(&cfg_bytes).map_err(|e| format!("{}: {e}", cfg_path.display()))?;
    let lod0 = model.lods.first().ok_or("model config has no meshes")?;
    tracing::info!(path = %cfg_path.display(), lods = model.lods.len(), meshes = lod0.meshes.len(), "vehicle model config parsed");

    let model_dir = cfg_path.parent().map(Path::to_path_buf).unwrap_or_default();
    let tex_dirs = vec![
        job.install
            .find_in(&bus_dir, "texture")
            .unwrap_or_else(|| bus_dir.join("texture")),
        model_dir.join("texture"),
    ];

    // Spawn point: the start road spline's lane position, converted to Bevy space.
    let (spawn_world, heading_deg) = job.spawn.ok_or("no spawn point")?;
    let bevy_pos = omsi_to_bevy_pos(spawn_world);
    let root_tf = Transform {
        translation: bevy_pos,
        rotation: omsi_euler_to_bevy_quat(-heading_deg, 0.0, 0.0),
        scale: Vec3::ONE,
    };

    job.vehicle_dirs = std::iter::once(bus_dir.clone())
        .chain(std::iter::once(model_dir.clone()))
        .chain(tex_dirs.iter().cloned())
        .map(|d| opts_bevy::hot_reload::cache::normalize(&d))
        .collect();
    let root = spawn_vehicle_model(commands, &manifest, &model, root_tf, &[], &[]);

    ensure_repaints(job, &bus_path, &manifest);
    job.cache.expected_bounds = manifest_bounds(&manifest);
    job.cache.texture_replacements = repaint_map(job, &model);
    let (mesh_count, vertex_count, min, max) = load_segment_meshes(
        job, commands, meshes, materials, images, root, lod0, &model_dir, &tex_dirs,
    );
    job.cache.expected_bounds = None;
    job.cache.texture_replacements.clear();
    if mesh_count == 0 {
        return Err("no vehicle meshes could be loaded".into());
    }

    let mut controller = VehicleController::from_manifest(&manifest);
    controller.sim.chassis.position = Vec2::new(spawn_world.x, spawn_world.y);
    controller.sim.chassis.elevation_z = spawn_world.z;
    // The simulation uses compass-style heading (clockwise from +Y), like OMSI map headings.
    controller.sim.chassis.heading_rad = heading_deg.to_radians();
    commands
        .entity(root)
        .insert((controller, PlayerVehicle, InGameEntity));
    attach_scripts(job, commands, root, &manifest, &bus_dir);
    load_key_bindings(job, commands);
    let trailer_cams = spawn_trailer_chain(
        job, commands, meshes, materials, images, root, &bus_dir, &manifest,
    );
    if !trailer_cams.is_empty() {
        commands.queue(move |world: &mut World| {
            if let Some(mut bus) = world.get_mut::<opts_bevy::BusVehicle>(root) {
                bus.trailer_pax_cameras = trailer_cams;
            }
        });
    }
    match load_driver_human(
        job,
        commands,
        meshes,
        materials,
        images,
        bindposes,
        root,
        &bus_dir,
        manifest.passengercabin_cfg.as_deref(),
    ) {
        Ok(name) => tracing::info!(human = %name, "player character seated at the driver position"),
        Err(e) => {
            tracing::error!(at = %opts_bevy::position_tag(), "cannot load the player character: {e}")
        }
    }

    tracing::debug!(min = ?min, max = ?max, "vehicle bounding box (Bevy space)");
    let length = (max.z - min.z).abs().max(1.0);
    let height = (max.y - min.y).abs().max(1.0);
    let driver_cam = manifest
        .driver_cameras
        .get(manifest.std_camera_index.unwrap_or(0))
        .or_else(|| manifest.driver_cameras.first());
    // Driver camera preset converted by the shared helper (vehicle space → Bevy).
    let cockpit_offset = driver_cam
        .map(|c| {
            camera_preset_to_bevy_transform(
                Vec3::new(c.x, c.y, c.z),
                c.dist,
                c.yaw_deg,
                c.pitch_deg,
            )
            .translation
        })
        .unwrap_or(Vec3::new(0.0, height * 0.8, 0.0));
    let chase_distance = length * 1.3;
    let chase_height = height * 1.6;
    job.camera_std_index = manifest.std_camera_index.unwrap_or(0);
    job.camera = Some(CameraSetup {
        start: Transform::from_translation(
            bevy_pos + root_tf.rotation * Vec3::new(0.0, chase_height, chase_distance),
        )
        .looking_at(bevy_pos + Vec3::Y * (height * 0.5), Vec3::Y),
        chase_distance,
        chase_height,
        cockpit_offset,
    });

    stats.vehicle_name = manifest
        .friendly_name
        .as_ref()
        .map(|f| format!("{} {}", f.manufacturer, f.model))
        .unwrap_or_else(|| {
            bus_path
                .file_stem()
                .unwrap_or_default()
                .to_string_lossy()
                .into_owned()
        });
    stats.vehicle_meshes = mesh_count;
    stats.vehicle_vertices = vertex_count;
    stats.scene_vertices += vertex_count;
    stats.asset_failures = job.cache.stats.failures.len();
    tracing::info!(
        vehicle = %stats.vehicle_name,
        meshes = mesh_count,
        vertices = vertex_count,
        "vehicle meshes loaded"
    );
    Ok(())
}

/// Loads the skinned meshes of a character: the first (highest-detail) LOD of its model manifest.
///
/// Fails with "no human meshes could be loaded" while scrambled meshes are still being decoded in
/// the background; callers may retry later.
pub(super) fn load_human_parts(
    job: &mut WorldLoadJob,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    images: &mut Assets<Image>,
    profile: &opts_core::formats::humans::HumanProfile,
    hum_dir: &Path,
) -> Result<Vec<opts_bevy::ModelPart>, String> {
    // Model manifest and skinned meshes (highest-detail LOD).
    let cfg_path = job
        .install
        .find_in(hum_dir, &profile.model)
        .ok_or_else(|| format!("human model config not found: {}", profile.model))?;
    let cfg = opts_core::formats::vehicles::model::ModelConfig::parse_bytes(
        &std::fs::read(&cfg_path).map_err(|e| format!("{}: {e}", cfg_path.display()))?,
    )
    .map_err(|e| format!("{}: {e}", cfg_path.display()))?;
    let model_dir = cfg_path.parent().map(Path::to_path_buf).unwrap_or_default();
    let tex_dirs = vec![
        model_dir.clone(),
        model_dir.join("texture"),
        hum_dir.join("texture"),
        model_dir
            .parent()
            .map(|p| p.join("Texture"))
            .unwrap_or_default(),
    ];
    let lod = cfg.lods.first().ok_or("human model has no meshes")?;
    let mut parts = Vec::new();
    for mesh_def in &lod.meshes {
        let Some(o3d_path) = job.install.find_in(&model_dir, &mesh_def.path) else {
            tracing::error!(at = %opts_bevy::position_tag(), mesh = %mesh_def.path, "human mesh not found");
            continue;
        };
        let overrides: Vec<TextureOverride> = mesh_def
            .materials
            .iter()
            .map(|m| TextureOverride {
                slot: m.slot_index as usize,
                texture: m.texture.clone(),
                alpha: m.alpha_mode,
                hidden_by_variable: m.alpha_scale_var.is_some(),
                trans_map: m.trans_map.clone().filter(|t| !t.trim().is_empty()),
                image: None,
                light_map: None,
            })
            .collect();
        let mut loader = ContentLoader {
            install: &job.install,
            cache: &mut job.cache,
            meshes,
            materials,
            images,
        };
        parts.extend(loader.o3d_skinned_parts(&o3d_path, &tex_dirs, &overrides));
    }
    if parts.is_empty() {
        return Err("no human meshes could be loaded".into());
    }
    Ok(parts)
}

/// Loads the player's character and seats it at the vehicle's `[drivpos]`.
///
/// The skinned mesh, joint pivots and head anchor come from the `.hum`, its model manifest and
/// the `.o3d` bone weights; the seat contact point comes from the vehicle's passenger cabin.
#[allow(clippy::too_many_arguments)]
pub(super) fn load_driver_human(
    job: &mut WorldLoadJob,
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    images: &mut Assets<Image>,
    bindposes: &mut Assets<bevy::mesh::skinning::SkinnedMeshInverseBindposes>,
    vehicle_root: Entity,
    bus_dir: &Path,
    cabin_rel: Option<&str>,
) -> Result<String, String> {
    use opts_bevy::{hip_offset, spawn_human, HumanPose};
    use opts_core::formats::humans::HumanProfile;
    use opts_core::formats::text::{decode_bytes, scan_blocks};

    let hum_path = job
        .config
        .human_path
        .clone()
        .ok_or("no .hum profile in the installation")?;
    let profile = HumanProfile::parse(
        &std::fs::read(&hum_path).map_err(|e| format!("{}: {e}", hum_path.display()))?,
    )
    .map_err(|e| format!("{}: {e}", hum_path.display()))?;
    let hum_dir = hum_path.parent().map(Path::to_path_buf).unwrap_or_default();

    // Driver seat: `[drivpos]` x, y, z, h, rot in the vehicle's passenger cabin.
    let cabin = cabin_rel
        .and_then(|c| job.install.find_in(bus_dir, c))
        .ok_or("vehicle has no passenger cabin file")?;
    let text =
        decode_bytes(&std::fs::read(&cabin).map_err(|e| format!("{}: {e}", cabin.display()))?);
    let blocks = scan_blocks(&text);
    let seat: Vec<f32> = blocks
        .iter()
        .find(|b| b.keyword == "drivpos")
        .ok_or("passenger cabin has no [drivpos]")?
        .parameters
        .iter()
        .take(5)
        .map(|p| {
            p.trim()
                .replace(',', ".")
                .parse::<f32>()
                .map_err(|_| format!("bad [drivpos] value '{p}'"))
        })
        .collect::<Result<_, _>>()?;
    if seat.len() < 5 {
        return Err("[drivpos] needs 5 values".into());
    }

    let parts = load_human_parts(job, meshes, materials, images, &profile, &hum_dir)?;

    tracing::info!(parts = parts.len(), vertices = parts.iter().map(|p| p.vertices).sum::<usize>(), seat = ?seat, "human meshes loaded");
    let name = hum_path
        .file_stem()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_default();
    let human = spawn_human(
        commands,
        bindposes,
        &profile,
        &parts,
        &name,
        HumanPose::Seated,
    );
    // Hips on the seat contact point, facing along `rot` (clockwise like headings).
    let contact = omsi_to_bevy_pos(Vec3::new(seat[0], seat[1], seat[2]));
    commands.entity(human).insert(Transform {
        translation: contact - hip_offset(&profile),
        rotation: omsi_euler_to_bevy_quat(-seat[4], 0.0, 0.0),
        scale: Vec3::ONE,
    });
    commands.entity(vehicle_root).add_child(human);
    Ok(name)
}

/// Loads the vehicle's `.osc` scripts, variable lists and constants into a script engine.
/// A vehicle with `[scriptshare]` has none of its own and gets no engine.
fn attach_scripts(
    job: &mut WorldLoadJob,
    commands: &mut Commands,
    root: Entity,
    manifest: &BusManifest,
    bus_dir: &Path,
) {
    if manifest.scripts.is_empty() {
        return;
    }
    let (engine, diags) = opts_sim::script::load_vehicle_scripts(manifest, |rel| {
        let found = job.install.find_in(bus_dir, rel);
        if let Some(p) = &found {
            job.vehicle_dirs
                .push(opts_bevy::hot_reload::cache::normalize(p));
        }
        found
    });
    let errors = diags
        .iter()
        .filter(|d| d.severity == opts_sim::script::Severity::Error)
        .count();
    for d in diags.iter().filter(|d| d.message == "file not found") {
        tracing::error!(at = %opts_bevy::position_tag(), file = %d.file, "vehicle script file not found");
    }
    tracing::info!(
        scripts = manifest.scripts.len(),
        diagnostics = diags.len(),
        errors,
        "vehicle scripts loaded"
    );
    let mut engine = engine;
    // Equipment options of the chosen repaint (`[setvar]` of its `.cti` items).
    if let Some(r) = &job.repaint {
        let name = if job.config.livery.is_empty() {
            r.default_name.as_str()
        } else {
            job.config.livery.as_str()
        };
        for (var, value) in r.setvars.get(name).into_iter().flatten() {
            match value.trim().replace(',', ".").parse::<f32>() {
                Ok(v) => engine.set(var, v),
                Err(_) => tracing::warn!(var, value, "repaint [setvar] value is not a number"),
            }
        }
    }
    commands
        .entity(root)
        .insert(opts_bevy::VehicleScripts::new(engine));
}

/// Loads the installation's `Inputs/keyboard.cfg` into [`opts_bevy::ScriptKeyBindings`], so the
/// player's keys raise the vehicle's script triggers. Without the file no key triggers exist.
fn load_key_bindings(job: &WorldLoadJob, commands: &mut Commands) {
    let Some(path) = job.install.find_file("Inputs/keyboard.cfg") else {
        tracing::warn!("Inputs/keyboard.cfg not found; vehicle script triggers have no keys");
        return;
    };
    let config = match std::fs::read(&path) {
        Ok(bytes) => opts_core::formats::engine_config::KeyboardConfig::parse(&bytes),
        Err(e) => {
            tracing::error!(at = %opts_bevy::position_tag(), path = %path.display(), "cannot read keyboard.cfg: {e}");
            return;
        }
    };
    let bindings = opts_bevy::ScriptKeyBindings::from_config(&config);
    tracing::info!(
        entries = config.vehicle_bindings().count(),
        bound = bindings.0.len(),
        "vehicle script key bindings loaded"
    );
    commands.insert_resource(bindings);
}
