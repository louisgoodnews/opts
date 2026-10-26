//! `[tree]` billboards, and the objects that model a tree as a mesh (M-0144 phase C).

use std::path::Path;

use bevy::prelude::*;
use opts_bevy::coords::{omsi_euler_to_bevy_quat, omsi_to_bevy_pos};
use opts_bevy::{tree_billboard_material, tree_billboard_mesh, ContentLoader, TileTerrain};
use opts_core::formats::map::MapObjectInstance;
use opts_core::formats::scenery::{SceneryObject, ScoTree};

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
    terrain: Option<&TileTerrain>,
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

    let ground = terrain.map_or(0.0, |t| t.height(obj.position.x, obj.position.y));
    let world = Vec3::new(
        origin.0 + obj.position.x,
        origin.1 + obj.position.y,
        ground + obj.position.z,
    );
    let mesh = tree_billboard_mesh(height, height * ratio);
    stats.scene_vertices += mesh.count_vertices();
    let material = job
        .cache
        .pool
        .material(materials, tree_billboard_material(handle));
    let transform = Transform::from_translation(omsi_to_bevy_pos(world))
        .with_rotation(omsi_euler_to_bevy_quat(-obj.heading_deg, 0.0, 0.0));
    // Not baked into a tile's merged mesh (`OPTS_INSTANCING=merge`) since trees move in the wind
    // (M-0144): each billboard stays its own entity.
    let id = commands
        .spawn((
            (
                Mesh3d(job.cache.pool.mesh(meshes, mesh)),
                MeshMaterial3d(material),
                transform,
            ),
            // Moves in the wind once `TreeWindPlugin` gives it the tree material (M-0144).
            opts_bevy::tree_wind::WindTree,
            Name::new(format!("Tree_{}", obj.id_code)),
        ))
        .id();
    stats.trees_loaded += 1;
    stats.objects_loaded += 1;
    Some(id)
}

/// Words of a foliage texture's name (lower case, `ae`/`oe`/`ue` for umlauts).
const FOLIAGE_WORDS: &[&str] = &[
    "laub",
    "leaf",
    "leaves",
    "blatt",
    "blaetter",
    "foliage",
    "nadel",
    "zweig",
    "branch",
    "baum",
    "baeume",
    "tree",
    "busch",
    "buesche",
    "bush",
    "strauch",
    "straeucher",
    "hecke",
    "hedge",
    "conifer",
    "tanne",
    "fichte",
    "kiefer",
    "pine",
    "birke",
    "eiche",
    "oak",
    "linde",
    "ahorn",
    "kastanie",
    "chestnut",
    "pappel",
    "weide",
    "palm",
    "palme",
    "gras",
    "grass",
];

/// The generic German words that may also end a compound (`Laubbaum`, `Kastanienbaum`,
/// `Ziergras`); the others only stand as words of their own (`SK_Bushokje` is a shelter, a
/// `Zifferblatt` a clock face).
const COMPOUND_TAILS: &[&str] = &[
    "baum",
    "baeume",
    "busch",
    "buesche",
    "strauch",
    "straeucher",
    "hecke",
    "laub",
    "gras",
];

/// Plural and inflection endings a word may carry (`Tannen`, `Birken`, `Zweige`, `leafs`).
const ENDINGS: &[&str] = &["", "s", "e", "n", "en", "es", "er"];

/// Whether `token` is a foliage word (optionally inflected), or ends in one that may close a
/// compound.
fn is_foliage_token(token: &str) -> bool {
    ENDINGS.iter().any(|end| {
        let Some(stem) = token.strip_suffix(end) else {
            return false;
        };
        FOLIAGE_WORDS.contains(&stem)
            || COMPOUND_TAILS
                .iter()
                .any(|w| stem.len() > w.len() && stem.ends_with(w))
    })
}

/// Whether a texture's name names foliage: one of its words (split at anything but letters)
/// is a foliage word.
fn is_foliage_texture(texture: &str) -> bool {
    let name = Path::new(texture.trim())
        .file_stem()
        .map(|s| s.to_string_lossy().to_lowercase())
        .unwrap_or_default()
        .replace('ä', "ae")
        .replace('ö', "oe")
        .replace('ü', "ue");
    name.split(|c: char| !c.is_ascii_alphabetic())
        .filter(|t| !t.is_empty())
        .any(is_foliage_token)
}

/// Whether a placed object models a tree (or a bush) as a mesh: no `[tree]` billboard, and its
/// cut-out or blended materials (`[matl_alpha]` 1 or 2) all have foliage textures. Its foliage
/// then moves in the wind like the billboards (M-0144 phase C). The texture name is the only
/// hint OMSI's files give: a corpus scan finds 246 such objects next to 941 billboard trees, and
/// the word rule keeps railings (`Lichtentanne_Gelaender`) and signs out. A building with a bush
/// texture next to its window glass (`building401`) is no tree: its glass must not sway.
pub(super) fn is_mesh_tree(sco: &SceneryObject) -> bool {
    if sco.tree.is_some() {
        return false;
    }
    let mut alpha = sco
        .meshes
        .iter()
        .filter(|m| !m.only_editor)
        .flat_map(|m| &m.materials)
        .filter(|t| matches!(t.alpha_mode, Some(1 | 2)))
        .peekable();
    alpha.peek().is_some() && alpha.all(|t| is_foliage_texture(&t.texture))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn foliage_textures_are_told_from_look_alikes() {
        for name in [
            "Laub_01.tga",
            "tree_medium_07_leaves.dds",
            "Tannen.bmp",
            "Kastanienbaum.tga",
            "Birken2.dds",
            "Ziergras.tga",
            "Blätter.bmp",
            "hedge.tga",
        ] {
            assert!(is_foliage_texture(name), "{name}");
        }
        for name in [
            "SK_Bushokje.tga",
            "Lichtentanne_Gelaender.bmp",
            "Verkehrszeichen.tga",
            "Bark01.dds",
            "Glas.tga",
            "Oaktown_Sign.bmp",
            "Uhr_Zifferblatt.tga",
        ] {
            assert!(!is_foliage_texture(name), "{name}");
        }
    }

    fn object(materials: &[(&str, Option<u32>)]) -> SceneryObject {
        use opts_core::formats::scenery::ScoMaterial;
        let mut sco = SceneryObject::default();
        let mut mesh = opts_core::formats::scenery::ScoMesh {
            file: "tree.o3d".into(),
            lod_threshold: None,
            cast_shadow: None,
            materials: Vec::new(),
            visible: None,
            mouse_event: None,
            text_texture_indices: Vec::new(),
            animations: Vec::new(),
            only_editor: false,
        };
        for (i, (tex, alpha)) in materials.iter().enumerate() {
            let mut m = ScoMaterial::new(*tex, i);
            m.alpha_mode = *alpha;
            mesh.materials.push(m);
        }
        sco.meshes.push(mesh);
        sco
    }

    #[test]
    fn an_object_is_a_tree_when_all_its_cut_outs_are_foliage() {
        assert!(is_mesh_tree(&object(&[
            ("Bark10.png", None),
            ("Branch10.png", Some(1))
        ])));
        assert!(!is_mesh_tree(&object(&[
            ("Wand.tga", None),
            ("busch_alpha.png", Some(1)),
            ("Fenster.tga", Some(2)),
        ])));
        assert!(
            !is_mesh_tree(&object(&[("Branch10.png", None)])),
            "opaque: no foliage"
        );
    }
}
