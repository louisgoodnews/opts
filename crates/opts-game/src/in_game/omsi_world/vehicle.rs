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
pub(super) fn load_segment_meshes_for(
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
    let script_defs = job.vehicle_script_defs.clone();
    let mut script_images: std::collections::HashMap<usize, Handle<Image>> = Default::default();
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
    // `[mesh_ident]` / `[animparent]`: a mesh inherits the animation of its parent.
    let pivots = anim_pivots(commands, root, &lod0.meshes);
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
        // The frame this mesh lives in: its parent's pivot (`[animparent]`), else the vehicle. A mesh
        // that others hang on (`[mesh_ident]`) animates its pivot, and its parts sit below it.
        let own_pivot = pivot_of(&pivots, mesh_def.mesh_ident.as_deref());
        let frame = pivot_of(&pivots, mesh_def.animparent.as_deref())
            .filter(|_| {
                !forms_cycle(
                    &lod0.meshes,
                    mesh_def.mesh_ident.as_deref(),
                    mesh_def.animparent.as_deref(),
                )
            })
            .unwrap_or(root);
        if let Some(pivot) = own_pivot {
            commands.entity(frame).add_child(pivot);
            if let Some(anim) = opts_bevy::MeshAnimation::from_mesh(
                owner,
                mesh_def,
                mesh_pivot(job, &mesh_def.path, model_dir, mesh_def),
            ) {
                commands.entity(pivot).insert(anim);
            }
        }
        let parts_parent = own_pivot.unwrap_or(frame);
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
                alpha_variable: m.alpha_scale_var.clone(),
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
                }).or_else(|| {
                    m.script_texture_index.and_then(|i| {
                        if let Some(h) = script_images.get(&i) {
                            return Some(h.clone());
                        }
                        let Some(def) = script_defs.get(i) else {
                            tracing::error!(at = %opts_bevy::position_tag(), mesh = %mesh_def.path, index = i, "[useScriptTexture] names a script texture the model does not declare");
                            return None;
                        };
                        let (w, h) = (def.width.max(1), def.height.max(1));
                        let handle = images.add(opts_bevy::text_texture::text_image(
                            w,
                            h,
                            vec![0; (w * h * 4) as usize],
                        ));
                        commands.spawn((
                            super::vehicle_script_texture::VehicleScriptTexture::new(
                                i,
                                job.vehicle_script_textures.clone(),
                                handle.clone(),
                            ),
                            Name::new(format!("script_texture_{i}")),
                            ChildOf(root),
                        ));
                        script_images.insert(i, handle.clone());
                        Some(handle)
                    })
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
        super::trace::mesh_used(&o3d_path);
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
        let spawned = attach_model_parts(commands, parts_parent, &parts, &mesh_def.path);
        // Script variables move and show the mesh (`[newanim]`, `[visible]`) and switch its light
        // maps; a click on it raises its `[mouseevent]` trigger. A pivot already carries the
        // animation of its parts.
        let anim = if own_pivot.is_some() {
            None
        } else {
            opts_bevy::MeshAnimation::from_mesh(
                owner,
                mesh_def,
                mesh_pivot(job, &mesh_def.path, model_dir, mesh_def),
            )
        };
        let view_scope = opts_bevy::ViewScope::of(mesh_def.viewpoint);
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
            if let Some(scope) = view_scope {
                entity.insert(scope);
            }
            if let Some(var) = &part.light_variable {
                entity.insert(opts_bevy::LightMapBinding::new(owner, var.clone()));
            }
            if let Some(var) = &part.alpha_variable {
                // Starts hidden: the variable decides on the first frame (glass scaled by a
                // script variable that is 0 stays out; dirt and rain overlays are not built).
                entity.insert((
                    opts_bevy::AlphaScaleBinding::new(owner, var.clone()),
                    Visibility::Hidden,
                ));
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

/// One entity per `[mesh_ident]` that some `[animparent]` of the segment names: the frame the
/// parent mesh animates and its children live in (names compare case-insensitively). Spawned
/// below `root` with an identity transform; the loop reparents and animates them in file order,
/// so a child may come before its parent.
fn anim_pivots(
    commands: &mut Commands,
    root: Entity,
    meshes: &[opts_core::formats::vehicles::model::ModelMesh],
) -> std::collections::HashMap<String, Entity> {
    let wanted = referenced_idents(meshes);
    let mut pivots = std::collections::HashMap::new();
    for ident in &wanted {
        let pivot = commands
            .spawn((
                Name::new(format!("anim_{ident}")),
                Transform::IDENTITY,
                Visibility::Inherited,
            ))
            .id();
        commands.entity(root).add_child(pivot);
        pivots.insert(ident.clone(), pivot);
    }
    pivots
}

/// The lower-case `[mesh_ident]`s that are declared by a mesh and named by another mesh's
/// `[animparent]`.
fn referenced_idents(
    meshes: &[opts_core::formats::vehicles::model::ModelMesh],
) -> std::collections::BTreeSet<String> {
    let declared: std::collections::BTreeSet<String> = meshes
        .iter()
        .filter_map(|m| m.mesh_ident.as_deref())
        .map(|i| i.trim().to_lowercase())
        .collect();
    meshes
        .iter()
        .filter_map(|m| m.animparent.as_deref())
        .map(|i| i.trim().to_lowercase())
        .filter(|i| declared.contains(i))
        .collect()
}

/// Whether hanging a mesh named `ident` below `parent` closes a loop of `[animparent]`s (the
/// mesh itself, or a chain of parents that leads back to it).
fn forms_cycle(
    meshes: &[opts_core::formats::vehicles::model::ModelMesh],
    ident: Option<&str>,
    parent: Option<&str>,
) -> bool {
    let (Some(ident), Some(mut at)) = (ident, parent) else {
        return false;
    };
    let same = |a: &str, b: &str| a.trim().eq_ignore_ascii_case(b.trim());
    for _ in 0..=meshes.len() {
        if same(at, ident) {
            return true;
        }
        let up = meshes
            .iter()
            .find(|m| m.mesh_ident.as_deref().is_some_and(|i| same(i, at)))
            .and_then(|m| m.animparent.as_deref());
        match up {
            Some(next) => at = next,
            None => return false,
        }
    }
    true
}

fn pivot_of(
    pivots: &std::collections::HashMap<String, Entity>,
    ident: Option<&str>,
) -> Option<Entity> {
    pivots.get(&ident?.trim().to_lowercase()).copied()
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

/// One car of a train formation behind the head: the vehicle file and whether it is turned
/// around (see `opts_core::formats::trains`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FormationCar {
    /// The resolved vehicle file.
    pub path: PathBuf,
    /// The car runs turned around in the train.
    pub turned: bool,
    /// Whether the car pulls: `None` lets the loader decide (a car made of several files does).
    pub powered: Option<bool>,
}

/// The most vehicle files one spawn follows: a single vehicle's `[couple_back]` chain, or a
/// formation's cars with their own chains.
fn max_parts(formation: bool) -> usize {
    if formation {
        opts_core::formats::trains::MAX_PARTS
    } else {
        40
    }
}

/// The two coupling points that join a unit to the one in front of it, in the frame of the
/// train (y forward): the rear point of the unit in front and the front point of the unit behind,
/// each relative to its own vehicle's origin. A turned unit has its points negated and swapped
/// (its model's front point faces the rear of the train). `None` when a point is missing.
pub(super) fn coupling_joint(
    parent: &BusManifest,
    parent_turned: bool,
    own: &BusManifest,
    own_turned: bool,
) -> Option<(Vec3, Vec3)> {
    let rear_of_parent = if parent_turned {
        parent.coupling_front.map(|c| -c)
    } else {
        parent.coupling_back
    }?;
    let front_of_own = if own_turned {
        own.coupling_back.map(|c| -c)
    } else {
        own.coupling_front
    }?;
    Some((rear_of_parent, front_of_own))
}

/// The installation as the planner reads it.
struct JobFiles<'a>(&'a WorldLoadJob);

impl super::consist_plan::Files for JobFiles<'_> {
    fn find_in(&self, base: &Path, rel: &str) -> Option<PathBuf> {
        self.0.install.find_in(base, rel)
    }

    fn manifest(&self, path: &Path) -> Option<BusManifest> {
        BusManifest::parse_bytes(&std::fs::read(path).ok()?).ok()
    }
}

/// Spawns the units behind the head `root`: its `[couple_back]` chain, then the cars of a train
/// formation (`job.formation_cars`), each with its own chain, where `consist_plan` puts them.
/// Returns the passenger camera presets of all trailers with the segment entity they are
/// defined in.
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
    let formation: Vec<FormationCar> = std::mem::take(&mut job.formation_cars);
    let rail = super::is_rail(tractor);
    let plan = super::consist_plan::plan(
        bus_dir,
        tractor,
        &formation,
        max_parts(!formation.is_empty()),
        &JobFiles(job),
    );
    let mut cams = Vec::new();
    // Mass of the unit in front: what a trailer without a stated mass is assumed to weigh.
    let mut previous_kg = manifest_mass_kg(tractor).unwrap_or(HEAD_MASS_DEFAULT_KG);
    let mut chain = Vec::new();
    let mut layout = Vec::new();
    // Axles counted so far: trailer models number theirs after the head's (`Wheel_Rotation_2_L`).
    let mut axle_base = tractor.axles.len();
    // Rail: the head's mass (at least 20 t), then the stated mass of every following unit and
    // whether it pulls; a unit that states none adds nothing.
    let head_kg = commands_mass(tractor);
    let mut powered_kg = head_kg;
    let mut total_kg = head_kg;
    for part in &plan.followers {
        let (path, manifest, dir) = (&part.path, &part.manifest, &part.dir);
        let name = path
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_default();
        let loaded = (|| -> Result<_, String> {
            let cfg_rel = manifest.model_cfg.clone().ok_or("no [model] entry")?;
            let cfg = job
                .install
                .find_in(dir, &cfg_rel)
                .ok_or_else(|| format!("model config not found: {cfg_rel}"))?;
            let model = ModelConfig::parse_bytes(&std::fs::read(&cfg).map_err(|e| e.to_string())?)
                .map_err(|e| e.to_string())?;
            Ok((cfg, model))
        })();
        let (cfg, model) = match loaded {
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
        job.vehicle_script_defs = model.script_textures.clone();
        let canvases: Vec<opts_core::formats::script_texture::CpuScriptTexture> = model
            .script_textures
            .iter()
            .map(|st| {
                opts_core::formats::script_texture::CpuScriptTexture::new(st.width, st.height)
            })
            .collect();
        job.vehicle_script_textures = std::sync::Arc::new(std::sync::RwLock::new(canvases));
        let reversed = part.turned;
        // The link to the part in front inside its unit (a unit's first file has none: only rail
        // places those, by `behind`, and the articulated segment is unused there).
        let (b, f) = part
            .joint
            .unwrap_or((Vec3::new(0.0, -part.behind, 0.0), Vec3::ZERO));
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
        // Road trailers assume the mass of the unit in front when they state none; rail cars
        // (bogies, joints and couplers are vehicles of their own and state 0) add what they state.
        let fallback_kg = if rail { 0.0 } else { previous_kg };
        if !rail && manifest_mass_kg(manifest).is_none() {
            tracing::warn!(trailer = %path.display(), assumed_kg = previous_kg, "trailer states no mass; the mass of the unit in front is used");
        }
        let load = opts_sim::UnitLoad::from_manifest_or(manifest, reversed, fallback_kg);
        previous_kg = load.mass_kg;
        if rail {
            total_kg += load.mass_kg;
            if part.powered {
                powered_kg += load.mass_kg;
            }
        }
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
                    s.load = Some(load.clone());
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
        let model_dir = cfg.parent().map(Path::to_path_buf).unwrap_or_default();
        let tex_dirs = vec![
            job.install
                .find_in(dir, "texture")
                .unwrap_or_else(|| dir.join("texture")),
            model_dir.join("texture"),
        ];
        job.cache.texture_replacements = repaint_map(job, &model);
        let (count, verts, _, _) = load_segment_meshes_for(
            job, commands, meshes, materials, images, seg, root, lod0, &model_dir, &tex_dirs,
        );
        job.cache.texture_replacements.clear();
        tracing::info!(trailer = %name, meshes = count, vertices = verts, "trailer segment attached");
        layout.push(super::rail::Part {
            entity: seg,
            name,
            behind: part.behind,
            half_span: manifest.bogie_half_span.unwrap_or(0.0),
            reversed,
            meshes: count,
            vertices: verts,
            body: manifest
                .bounding_box
                .map(|(size, centre)| super::BodyBox::from_bounding_box(size, centre)),
        });
        cams.extend(manifest.pax_cameras.iter().cloned().map(|c| (seg, c)));
    }
    if rail {
        // The whole consist's masses for the dynamics (set once the head has its `RailPlayer`).
        let load = opts_sim::rail::ConsistLoad {
            total_kg,
            powered_kg,
            braked_kg: total_kg,
        };
        commands.queue(move |world: &mut World| {
            if let Some(mut player) = world.get_mut::<super::rail::RailPlayer>(root) {
                player.set_consist(load);
            }
        });
    }
    if !chain.is_empty() {
        if !rail {
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

/// Mass the simulation assumes for a head unit that states none (kg).
const HEAD_MASS_DEFAULT_KG: f32 = 11_500.0;

/// The mass a vehicle file states (kg); `None` when it states none or an implausible one.
fn manifest_mass_kg(manifest: &BusManifest) -> Option<f32> {
    manifest
        .mass_tonnes
        .filter(|&m| m > 0.5)
        .map(|m| m * 1_000.0)
}

/// What a following unit adds to the consist: its stated mass, else `previous_kg` (the unit in
/// front; the installed consists have a median trailer/head ratio of 0.90, `consist_census`), and
/// its axles with those marked driven (`achse_antrieb`).
#[cfg(test)]
fn unit_load(manifest: &BusManifest, reversed: bool, previous_kg: f32) -> opts_sim::UnitLoad {
    opts_sim::UnitLoad::from_manifest_or(manifest, reversed, previous_kg)
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
    job.vehicle_script_defs = model.script_textures.clone();
    let canvases: Vec<opts_core::formats::script_texture::CpuScriptTexture> = model
        .script_textures
        .iter()
        .map(|st| opts_core::formats::script_texture::CpuScriptTexture::new(st.width, st.height))
        .collect();
    job.vehicle_script_textures = std::sync::Arc::new(std::sync::RwLock::new(canvases));
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
    job.cache.texture_replacements = repaint_map(job, &model);
    super::trace::begin_frame();
    let (mesh_count, vertex_count, min, max) = load_segment_meshes(
        job, commands, meshes, materials, images, root, lod0, &model_dir, &tex_dirs,
    );
    let trace_frame = super::trace::end_frame();
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
    // The catalogue identity of the vehicle keys its stored viewpoints.
    if let Some(key) = crate::viewpoints::key_of_bus_file(&bus_path) {
        commands
            .entity(root)
            .insert(crate::viewpoints::ViewpointKey(key));
    }
    if let Some(model) = crate::viewpoints::model_of_bus_file(&bus_path) {
        commands
            .entity(root)
            .insert(crate::viewpoints::ViewpointModel(model));
    }
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
        let mut player = super::RailPlayer::new(mass, manifest.bogie_half_span.unwrap_or(0.0));
        if !job.formation_cars.is_empty() {
            player = player.with_snap(super::rail::FORMATION_SNAP_M);
        }
        commands.entity(root).insert(player);
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
    stats.vehicle_meshes = mesh_count;
    stats.vehicle_vertices = vertex_count;
    stats.scene_vertices += vertex_count;
    stats.asset_failures = job.cache.stats.failures.len();
    tracing::info!(
        vehicle = %vehicle_name,
        session_id,
        meshes = mesh_count,
        vertices = vertex_count,
        "vehicle meshes loaded"
    );
    if opts_bevy::scene_trace::enabled() {
        super::trace::spawned(
            opts_bevy::scene_trace::kind::VEHICLE,
            &vehicle_name,
            &opts_bevy::scene_trace::relative(&bus_path),
            root,
            &root_tf,
            &trace_frame,
            "ok",
            serde_json::json!({
                "session_id": session_id, "active": activate,
                "model_cfg": opts_bevy::scene_trace::relative(&cfg_path),
                "meshes": mesh_count, "vertices": vertex_count,
            }),
        );
    }
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
                alpha_variable: m.alpha_scale_var.clone(),
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
        super::trace::mesh_used(&o3d_path);
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

/// Discovers and loads the best matching depot file for the vehicle and current map.
fn load_depot_file(
    bus_dir: &Path,
    omsi_root: &Path,
    map_name: &str,
) -> Option<opts_core::formats::hof::HofFile> {
    let map_clean = map_name.trim().to_ascii_lowercase();

    // 1. Look in the vehicle directory
    if let Ok(entries) = std::fs::read_dir(bus_dir) {
        let mut candidates = Vec::new();
        for entry in entries.flatten() {
            let path = entry.path();
            if path
                .extension()
                .is_some_and(|ext| ext.eq_ignore_ascii_case("hof"))
            {
                let stem = path
                    .file_stem()
                    .unwrap_or_default()
                    .to_string_lossy()
                    .to_ascii_lowercase();
                if !map_clean.is_empty() && stem == map_clean {
                    if let Ok(bytes) = std::fs::read(&path) {
                        if let Ok(hof) = opts_core::formats::hof::HofFile::parse_bytes(&bytes) {
                            return Some(hof);
                        }
                    }
                }
                candidates.push((stem, path));
            }
        }
        if !map_clean.is_empty() {
            for (stem, path) in &candidates {
                if stem.contains(&map_clean) || map_clean.contains(stem.as_str()) {
                    if let Ok(bytes) = std::fs::read(path) {
                        if let Ok(hof) = opts_core::formats::hof::HofFile::parse_bytes(&bytes) {
                            return Some(hof);
                        }
                    }
                }
            }
        }
        if let Some((_, path)) = candidates.first() {
            if let Ok(bytes) = std::fs::read(path) {
                if let Ok(hof) = opts_core::formats::hof::HofFile::parse_bytes(&bytes) {
                    return Some(hof);
                }
            }
        }
    }

    // 2. Look in shared Vehicles folders in OMSI root
    if !map_clean.is_empty() {
        let shared_dirs = [
            omsi_root.join("Vehicles"),
            omsi_root.join("Vehicles/.hof Dateien"),
        ];
        for dir in shared_dirs {
            if let Ok(entries) = std::fs::read_dir(dir) {
                for entry in entries.flatten() {
                    let path = entry.path();
                    if path
                        .extension()
                        .is_some_and(|ext| ext.eq_ignore_ascii_case("hof"))
                    {
                        let stem = path
                            .file_stem()
                            .unwrap_or_default()
                            .to_string_lossy()
                            .to_ascii_lowercase();
                        if stem == map_clean || stem.contains(&map_clean) {
                            if let Ok(bytes) = std::fs::read(&path) {
                                if let Ok(hof) =
                                    opts_core::formats::hof::HofFile::parse_bytes(&bytes)
                                {
                                    return Some(hof);
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    None
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
    let map_name = job
        .manifest
        .friendly_name
        .as_deref()
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| {
            job.config
                .map_dir
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("")
        });
    let hof = load_depot_file(bus_dir, job.install.root(), map_name);

    let mut active_route_index = None;
    let mut active_line_string = String::new();

    if let Some(h) = &hof {
        tracing::info!(
            depot_name = %h.name,
            termini = h.termini.len(),
            routes = h.routes.len(),
            "depot (.hof) file loaded for vehicle"
        );
        engine.set_string("Yard", &h.name);

        if let Some((idx, r)) = h
            .routes
            .iter()
            .enumerate()
            .find(|(_, r)| r.line_string == "33" || r.route_code / 100 == 33)
            .or_else(|| h.routes.iter().enumerate().next())
        {
            active_route_index = Some(idx);
            active_line_string = if r.line_string.chars().all(|c| c == 'X') {
                (r.route_code / 100).to_string()
            } else {
                r.line_string.clone()
            };
            engine.set_string("SetLineTo", &active_line_string);
            engine.set_string("Ident", &r.route_code.to_string());
            engine.set("schedule_active", 1.0);
        }
    }

    let ctx = opts_bevy::host_callbacks::VehicleHostContext {
        seed: root.to_bits(),
        hof: hof.map(std::sync::Arc::new),
        active_route_index,
        current_stop_index: 0,
        delay_seconds: 0.0,
        line_string: active_line_string,
        script_textures: job.vehicle_script_textures.clone(),
        fonts: std::sync::Arc::new(std::sync::RwLock::new(opts_bevy::text_texture::load_fonts(
            &job.install,
        ))),
        texture_base_dir: Some(bus_dir.join("Texture")),
    };
    engine.set_external_handler(opts_bevy::host_callbacks::vehicle_callbacks_with_context(
        ctx,
    ));
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

#[cfg(test)]
mod consist_mass_tests {
    use super::*;

    fn bus(mass: Option<f32>, axles: &[bool]) -> BusManifest {
        let mut text = String::new();
        if let Some(m) = mass {
            text.push_str(&format!("[mass]\n{m}\n\n"));
        }
        for driven in axles {
            text.push_str(&format!(
                "[newachse]\n\nachse_antrieb\n{}\n\n",
                u8::from(*driven)
            ));
        }
        BusManifest::parse(&text).expect("manifest")
    }

    #[test]
    fn a_trailer_adds_its_stated_mass_and_driven_axles() {
        let u = unit_load(&bus(Some(7.2), &[false, true]), false, 8_000.0);
        assert_eq!(u.mass_kg, 7_200.0);
        assert_eq!(u.axles.len(), 2);
        assert_eq!(u.axles.iter().filter(|a| a.is_driven).count(), 1);
    }

    #[test]
    fn a_trailer_without_a_mass_takes_the_mass_of_the_unit_in_front() {
        assert_eq!(
            unit_load(&bus(None, &[false]), false, 8_000.0).mass_kg,
            8_000.0
        );
        // An implausible mass counts as missing, like for the head.
        assert_eq!(
            unit_load(&bus(Some(0.1), &[false]), false, 8_000.0).mass_kg,
            8_000.0
        );
    }

    /// The MAN GN92 (the project's NG272 test vehicle) of the installed corpus: 8 t head and
    /// 7.2 t trailer make a 15.2 t consist. Skipped without `OMSI_PATH`.
    #[test]
    fn the_gn92_consist_weighs_head_and_trailer() {
        let Some(root) = std::env::var_os("OMSI_PATH").map(std::path::PathBuf::from) else {
            return;
        };
        let dir = root.join("Vehicles/MAN_NL_NG");
        let read = |f: &str| {
            std::fs::read(dir.join(f))
                .ok()
                .and_then(|b| BusManifest::parse_bytes(&b).ok())
        };
        let (Some(main), Some(trail)) = (read("MAN_GN92_main.bus"), read("MAN_GN92_trail.bus"))
        else {
            return;
        };
        let head = manifest_mass_kg(&main).expect("head mass");
        let mut sim = opts_sim::VehicleSimulation::from_manifest(&main);
        sim.set_trailers(vec![unit_load(&trail, false, head)]);
        assert!(
            (sim.consist_mass_kg() - 15_200.0).abs() < 1.0,
            "{}",
            sim.consist_mass_kg()
        );
        assert!((sim.head_mass_kg() - 8_000.0).abs() < 1.0);
    }
}

#[cfg(test)]
mod formation_tests {
    use super::*;

    /// A manifest with the two coupling points at `front_y` and `back_y` (x 0, z 1).
    fn car(front_y: f32, back_y: f32) -> BusManifest {
        BusManifest::parse(&format!(
            "[coupling_front]\n0\n{front_y}\n1\n\n[coupling_back]\n0\n{back_y}\n1\n\n"
        ))
        .expect("manifest")
    }

    /// Distance of the follower's origin behind the leader's origin: the loader's `f.y - b.y`.
    fn gap(parent: &BusManifest, pt: bool, own: &BusManifest, ot: bool) -> f32 {
        let (b, f) = coupling_joint(parent, pt, own, ot).expect("points");
        f.y - b.y
    }

    #[test]
    fn two_unturned_cars_join_back_to_front() {
        // Leader's rear coupling 10 m behind its origin, follower's front coupling 12 m ahead.
        let (a, b) = (car(11.0, -10.0), car(12.0, -9.0));
        assert!((gap(&a, false, &b, false) - 22.0).abs() < 1e-4);
    }

    #[test]
    fn a_turned_follower_presents_its_back_point_as_its_front() {
        let (a, b) = (car(11.0, -10.0), car(12.0, -9.0));
        // Turned: its model's back point (-9) becomes the front point (+9).
        assert!((gap(&a, false, &b, true) - 19.0).abs() < 1e-4);
        let (b_pt, f_pt) = coupling_joint(&a, false, &b, true).expect("points");
        assert_eq!((b_pt.y, f_pt.y), (-10.0, 9.0));
    }

    #[test]
    fn a_turned_leader_presents_its_front_point_as_its_rear() {
        let (a, b) = (car(11.0, -10.0), car(12.0, -9.0));
        // Turned leader: its model's front point (+11) becomes the rear point (-11).
        assert!((gap(&a, true, &b, false) - 23.0).abs() < 1e-4);
        // Both turned: rear -11, follower front +9.
        assert!((gap(&a, true, &b, true) - 20.0).abs() < 1e-4);
    }

    #[test]
    fn a_symmetric_pair_is_the_same_whichever_way_it_is_turned() {
        let (a, b) = (car(13.2, -13.2), car(13.2, -13.2));
        for (pt, ot) in [(false, false), (false, true), (true, false), (true, true)] {
            assert!((gap(&a, pt, &b, ot) - 26.4).abs() < 1e-4, "{pt} {ot}");
        }
    }

    #[test]
    fn a_missing_point_gives_no_joint() {
        let a = car(11.0, -10.0);
        let bare = BusManifest::parse("[mass]\n10\n").expect("manifest");
        assert!(coupling_joint(&a, false, &bare, false).is_none());
        assert!(coupling_joint(&bare, false, &a, false).is_none());
    }

    #[test]
    fn the_ai_loader_and_the_formation_loader_agree_on_a_couple_back_link() {
        // The old inline formula of `load_spec`/`spawn_trailer_chain` for a link flagged reversed.
        let (a, b) = (car(11.0, -10.0), car(12.0, -9.0));
        let old = |reversed: bool| {
            let own = if reversed {
                b.coupling_back.map(|c| -c)
            } else {
                b.coupling_front
            };
            own.expect("own").y - a.coupling_back.expect("back").y
        };
        for reversed in [false, true] {
            assert!((gap(&a, false, &b, reversed) - old(reversed)).abs() < 1e-6);
        }
    }

    #[test]
    fn the_part_limit_is_forty_for_a_single_vehicle_and_the_formation_limit_otherwise() {
        assert_eq!(max_parts(false), 40);
        assert_eq!(max_parts(true), opts_core::formats::trains::MAX_PARTS);
    }
}

#[cfg(test)]
mod anim_parent_tests {
    use super::*;
    use opts_core::formats::vehicles::model::ModelConfig;

    fn meshes(text: &str) -> Vec<opts_core::formats::vehicles::model::ModelMesh> {
        let config = ModelConfig::parse(text).expect("model");
        config.lods.into_iter().next().expect("lod").meshes
    }

    /// The cockpit shape of the Hamburg e-bus: a dashboard base that tilts, with switches hung on it.
    const COCKPIT: &str = "[mesh]\nbase.o3d\n[mesh_ident]\nCP_Basis\n\n\
[mesh]\nswitch_a.o3d\n[animparent]\ncp_basis\n\n\
[mesh]\nswitch_b.o3d\n[animparent]\nunknown_parent\n\n\
[mesh]\nflap.o3d\n[mesh_ident]\nflap\n[animparent]\nhandle\n\n\
[mesh]\nhandle.o3d\n[mesh_ident]\nhandle\n[animparent]\nflap\n";

    #[test]
    fn only_declared_and_named_idents_get_a_pivot() {
        let m = meshes(COCKPIT);
        let ids: Vec<_> = referenced_idents(&m).into_iter().collect();
        assert_eq!(ids, ["cp_basis", "flap", "handle"]);
    }

    #[test]
    fn a_loop_of_parents_is_cut() {
        let m = meshes(COCKPIT);
        assert!(forms_cycle(&m, Some("flap"), Some("handle")));
        assert!(forms_cycle(&m, Some("handle"), Some("flap")));
        assert!(forms_cycle(&m, Some("x"), Some("x")), "own parent");
        assert!(
            !forms_cycle(&m, None, Some("cp_basis")),
            "a leaf has no ident"
        );
        assert!(!forms_cycle(&m, Some("switch"), Some("CP_BASIS")));
    }
}
