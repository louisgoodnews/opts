//! Offline inventory of repeated mesh and material usage per map (M-0082, mesh instancing research).
//!
//! For every map named on the command line the example reads all `tile_*.map` files, resolves each
//! placed `.sco` and its `.o3d` meshes (header data only: material slots, per-face material ids)
//! and counts how often the same mesh part and material repeat. It builds no Bevy assets, so it
//! needs no GPU and runs over the whole corpus.
//!
//! `cargo run --release -p opts-bevy --example instancing_inventory -- <OMSI dir> <map dir name>...`
//!
//! Environment: `MAX_TILES` (default 100000) limits the tiles read per map; `TOP` (default 8) the
//! number of most repeated objects listed.
//!
//! Terms (see `docs/src/architecture/instancing-research.md`):
//! * **part** — one Bevy `Mesh3d + MeshMaterial3d` entity: a mesh file's material slot with
//!   triangles, per placed object (and per LOD mesh).
//! * **current handles** — the game caches parts per (`.o3d` path, `[matl]` overrides), so equal
//!   keys already share one mesh handle and one material handle; objects with a rendered text
//!   texture never share.
//! * **content key** — what a material really looks like: resolved texture name, the `.o3d`
//!   material numbers, alpha mode, light map. Different keys cannot share a material handle.
use std::collections::hash_map::DefaultHasher;
use std::collections::{BTreeSet, HashMap, HashSet};
use std::hash::{Hash, Hasher};
use std::path::{Path, PathBuf};

use opts_bevy::OmsiInstall;
use opts_core::formats::map::MapTile;
use opts_core::formats::scenery::SceneryObject;

/// One material slot of a mesh file that has triangles.
#[derive(Clone)]
struct Slot {
    texture: String,
    /// `.o3d` material numbers, quantised.
    numbers: [i32; 11],
}

struct MeshInfo {
    /// Hash of the file bytes (equal for the same mesh copied between add-ons).
    content: u64,
    slots: Vec<Slot>,
    triangles: usize,
    vertices: usize,
    scrambled: bool,
}

struct ScoInfo {
    sco: SceneryObject,
    /// `[mesh]` number → resolved mesh path (None when missing).
    meshes: Vec<Option<PathBuf>>,
    /// Needs the runtime hierarchy (scripts, lights, animation, visibility, LOD, light-map variable).
    dynamic: bool,
    /// Only LOD groups, nothing driven by variables.
    lod_only: bool,
    text_textures: bool,
}

fn base_name(p: &str) -> String {
    p.replace('\\', "/")
        .rsplit('/')
        .next()
        .unwrap_or("")
        .to_lowercase()
}

fn q(v: f32) -> i32 {
    (v * 1000.0).round() as i32
}

fn mesh_info(path: &Path) -> Option<MeshInfo> {
    let bytes = std::fs::read(path).ok()?;
    let mut h = DefaultHasher::new();
    bytes.hash(&mut h);
    let models = opts_bevy::mesh::parse_mesh_file(path, &bytes).ok()?;
    let mut slots = Vec::new();
    let (mut triangles, mut vertices) = (0, 0);
    let mut scrambled = false;
    for m in &models {
        triangles += m.indices.len();
        vertices += m.vertices.len();
        scrambled |= matches!(
            m.vertex_layout(),
            opts_core::formats::o3d::VertexLayout::Unreadable
        );
        let unreadable = matches!(
            m.vertex_layout(),
            opts_core::formats::o3d::VertexLayout::Unreadable
        );
        // Scrambled files: the per-face material ids are not trusted; assume every table entry
        // is used (the decoder of the game recovers the real split).
        let used: BTreeSet<usize> = if unreadable {
            (0..m.materials.len().max(1)).collect()
        } else {
            (0..m.indices.len())
                .map(|i| m.face_materials.get(i).copied().unwrap_or(0) as usize)
                .collect()
        };
        for s in used {
            let (texture, numbers) = match m.materials.get(s) {
                Some(mat) => (
                    mat.primary_texture().map(base_name).unwrap_or_default(),
                    [
                        q(mat.diffuse[0]),
                        q(mat.diffuse[1]),
                        q(mat.diffuse[2]),
                        q(mat.diffuse[3]),
                        q(mat.specular[0]),
                        q(mat.specular[1]),
                        q(mat.specular[2]),
                        q(mat.emissive[0]),
                        q(mat.emissive[1]),
                        q(mat.emissive[2]),
                        q(mat.power),
                    ],
                ),
                None => (String::new(), [1000; 11]),
            };
            slots.push(Slot { texture, numbers });
        }
    }
    Some(MeshInfo {
        content: h.finish(),
        slots,
        triangles,
        vertices,
        scrambled,
    })
}

