//! Loading the segment meshes of a vehicle and its repaints.

use super::skins::{
    anim_pivots, articulation_runs, bind_skins, forms_cycle, is_bone_stem, mesh_pivot, pivot_of,
    SkinBinding,
};
use super::*;

/// Reads the first vehicle listed in the map's `ailists.cfg` (fallback when no `--vehicle-path`).
pub(in super::super::super) fn first_ai_list_vehicle(job: &WorldLoadJob) -> Option<PathBuf> {
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
pub(super) fn ensure_repaints(job: &mut WorldLoadJob, bus_path: &Path, manifest: &BusManifest) {
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
pub(super) fn repaint_map(
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
pub(in super::super::super) fn load_segment_meshes(
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
pub(in super::super::super) fn load_segment_meshes_for(
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
    // How often a mesh of each file name came before (see `in_file_order`).
    let mut repeats: std::collections::HashMap<String, u32> = Default::default();
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
                        super::super::vehicle_text::VehicleTextTexture::new(owner, def, handle.clone()),
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
                        // A canvas used as `[matl_transmap] \S:n` is only the alpha: the colour
                        // is the material's own texture (an LED matrix's `.dds`).
                        let transmap = m
                            .trans_map
                            .as_deref()
                            .is_some_and(|t| t.trim().to_ascii_lowercase().starts_with("\\s:"));
                        let tint = transmap
                            .then(|| {
                                tex_dirs
                                    .iter()
                                    .find_map(|d| job.install.find_in(d, &m.texture))
                                    .and_then(|p| std::fs::read(p).ok())
                                    .and_then(|b| opts_bevy::texture::load_bevy_image(&b).ok())
                                    .and_then(|img| {
                                        super::super::vehicle_script_texture::bright_colour(&img)
                                    })
                            })
                            .flatten();
                        let mut blank = vec![0; (w * h * 4) as usize];
                        if let Some(rgb) = tint {
                            for p in blank.chunks_exact_mut(4) {
                                p[..3].copy_from_slice(&rgb);
                            }
                        }
                        let handle =
                            images.add(opts_bevy::text_texture::text_image(w, h, blank));
                        let mut component =
                            super::super::vehicle_script_texture::VehicleScriptTexture::new(
                                i,
                                job.vehicle_script_textures.clone(),
                                handle.clone(),
                            );
                        if let Some(rgb) = tint {
                            component = component.with_tint(rgb);
                        }
                        // The colour texture may be named by a string variable
                        // (`[matl_freetex]`: the LED matrix's `Matrix_Farbe`, the ATRON AFR4's
                        // `AFR4_ST_texture` over a black placeholder, which has no colour of its
                        // own to start from).
                        if let (true, Some(free)) = (transmap, &m.free_tex) {
                            component = component.following(
                                owner,
                                free.variable.trim(),
                                tex_dirs.to_vec(),
                            );
                        }
                        commands.spawn((
                            component,
                            Name::new(format!("script_texture_{i}")),
                            ChildOf(root),
                        ));
                        script_images.insert(i, handle.clone());
                        Some(handle)
                    })
                }),
                light_map: m.light_map.clone(),
                night_map: m.night_map.clone().filter(|t| !t.trim().is_empty()),
                // A texture named by a string variable (`[matl_freetex]`): the C2 display's
                // pages. Canvases follow theirs through the tint above.
                free_texture: m
                    .free_tex
                    .as_ref()
                    .filter(|_| m.script_texture_index.is_none())
                    .map(|f| f.variable.trim().to_owned())
                    .filter(|v| !v.is_empty()),
            })
            .collect();
        let mut loader = ContentLoader {
            install: &job.install,
            cache: &mut job.cache,
            meshes,
            materials,
            images,
        };
        super::super::trace::mesh_used(&o3d_path);
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
        let copy = *repeats
            .entry(mesh_stem(&mesh_def.path))
            .and_modify(|n| *n += 1)
            .or_insert(0);
        let parts = in_file_order(job, materials, parts, copy);
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
            if let Some((var, light_map)) = &part.free_texture {
                entity.insert(opts_bevy::free_texture::FreeTextureBinding::new(
                    owner,
                    var.clone(),
                    tex_dirs.to_vec(),
                    light_map.clone(),
                ));
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

/// The file name of a `[mesh]` path without folders and extension, lower case: layer copies of
/// one display sit in different folders under one name.
fn mesh_stem(path: &str) -> String {
    let name = path.rsplit(['\\', '/']).next().unwrap_or(path);
    name.rsplit_once('.')
        .map_or(name, |(stem, _)| stem)
        .to_lowercase()
}

/// Depth bias per earlier mesh of the same file name (constant depth-bias units).
const COPY_DEPTH_BIAS: f32 = 1.0;

/// The parts of the `copy`-th mesh of one file name in the model (0: the first). OMSI draws a
/// model's meshes in file order and a later surface at equal depth wins, which displays rely on:
/// the Lion's City's VDV display stacks a background (`VDV\BHD_LC_VDV_Display.o3d`, the
/// placeholder texture), the page and its overlays (`VDV\Display\BHD_LC_VDV_Display.o3d`) on
/// one screen. Bevy keeps no such order, so a later copy gets a slightly larger depth bias (its
/// own material, shared through the pool).
fn in_file_order(
    job: &mut WorldLoadJob,
    materials: &mut Assets<StandardMaterial>,
    mut parts: Vec<opts_bevy::ModelPart>,
    copy: u32,
) -> Vec<opts_bevy::ModelPart> {
    if copy == 0 {
        return parts;
    }
    for part in &mut parts {
        let Some(mut m) = materials.get(&part.material).cloned() else {
            continue;
        };
        m.depth_bias += COPY_DEPTH_BIAS * copy as f32;
        part.material = job.cache.pool.material(materials, m);
    }
    parts
}

#[cfg(test)]
mod order_tests {
    use super::mesh_stem;

    #[test]
    fn copies_in_other_folders_share_a_stem() {
        assert_eq!(
            mesh_stem("VDV\\BHD_LC_VDV_Display.o3d"),
            "bhd_lc_vdv_display"
        );
        assert_eq!(
            mesh_stem("VDV\\Display\\BHD_LC_VDV_Display.o3d"),
            "bhd_lc_vdv_display"
        );
        assert_eq!(mesh_stem("plain.x"), "plain");
        assert_eq!(mesh_stem("noext"), "noext");
    }
}
