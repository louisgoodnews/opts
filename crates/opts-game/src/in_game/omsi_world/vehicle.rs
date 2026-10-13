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
    // Dynamic text textures of this segment: one image per `[texttexture]` index, redrawn when the
    // script string variable it shows changes (`vehicle_text.rs`).
    let text_defs = job.vehicle_text_defs.clone();
    let mut text_images: std::collections::HashMap<usize, Handle<Image>> = Default::default();
    // Dummy meshes (`…Dum_Gelenk_A`) that carry a bone's animation, and the skinned meshes
    // (`[smoothskin]` + `[setbone]`: the articulation bellows) bound to them.
    let mut dummies: Vec<(String, Entity)> = Vec::new();
    let mut skins: Vec<SkinBinding> = Vec::new();
    // Runs of `bone_<letter>` dummies: a skinned mesh numbers its bones (`[setbone] name n`)
    // along the run of dummies declared before it (the NF6D-style bellows of the Heuliez GX).
    // (shared stem without the last letter, dummies in declaration order); the run touched last
    // belongs to the skinned mesh that follows.
    let mut bone_runs: Vec<(String, Vec<Entity>)> = Vec::new();
    let mut last_run: Option<usize> = None;
    // Names of the bones the skinned meshes of this model list (`[setbone]`): a mesh with that
    // file name is the dummy that carries the bone's animation (`Gelenk_A.o3d` for `Gelenk_A`).
    let bone_names: std::collections::HashSet<String> = lod0
        .meshes
        .iter()
        .flat_map(|m| m.bones.iter().map(|b| b.name.to_lowercase()))
        .collect();
    let sibling_run = articulation_runs(&lod0.meshes);
    for (mesh_index, mesh_def) in lod0.meshes.iter().enumerate() {
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
        // Numbered siblings driven by `articulation_*` (`Dum_A`…`Dum_D`, `Gelenk_Dummy_A`…,
        // `przd_a`…) are the bones' dummies even when their names say nothing about bones.
        let is_bone_dummy = is_bone_stem(&stem) || sibling_run[mesh_index];
        let carries_animation = mesh_def.anims.iter().any(|a| a.driver_variable().is_some());
        if (stem.contains("dum") || is_bone_dummy || bone_names.contains(&stem))
            && carries_animation
        {
            let mut joint = commands.spawn((
                Name::new(format!("joint_{stem}")),
                Transform::IDENTITY,
                Visibility::Inherited,
            ));
            if let Some(anim) = opts_bevy::MeshAnimation::from_mesh(
                owner,
                mesh_def,
                mesh_pivot(job, &mesh_def.path, model_dir, mesh_def),
            ) {
                joint.insert(anim);
            }
            let id = joint.id();
            commands.entity(root).add_child(id);
            dummies.push((stem.clone(), id));
            if is_bone_dummy {
                let mut chars = stem.chars();
                chars.next_back();
                let prefix = chars.as_str().to_owned();
                let at = match bone_runs.iter().position(|(k, _)| *k == prefix) {
                    Some(at) => at,
                    None => {
                        bone_runs.push((prefix, Vec::new()));
                        bone_runs.len() - 1
                    }
                };
                bone_runs[at].1.push(id);
                last_run = Some(at);
            }
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
                image: m.text_texture_index.and_then(|i| {
                    if let Some(h) = text_images.get(&i) {
                        return Some(h.clone());
                    }
                    let Some(def) = text_defs.get(i) else {
                        tracing::error!(at = %opts_bevy::position_tag(), mesh = %mesh_def.path, index = i, "[useTextTexture] names a text texture the model does not declare");
                        return None;
                    };
                    let (w, h) = (def.width.max(1), def.height.max(1));
                    let handle = images.add(opts_bevy::text_texture::text_image(
                        w,
                        h,
                        vec![0; (w * h * 4) as usize],
                    ));
                    commands.spawn((
                        super::vehicle_text::VehicleTextTexture::new(owner, def, handle.clone()),
                        Name::new(format!("text_texture_{i}_{}", def.variable)),
                        ChildOf(root),
                    ));
                    text_images.insert(i, handle.clone());
                    Some(handle)
                }),
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
            let slots: Vec<(String, u16)> = mesh_def
                .bones
                .iter()
                .filter_map(|b| u16::try_from(b.index).ok().map(|i| (b.name.clone(), i)))
                .collect();
            loader.o3d_skinned_parts_slots(&o3d_path, tex_dirs, &overrides, &slots)
        } else {
            loader.o3d_parts(&o3d_path, tex_dirs, &overrides)
        };
        for part in &parts {
            job.cache.stats.uv_scope.0 += part.vertices;
            job.cache.stats.uv_scope.1 += part.uv_unreliable;
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
        let anim = opts_bevy::MeshAnimation::from_mesh(
            owner,
            mesh_def,
            mesh_pivot(job, &mesh_def.path, model_dir, mesh_def),
        );
        let hot_spot = mesh_def
            .mouseevent
            .as_deref()
            .map(str::trim)
            .filter(|t| !t.is_empty());
        if skinned {
            skins.push(SkinBinding {
                entities: spawned.clone(),
                bones: mesh_def.bones.clone(),
                run: {
                    // Most recently touched run first; the skinned mesh takes the first one long
                    // enough for its bone numbers (damper pairs `…_l`/`…_r` form short runs).
                    let need = mesh_def
                        .bones
                        .iter()
                        .filter_map(|b| usize::try_from(b.index).ok())
                        .max()
                        .map_or(0, |m| m + 1);
                    let mut order: Vec<usize> = (0..bone_runs.len()).rev().collect();
                    if let Some(at) = last_run {
                        order.retain(|i| *i != at);
                        order.insert(0, at);
                    }
                    order
                        .into_iter()
                        .map(|i| &bone_runs[i].1)
                        .find(|r| r.len() >= need)
                        .or_else(|| last_run.and_then(|at| bone_runs.get(at)).map(|r| &r.1))
                        .cloned()
                        .unwrap_or_default()
                },
            });
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

/// A skinned mesh waiting for its bones: its entities, its `[setbone]` list and the run of
/// `bone_<letter>` dummies declared before it.
struct SkinBinding {
    entities: Vec<Entity>,
    bones: Vec<opts_core::formats::vehicles::model::MeshBone>,
    run: Vec<Entity>,
}

/// Marks the meshes that belong to a run of numbered dummies: two or more meshes of the model whose
/// file stems differ only in their last letter or digit and whose animations are driven by
/// `articulation_*` variables (the bones of a bellows in the order of their `[setbone]` index).
fn articulation_runs(meshes: &[opts_core::formats::vehicles::model::ModelMesh]) -> Vec<bool> {
    let key = |m: &opts_core::formats::vehicles::model::ModelMesh| -> Option<String> {
        let driven = m.anims.iter().any(|a| {
            a.driver_variable()
                .is_some_and(|v| v.to_ascii_lowercase().starts_with("articulation_"))
        });
        let stem = Path::new(&m.path.replace('\\', "/"))
            .file_stem()?
            .to_string_lossy()
            .to_lowercase();
        let mut chars = stem.chars();
        let last = chars.next_back()?;
        (driven && last.is_ascii_alphanumeric() && stem.chars().count() >= 3)
            .then(|| chars.as_str().to_owned())
    };
    let keys: Vec<Option<String>> = meshes.iter().map(key).collect();
    let mut count: std::collections::HashMap<&str, usize> = std::collections::HashMap::new();
    for k in keys.iter().flatten() {
        *count.entry(k.as_str()).or_default() += 1;
    }
    keys.iter()
        .map(|k| k.as_deref().is_some_and(|k| count[k] >= 2))
        .collect()
}

/// Whether a mesh file stem names a bone dummy of a bellows (`bone_a` … `bone_f`).
fn is_bone_stem(stem: &str) -> bool {
    // `bone_a` … `bone_d` (Heuliez), `dummya` … `dummyd` (MAN New Lion's City), `dummy_a`.
    ["bone_", "dummy_", "dummy"].iter().any(|prefix| {
        stem.strip_prefix(prefix)
            .is_some_and(|rest| rest.len() == 1 && rest.chars().all(|c| c.is_ascii_alphabetic()))
    })
}

/// The mesh's own origin in OMSI vehicle axes, for animations with `origin_from_mesh`: the
/// translation row of the `.o3d` matrix chunk (o3d axes x lateral, y up, z longitudinal).
fn mesh_pivot(
    job: &WorldLoadJob,
    rel: &str,
    model_dir: &Path,
    mesh_def: &opts_core::formats::vehicles::model::ModelMesh,
) -> Option<opts_bevy::MeshPivot> {
    if !mesh_def.anims.iter().any(|a| a.origin_from_mesh) {
        return None;
    }
    let path = job.install.find_in(model_dir, rel)?;
    let o3d = opts_core::formats::o3d::parse(&std::fs::read(path).ok()?).ok()?;
    let m = o3d.transform?;
    // Rotation part of the matrix: normalised rows (images of the local axes, o3d coordinates).
    let row = |i: usize| {
        let v = Vec3::new(m[i][0], m[i][1], m[i][2]);
        let v = v.try_normalize().unwrap_or(Vec3::ZERO);
        [v.x, v.y, v.z]
    };
    Some(opts_bevy::MeshPivot {
        origin: Vec3::new(m[3][0], m[3][2], m[3][1]),
        rows: [row(0), row(1), row(2)],
    })
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
    skins: Vec<SkinBinding>,
) {
    for SkinBinding {
        entities,
        bones,
        run,
    } in skins
    {
        let run_stems: Vec<(&String, Entity)> = dummies
            .iter()
            .filter(|(_, e)| run.contains(e))
            .map(|(stem, e)| (stem, *e))
            .collect();
        let n = bones
            .iter()
            .map(|b| usize::try_from(b.index).map_or(0, |i| i + 1))
            .max()
            .unwrap_or(0);
        if n == 0 {
            continue;
        }
        // One extra joint after the highest: bones of the file the model does not list (the
        // base bone of a bellows) stay on the vehicle.
        let mut joints = vec![root; n + 1];
        for b in &bones {
            let Ok(i) = usize::try_from(b.index) else {
                continue;
            };
            let name = b.name.to_lowercase();
            // By name (`Gelenk_A.o3d` for `Gelenk_A`), else the dummy of the same trailing letter
            // or digit (`Bone_C` ↔ `Dum_1_C`), else the dummy at the bone's number in the run.
            let suffix = name
                .rsplit(['_', ' '])
                .next()
                .filter(|t| t.chars().count() == 1);
            let lettered = suffix.and_then(|t| {
                run_stems
                    .iter()
                    .find(|(stem, _)| stem.ends_with(t))
                    .map(|(_, e)| *e)
            });
            match dummies.iter().find(|(stem, _)| stem.contains(&name)) {
                Some((_, e)) => joints[i] = *e,
                None => match lettered.or_else(|| run.get(i).copied()) {
                    Some(e) => joints[i] = e,
                    None => {
                        tracing::warn!(bone = %b.name, index = b.index, run = run.len(), dummies = ?dummies.iter().map(|d| d.0.as_str()).collect::<Vec<_>>(), "no dummy mesh for bone; it stays on the root");
                    }
                },
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
    const MAX_SEGMENTS: usize = 40;
    let mut cams = Vec::new();
    let mut chain = Vec::new();
    let mut layout = Vec::new();
    let mut behind = 0.0_f32;
    // Axles counted so far: trailer models number theirs after the head's (`Wheel_Rotation_2_L`).
    let mut axle_base = tractor.axles.len();
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
        let Some(detail) = model.detail_lod() else {
            break;
        };
        let lod0 = &detail;
        job.vehicle_text_defs = model.text_textures.clone();
        // Both coupling points are in their own vehicle frame; the segment follows its leader
        // with its front joint on the leader's rear joint (`opts_bevy::articulation`).
        let reversed = parent_manifest.couple_back_reversed;
        let own_joint = if reversed {
            manifest.coupling_back.map(|c| -c)
        } else {
            manifest.coupling_front
        };
        let (Some(b), Some(f)) = (parent_manifest.coupling_back, own_joint) else {
            tracing::error!(at = %opts_bevy::position_tag(), trailer = %path.display(), "coupling points missing; trailer not attached");
            break;
        };
        let rear_axle_y = if reversed {
            // Mirrored: the model's frontmost axle becomes the rearmost.
            -manifest
                .axles
                .iter()
                .map(|a| a.longitudinal_m)
                .fold(f32::NEG_INFINITY, f32::max)
        } else {
            manifest
                .axles
                .iter()
                .map(|a| a.longitudinal_m)
                .fold(f32::INFINITY, f32::min)
        };
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
                {
                    let mut s = opts_bevy::ArticulatedSegment::new(
                        Vec2::new(b.x, b.y),
                        Vec2::new(f.x, f.y),
                        rear_axle_y,
                        max_alpha,
                    );
                    s.reversed = reversed;
                    // The bus files list min and max in either order (3 files have 20,-20).
                    s.beta_limits_deg = manifest.coupling_front_limits.map(|l| {
                        (
                            l.min_beta_deg.min(l.max_beta_deg),
                            l.min_beta_deg.max(l.max_beta_deg),
                        )
                    });
                    s.set_wheels(
                        axle_base,
                        manifest
                            .axles
                            .iter()
                            .map(|a| a.wheel_diameter_m.filter(|&d| d > 0.1).unwrap_or(0.98) * 0.5),
                    );
                    s
                },
                InGameEntity,
            ))
            .id();
        chain.push(seg);
        axle_base += manifest.axles.len();
        // Origin of the segment behind the root's origin along the vehicle's axis (for rails).
        behind += f.y - b.y;
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
        layout.push(super::rail::Part {
            entity: seg,
            name: name.clone(),
            behind,
            half_span: manifest.bogie_half_span.unwrap_or(0.0),
            reversed,
            meshes: count,
            vertices: verts,
            body: manifest
                .bounding_box
                .map(|(size, centre)| super::BodyBox::from_bounding_box(size, centre)),
        });
        cams.extend(manifest.pax_cameras.iter().cloned().map(|c| (seg, c)));
        if reversed {
            // The chain of a reversed head leads back into this consist (the NF6D's B head
            // couples to the articulation joint again): the ring is closed here.
            break;
        }
        (parent_dir, parent_manifest) = (dir, manifest);
    }
    if !chain.is_empty() {
        if !super::is_rail(tractor) {
            commands
                .entity(root)
                .insert(opts_bevy::ArticulatedChain { segments: chain });
        }
        commands
            .entity(root)
            .insert((super::rail::VehicleComposition {
                head: super::rail::Part {
                    entity: root,
                    name: tractor
                        .friendly_name
                        .as_ref()
                        .map(|f| f.model.clone())
                        .unwrap_or_default(),
                    behind: 0.0,
                    half_span: tractor.bogie_half_span.unwrap_or(0.0),
                    reversed: false,
                    meshes: 0,
                    vertices: 0,
                    body: tractor
                        .bounding_box
                        .map(|(size, centre)| super::BodyBox::from_bounding_box(size, centre)),
                },
                followers: layout,
            },));
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
    let Some(detail) = model.detail_lod() else {
        return Vec::new();
    };
    let lod0 = &detail;
    job.vehicle_text_defs = model.text_textures.clone();
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

/// Mass of a rail consist for the dynamics (kg): the head unit's mass, at least 20 t (the other
/// units usually state 0; assumed, see `docs/src/simulation/rail-vehicles.md`).
fn commands_mass(manifest: &BusManifest) -> f32 {
    (manifest.mass_tonnes.unwrap_or(0.0) * 1000.0).max(20_000.0)
}

/// A vehicle [`load_player_vehicle`] built.
#[derive(Debug, Clone)]
pub struct SpawnedVehicle {
    /// The vehicle's root entity.
    pub root: Entity,
    /// Display name (manufacturer and model, else the `.bus` file's name).
    pub name: String,
    /// Camera parameters derived from the vehicle.
    pub camera: CameraSetup,
}

/// Loads a user vehicle from its `.bus` file and spawns it at the session's spawn point
/// ([`WorldLoadJob::spawn_point`]), as session vehicle `session_id`. With `activate` it also becomes the
/// player's vehicle (`PlayerVehicle`, the HUD and `WorldStats` follow it); without, it is built
/// as a parked session vehicle and the caller switches to it when it wants to.
#[allow(clippy::too_many_arguments)]
pub fn load_player_vehicle(
    job: &mut WorldLoadJob,
    stats: &mut WorldStats,
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    images: &mut Assets<Image>,
    bindposes: &mut Assets<bevy::mesh::skinning::SkinnedMeshInverseBindposes>,
    session_id: crate::session_vehicles::VehicleId,
    activate: bool,
) -> Result<SpawnedVehicle, String> {
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
    let detail = model.detail_lod().ok_or("model config has no meshes")?;
    let lod0 = &detail;
    job.vehicle_text_defs = model.text_textures.clone();
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
    job.cache.stats.uv_scope = (0, 0);
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
    commands.entity(root).insert((
        controller,
        crate::session_vehicles::SessionVehicle(session_id),
        InGameEntity,
    ));
    if activate {
        commands.entity(root).insert(PlayerVehicle);
    }
    if let Some((size, centre)) = manifest.bounding_box {
        commands
            .entity(root)
            .insert(super::PlayerBox(super::BodyBox::from_bounding_box(
                size, centre,
            )));
    }
    if super::is_rail(&manifest) {
        // Rail-bound: held on the track, speed from the rail dynamics (see `rail.rs`); the
        // coupled units are placed on the track too, not by trailer kinematics.
        let mass = commands_mass(&manifest);
        commands.entity(root).insert(super::RailPlayer::new(
            mass,
            manifest.bogie_half_span.unwrap_or(0.0),
        ));
    }
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
        .unwrap_or(Vec3::new(0.0, 1.2, 0.0));
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
        cockpit_offset,
    ) {
        Ok(name) => tracing::info!(human = %name, "player character seated at the driver position"),
        Err(e) => {
            tracing::error!(at = %opts_bevy::position_tag(), "cannot load the player character: {e}")
        }
    }

    tracing::debug!(min = ?min, max = ?max, "vehicle bounding box (Bevy space)");
    let length = (max.z - min.z).abs().max(1.0);
    let height = (max.y - min.y).abs().max(1.0);
    let chase_distance = length * 1.3;
    let chase_height = height * 1.6;
    job.camera_std_index = manifest.std_camera_index.unwrap_or(0);
    let camera = CameraSetup {
        start: Transform::from_translation(
            bevy_pos + root_tf.rotation * Vec3::new(0.0, chase_height, chase_distance),
        )
        .looking_at(bevy_pos + Vec3::Y * (height * 0.5), Vec3::Y),
        chase_distance,
        chase_height,
        cockpit_offset,
    };
    job.camera = Some(camera);

    let vehicle_name = manifest
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
    if activate {
        stats.vehicle_name.clone_from(&vehicle_name);
    }
    // Texture coordinates the decoder could not prove (scrambled meshes): named once per vehicle.
    let (uv_all, uv_bad) = job.cache.stats.uv_scope;
    if uv_all > 0 && uv_bad * 10 > uv_all {
        tracing::warn!(
            vehicle = %vehicle_name,
            unreliable_percent = (100 * uv_bad / uv_all) as u64,
            unreliable_vertices = uv_bad,
            vertices = uv_all,
            "vehicle: more than 10% of the texture coordinates are unreliable (see docs/src/architecture/uv-decoder.md)"
        );
    }
    stats.vehicle_meshes = mesh_count;
    stats.vehicle_vertices = vertex_count;
    stats.scene_vertices += vertex_count;
    stats.asset_failures = job.cache.stats.failures.len();
    tracing::info!(
        vehicle = %vehicle_name,
        session_id,
        meshes = mesh_count,
        vertices = vertex_count,
        uv_vertices = uv_all,
        uv_unreliable = uv_bad,
        "vehicle meshes loaded"
    );
    Ok(SpawnedVehicle {
        root,
        name: vehicle_name,
        camera,
    })
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
    eye_target: Vec3,
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
    // The eyes go where the vehicle's driver camera is (set once the camera is known).
    commands.entity(human).insert(opts_bevy::AlignEyes {
        target: eye_target,
        wait: 3,
    });
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
    engine.set_external_handler(opts_bevy::host_callbacks::vehicle_callbacks(root.to_bits()));
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
    for (var, value) in &job.config.setvars {
        engine.set(var, *value as f32);
    }
    commands.entity(root).insert((
        opts_bevy::VehicleScripts::new(engine),
        opts_bevy::ScriptStartup::default(),
    ));
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

#[cfg(test)]
mod bone_tests {
    use super::is_bone_stem;

    #[test]
    fn bone_dummies_are_bone_underscore_letter() {
        assert!(is_bone_stem("bone_a"));
        assert!(is_bone_stem("bone_d"));
        assert!(is_bone_stem("dummya"));
        assert!(is_bone_stem("dummy_c"));
        assert!(!is_bone_stem("bone_ab"));
        assert!(!is_bone_stem("bone_1"));
        assert!(!is_bone_stem("gn_dum_gelenk_a"));
    }
}