#[derive(Default)]
struct Totals {
    tiles: usize,
    objects: usize,
    tree_objects: usize,
    vehicle_objects: usize,
    no_mesh: usize,
    missing_sco: usize,
    parts: usize,
    parts_flat: usize,
    parts_lod_only: usize,
    parts_dynamic: usize,
    parts_text: usize,
    parts_trees: usize,
    triangles: u64,
    scrambled_parts: usize,
    // distinct sets
    mesh_paths: HashSet<PathBuf>,
    mesh_content: HashSet<u64>,
    /// (mesh path, [matl] overrides) → parts it creates: the game's current handle keys.
    handle_keys: HashMap<(PathBuf, String), usize>,
    /// distinct material content keys
    material_keys: HashSet<String>,
    /// distinct (mesh content, slot index, material content key) = lower bound of batches
    pair_keys: HashSet<(u64, usize, String)>,
    /// same, only for flat static parts
    pair_keys_flat: HashSet<(u64, usize, String)>,
    /// parts per sco path
    per_sco: HashMap<String, (usize, usize)>,
    /// per tile: (parts, distinct pairs)
    per_tile: Vec<(usize, usize)>,
}

#[path = "instancing_inventory/report.rs"]
mod report;
use report::report;

fn main() {
    let mut args = std::env::args().skip(1);
    let Some(omsi) = args.next() else {
        eprintln!("usage: instancing_inventory <OMSI dir> <map dir name>...");
        return;
    };
    let maps: Vec<String> = args.collect();
    let max_tiles: usize = std::env::var("MAX_TILES")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(100_000);
    let top: usize = std::env::var("TOP")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(8);
    let install = OmsiInstall::new(&omsi);
    let mut sco_cache: HashMap<PathBuf, Option<ScoInfo>> = HashMap::new();
    let mut mesh_cache: HashMap<PathBuf, Option<MeshInfo>> = HashMap::new();

    for map in &maps {
        let dir = Path::new(&omsi).join("maps").join(map);
        let mut tiles: Vec<PathBuf> = std::fs::read_dir(&dir)
            .into_iter()
            .flatten()
            .flatten()
            .map(|e| e.path())
            .filter(|p| {
                let n = p.file_name().map(|n| n.to_string_lossy().to_lowercase());
                n.is_some_and(|n| n.starts_with("tile_") && n.ends_with(".map"))
            })
            .collect();
        tiles.sort();
        tiles.truncate(max_tiles);
        let mut t = Totals::default();
        for tile_path in &tiles {
            let Ok(bytes) = std::fs::read(tile_path) else {
                continue;
            };
            let Ok(tile) = MapTile::parse(&bytes) else {
                continue;
            };
            t.tiles += 1;
            let mut tile_parts = 0;
            let mut tile_pairs: HashSet<(u64, usize, String)> = HashSet::new();
            for obj in &tile.objects {
                t.objects += 1;
                let Some(sco_path) = install.find_file(&obj.sco_file) else {
                    t.missing_sco += 1;
                    continue;
                };
                let info = sco_cache.entry(sco_path.clone()).or_insert_with(|| {
                    let sco = SceneryObject::parse(&std::fs::read(&sco_path).ok()?).ok()?;
                    let sco_dir = sco_path.parent()?.to_path_buf();
                    let model_dir = install
                        .find_in(&sco_dir, "model")
                        .unwrap_or_else(|| sco_dir.join("model"));
                    let meshes = sco
                        .meshes
                        .iter()
                        .map(|m| install.find_in(&model_dir, &m.file))
                        .collect();
                    let driven = !sco.scripts.is_empty()
                        || !sco.lights.is_empty()
                        || sco.meshes.iter().any(|m| {
                            m.visible.is_some()
                                || !m.animations.is_empty()
                                || m.materials.iter().any(|x| {
                                    x.light_map.is_some() && x.light_map_variable.is_some()
                                })
                        });
                    let lod = sco.meshes.iter().any(|m| m.lod_threshold.is_some());
                    let text_textures = sco
                        .meshes
                        .iter()
                        .any(|m| m.materials.iter().any(|x| x.text_texture.is_some()));
                    Some(ScoInfo {
                        sco,
                        meshes,
                        dynamic: driven,
                        lod_only: lod && !driven,
                        text_textures,
                    })
                });
                let Some(info) = info.as_ref() else {
                    t.missing_sco += 1;
                    continue;
                };
                if let Some(tree) = &info.sco.tree {
                    // One billboard part per tree: its own mesh (height, width) and material
                    // handle today; the content key is the texture.
                    t.tree_objects += 1;
                    let texture = obj
                        .string_vars
                        .first()
                        .map(|s| s.trim())
                        .filter(|s| !s.is_empty())
                        .unwrap_or(&tree.texture)
                        .to_lowercase();
                    let content = format!("tree|{texture}");
                    t.material_keys.insert(content.clone());
                    let mut h = DefaultHasher::new();
                    "tree-billboard".hash(&mut h);
                    let pair = (h.finish(), 0, content);
                    tile_pairs.insert(pair.clone());
                    t.pair_keys_flat.insert(pair.clone());
                    t.pair_keys.insert(pair);
                    t.parts += 1;
                    t.parts_flat += 1;
                    t.parts_trees += 1;
                    tile_parts += 1;
                    continue;
                }
                if info.sco.model_cfg.is_some() || info.sco.carpark_p {
                    t.vehicle_objects += 1;
                    continue;
                }
                if info.sco.meshes.is_empty() {
                    t.no_mesh += 1;
                    continue;
                }
                let mut obj_parts = 0;
                for (mi, mesh_def) in info.sco.meshes.iter().enumerate() {
                    let Some(path) = info.meshes[mi].clone() else {
                        continue;
                    };
                    let mesh = mesh_cache
                        .entry(path.clone())
                        .or_insert_with(|| mesh_info(&path));
                    let Some(mesh) = mesh.as_ref() else { continue };
                    t.mesh_paths.insert(path.clone());
                    t.mesh_content.insert(mesh.content);
                    let overrides = format!("{:?}", mesh_def.materials);
                    let key = (path.clone(), overrides);
                    let entry = t.handle_keys.entry(key).or_insert(0);
                    *entry = (*entry).max(mesh.slots.len());
                    for (si, slot) in mesh.slots.iter().enumerate() {
                        // The `[matl]` entry of this slot changes texture, alpha and light map.
                        let over = mesh_def.materials.iter().find(|m| {
                            base_name(&m.texture) == slot.texture || m.material_index == si
                        });
                        let content = format!(
                            "{}|{:?}|alpha={:?}|lm={:?}/{:?}|tm={}|tt={:?}",
                            over.map_or(slot.texture.clone(), |o| {
                                let t = base_name(&o.texture);
                                if t.is_empty() {
                                    slot.texture.clone()
                                } else {
                                    t
                                }
                            }),
                            slot.numbers,
                            over.and_then(|o| o.alpha_mode),
                            over.and_then(|o| o.light_map.as_deref().map(base_name)),
                            over.and_then(|o| o.light_map_variable.as_deref()),
                            over.is_some_and(|o| o.trans_map),
                            over.and_then(|o| o.text_texture),
                        );
                        t.material_keys.insert(content.clone());
                        let pair = (mesh.content, si, content);
                        tile_pairs.insert(pair.clone());
                        if !info.dynamic && !info.lod_only {
                            t.pair_keys_flat.insert(pair.clone());
                        }
                        t.pair_keys.insert(pair);
                        t.parts += 1;
                        obj_parts += 1;
                        t.triangles += (mesh.triangles / mesh.slots.len().max(1)) as u64;
                        if mesh.scrambled {
                            t.scrambled_parts += 1;
                        }
                        if info.text_textures {
                            t.parts_text += 1;
                        }
                        if info.dynamic {
                            t.parts_dynamic += 1;
                        } else if info.lod_only {
                            t.parts_lod_only += 1;
                        } else {
                            t.parts_flat += 1;
                        }
                    }
                    let _ = mesh.vertices;
                }
                tile_parts += obj_parts;
                let e = t.per_sco.entry(obj.sco_file.clone()).or_default();
                e.0 += 1;
                e.1 += obj_parts;
            }
            t.per_tile.push((tile_parts, tile_pairs.len()));
        }
        report(map, &t, top);
    }
}
