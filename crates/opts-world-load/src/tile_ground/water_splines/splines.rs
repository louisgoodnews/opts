//! The spline stage of a tile build: extrudes each spline's profile into ribbons, with one material
//! per profile texture shared by all of its ribbons.

use std::path::Path;

use opts_bevy::extrude_spline_segment;
use opts_core::formats::map::MapTile;

use super::*;

/// The material of one texture of a spline profile, shared by every ribbon of that profile.
fn spline_material(
    job: &mut WorldLoadJob,
    (meshes, materials, images): (
        &mut Assets<Mesh>,
        &mut Assets<StandardMaterial>,
        &mut Assets<Image>,
    ),
    (profile, sli_key, tex_idx): (
        &opts_core::formats::scenery::SplineProfile,
        &std::path::PathBuf,
        usize,
    ),
    tex_dirs: &[std::path::PathBuf],
) -> Handle<StandardMaterial> {
    if let Some(m) = job.spline_materials.get(&(sli_key.clone(), tex_idx)) {
        m.clone()
    } else {
        let tex = profile.textures.get(tex_idx);
        let handle = tex.and_then(|t| {
            let mut loader = ContentLoader {
                install: &job.install,
                cache: &mut job.cache,
                meshes,
                materials,
                images,
            };
            loader.texture(&t.file, tex_dirs)
        });
        let alpha = tex.is_some_and(|t| t.alpha_mode.is_some());
        let m = job.cache.pool.material(
            materials,
            StandardMaterial {
                base_color_texture: handle,
                perceptual_roughness: 0.9,
                // Asphalt and pavement have no specular colour in OMSI: no sun glare.
                reflectance: 0.1,
                double_sided: true,
                cull_mode: None,
                alpha_mode: if alpha {
                    AlphaMode::Mask(0.5)
                } else {
                    AlphaMode::Opaque
                },
                // Road surfaces share their plane with the terrain deformed to them.
                depth_bias: 4.0,
                ..default()
            },
        );
        job.spline_materials
            .insert((sli_key.clone(), tex_idx), m.clone());
        m
    }
}

/// Spline stage: extrudes every spline's profile and spawns the ribbons.
pub(in super::super::super) fn spawn_splines(
    (job, stats, commands): (&mut WorldLoadJob, &mut WorldStats, &mut Commands),
    (meshes, materials, images): (
        &mut Assets<Mesh>,
        &mut Assets<StandardMaterial>,
        &mut Assets<Image>,
    ),
    (tile, origin): (&MapTile, (f32, f32)),
    children: &mut Vec<Entity>,
) {
    // --- Splines ---------------------------------------------------------------------------
    for inst in &tile.splines {
        let Some(profile) = job.sli(&inst.sli_file) else {
            stats.splines_failed += 1;
            continue;
        };
        if profile.only_editor {
            continue;
        }
        let spline_meshes = extrude_spline_segment(&profile, inst, origin);
        if spline_meshes.is_empty() {
            // Profiles without a graphical part (path-only helpers) have nothing to draw.
            tracing::debug!(sli = %inst.sli_file, "spline has no graphical profile");
            continue;
        }
        let sli_key = job.locate(&inst.sli_file).unwrap_or_default();
        // Spline textures live in the profile's own pack folder (`Splines/<Pack>/texture`).
        let sli_dir = sli_key.parent().map(Path::to_path_buf).unwrap_or_default();
        let tex_dirs = vec![
            sli_dir.join("texture"),
            sli_dir,
            job.config.map_dir.join("texture"),
        ];
        for (tex_idx, mesh) in spline_meshes {
            let mat = spline_material(
                job,
                (meshes, materials, images),
                (&profile, &sli_key, tex_idx),
                &tex_dirs,
            );
            stats.scene_vertices += mesh.count_vertices();
            // `OPTS_INSTANCING=merge`: the ribbon (already in world space) joins the merged mesh.
            if opts_bevy::instancing::merge()
                && opts_bevy::instancing::StaticMerger::mesh_ok(&mesh)
                && materials
                    .get(&mat)
                    .is_some_and(opts_bevy::instancing::StaticMerger::material_ok)
                && job.cache.merger.bake(&mesh, &mat, &Transform::IDENTITY)
            {
                continue;
            }
            children.push(
                commands
                    .spawn((
                        (Mesh3d(meshes.add(mesh)), MeshMaterial3d(mat)),
                        Name::new(format!("Spline_{}_{}", inst.id_code, tex_idx)),
                    ))
                    .id(),
            );
        }
        stats.splines_loaded += 1;
    }
}
