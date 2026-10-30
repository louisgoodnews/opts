//! Water and spline mesh stages of a tile build.

use std::path::Path;

use opts_bevy::{build_water_mesh, extrude_spline_segment, water_material};
use opts_core::formats::map::{MapTile, MapWater};

use super::*;

/// Water stage: the tile's `.map.water` surface over the terrain it floods.
///
/// Drawn when the tile announces water (`[water]` marker) and the file is there: 93 of 1,906
/// water files in the installation lack the marker, scattered over 17 maps, and are taken as
/// water the editor switched off (`docs/src/formats/map/tiles.md`, Q-M07).
pub(in super::super) fn spawn_water(
    (job, stats, commands): (&mut WorldLoadJob, &mut WorldStats, &mut Commands),
    (meshes, materials): (&mut Assets<Mesh>, &mut Assets<StandardMaterial>),
    (tile, terrain, key, origin): (&MapTile, Option<&TileTerrain>, (i32, i32), (f32, f32)),
    children: &mut Vec<Entity>,
) {
    if !tile.has_water || !job.water_enabled {
        return;
    }
    let rel = format!("{}.water", job.tile_files[&key]);
    let Some(path) = job.install.find_in(&job.config.map_dir, &rel) else {
        tracing::warn!(tile = ?key, "tile announces water but has no .water file");
        return;
    };
    job.loading_deps
        .insert(opts_bevy::hot_reload::cache::normalize(&path));
    let water = match std::fs::read(&path).map(|b| MapWater::parse(&b)) {
        Ok(Ok(w)) => w,
        Ok(Err(e)) => {
            tracing::error!(at = %opts_bevy::position_tag(), path = %path.display(), "water parse failed: {e}");
            return;
        }
        Err(e) => {
            tracing::error!(at = %opts_bevy::position_tag(), path = %path.display(), "water read failed: {e}");
            return;
        }
    };
    let Some(terrain) = terrain else {
        tracing::warn!(tile = ?key, "water without a terrain grid is not drawn");
        return;
    };
    let Some(mesh) = build_water_mesh(&terrain.grid, &water, origin, terrain.cell) else {
        tracing::debug!(tile = ?key, levels = ?water.levels, "water lies below the terrain everywhere");
        return;
    };
    let material = job
        .water_material
        .get_or_insert_with(|| materials.add(water_material()))
        .clone();
    stats.scene_vertices += mesh.count_vertices();
    stats.water_tiles += 1;
    job.tile_water.insert(key, water);
    children.push(
        commands
            .spawn((
                Mesh3d(meshes.add(mesh)),
                MeshMaterial3d(material),
                Name::new(format!("Water_{}_{}", key.0, key.1)),
            ))
            .id(),
    );
}

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
pub(in super::super) fn spawn_splines(
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
