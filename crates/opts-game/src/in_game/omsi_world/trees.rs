//! `[tree]` billboards.

use std::path::Path;

use bevy::prelude::*;
use opts_bevy::coords::{omsi_euler_to_bevy_quat, omsi_to_bevy_pos};
use opts_bevy::{sample_terrain, tree_billboard_material, tree_billboard_mesh, ContentLoader};
use opts_core::formats::map::{MapObjectInstance, MapTerrain};
use opts_core::formats::scenery::ScoTree;

/// Asset stores the loading systems write to (grouped to stay within Bevy's parameter limit).
use super::{WorldLoadJob, WorldStats};

/// Spawns a `[tree]` billboard. The instance's string variables carry
/// `[texture, height, width/height ratio]`; the `.sco` ranges are the fallback.
#[allow(clippy::too_many_arguments)]
pub(super) fn spawn_tree(
    job: &mut WorldLoadJob,
    stats: &mut WorldStats,
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    images: &mut Assets<Image>,
    tree: &ScoTree,
    obj: &MapObjectInstance,
    origin: (f32, f32),
    terrain: Option<&MapTerrain>,
) -> Option<Entity> {
    let var = |i: usize| {
        obj.string_vars
            .get(i)
            .map(|s| s.trim())
            .filter(|s| !s.is_empty())
    };
    let texture = var(0).unwrap_or(&tree.texture).to_string();
    let height = var(1)
        .and_then(|v| v.parse::<f32>().ok())
        .unwrap_or((tree.min_height + tree.max_height) * 0.5);
    let ratio = var(2)
        .and_then(|v| v.parse::<f32>().ok())
        .unwrap_or((tree.min_width_ratio + tree.max_width_ratio) * 0.5);

    let sco_dir = job
        .install
        .find_file(&obj.sco_file)
        .and_then(|p| p.parent().map(Path::to_path_buf))
        .unwrap_or_default();
    let dirs = vec![sco_dir.join("texture"), job.config.map_dir.join("texture")];
    let handle = {
        let mut loader = ContentLoader {
            install: &job.install,
            cache: &mut job.cache,
            meshes,
            materials,
            images,
        };
        loader.texture(&texture, &dirs)
    };
    let Some(handle) = handle else {
        stats.objects_failed += 1;
        return None;
    };

    let ground = terrain.map_or(0.0, |t| sample_terrain(t, obj.position.x, obj.position.y));
    let world = Vec3::new(
        origin.0 + obj.position.x,
        origin.1 + obj.position.y,
        ground + obj.position.z,
    );
    let mesh = tree_billboard_mesh(height, height * ratio);
    stats.scene_vertices += mesh.count_vertices();
    let material = materials.add(tree_billboard_material(handle));
    let id = commands
        .spawn((
            (
                Mesh3d(meshes.add(mesh)),
                MeshMaterial3d(material),
                Transform::from_translation(omsi_to_bevy_pos(world))
                    .with_rotation(omsi_euler_to_bevy_quat(-obj.heading_deg, 0.0, 0.0)),
            ),
            Name::new(format!("Tree_{}", obj.id_code)),
        ))
        .id();
    stats.trees_loaded += 1;
    stats.objects_loaded += 1;
    Some(id)
}
