//! The expected geometry: the triangles OPTS should draw for an object, in the map frame.
//!
//! Meshes are read with OPTS's own parsers and interpreted the way the loader documents it
//! (axis-cycled layout, `.x` frame transforms, `[matl]` overrides, `[alphascale]` materials
//! hidden, keyed meshes decoded exactly). The **placement** (heading
//! clockwise from north, object +Z forward, +X right, +Y up) is computed here independently of
//! the game (`docs/src/architecture/coordinates.md`).

use std::path::Path;

use bevy::math::Vec3;
use opts_bevy::model::find_override;
use opts_bevy::{OmsiInstall, TextureOverride};
use opts_core::formats::o3d::{O3d, VertexLayout};
use opts_core::formats::scenery::{SceneryObject, ScoMesh};

/// Where the object stands: map-frame position and heading (degrees, clockwise from north).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Placement {
    /// Map-frame position (x east, y north, z up).
    pub position: Vec3,
    /// Heading in degrees, clockwise from north.
    pub heading_deg: f32,
    /// Mirror the object's x axis (the rotation search).
    pub mirror_x: bool,
    /// Uniform scale of the object (1; fault seeding uses other values).
    pub scale: f32,
}

impl Placement {
    /// Object space (x right, y up, z forward) to the map frame.
    #[must_use]
    pub fn to_map(&self, p: Vec3) -> Vec3 {
        let (s, c) = self.heading_deg.to_radians().sin_cos();
        let p = p * self.scale;
        let x = if self.mirror_x { -p.x } else { p.x };
        Vec3::new(
            self.position.x + x * c + p.z * s,
            self.position.y - x * s + p.z * c,
            self.position.z + p.y,
        )
    }
}

/// Per-triangle properties that select the masks it belongs to.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct TriFlags {
    /// Part of `E_base` (no `[visible]`, highest-detail LOD group or no LOD).
    pub base: bool,
    /// Drawn alpha-tested or blended (coverage depends on the texture).
    pub alpha: bool,
    /// Carries a text texture.
    pub text: bool,
    /// Drawn from both sides (tree billboards); every other triangle is back-face culled.
    pub two_sided: bool,
}

/// One expected triangle in object space.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ExpectedTri {
    /// Vertices in object space (x right, y up, z forward).
    pub v: [Vec3; 3],
    /// Mask membership.
    pub flags: TriFlags,
    /// Index of the `[mesh]` of the `.sco` the triangle belongs to (0 for trees).
    pub mesh: usize,
}

/// Keyed (scrambled) meshes and how they were resolved.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct KeyedSummary {
    /// Keyed meshes referenced.
    pub total: usize,
    /// Decoded exactly (`o3d::exact`).
    pub decoded: usize,
    /// Not resolvable (the generator does not cover the file): no expected geometry for them.
    pub unresolved: usize,
}

/// Expected geometry of one object.
#[derive(Debug, Clone, Default)]
pub struct Expected {
    /// Triangles (object space).
    pub tris: Vec<ExpectedTri>,
    /// Keyed meshes.
    pub keyed: KeyedSummary,
    /// Distinct `[LOD]` groups.
    pub lod_groups: usize,
    /// Load-stage categories found while reading meshes.
    pub categories: Vec<String>,
}

/// Builds the expected triangles of `sco` (file at `sco_path`).
#[must_use]
pub fn build(install: &OmsiInstall, sco_path: &Path, sco: &SceneryObject) -> Expected {
    let mut out = Expected::default();
    if let Some(tree) = &sco.tree {
        let h = (tree.min_height + tree.max_height) * 0.5;
        let w = h * (tree.min_width_ratio + tree.max_width_ratio) * 0.5;
        out.tris = tree_billboard(h, w);
        return out;
    }
    if sco.is_editor_only() || sco.carpark_p || sco.model_cfg.is_some() {
        return out;
    }
    let sco_dir = sco_path.parent().unwrap_or(Path::new("."));
    let model_dir = install
        .find_in(sco_dir, "model")
        .unwrap_or_else(|| sco_dir.join("model"));
    let lods: Vec<f32> = sco.meshes.iter().filter_map(|m| m.lod_threshold).collect();
    let top_lod = lods
        .iter()
        .copied()
        .fold(None, |a: Option<f32>, b| Some(a.map_or(b, |a| a.max(b))));
    let mut distinct = lods.clone();
    distinct.sort_by(f32::total_cmp);
    distinct.dedup();
    out.lod_groups = distinct.len();
    for (mesh_index, mesh) in sco.meshes.iter().enumerate() {
        if !sco.mesh_in_game(mesh_index) {
            continue;
        }
        let Some(path) = install.find_in(&model_dir, &mesh.file) else {
            push_unique(&mut out.categories, "F-MESH-MISSING");
            // The loader gives up on the whole object at the first missing mesh.
            out.tris.clear();
            return out;
        };
        let base = mesh.visible.is_none() && mesh.lod_threshold.is_none_or(|l| Some(l) == top_lod);
        let models = match read_models(&path, &mut out.keyed) {
            Ok(m) => m,
            Err(()) => {
                push_unique(&mut out.categories, "F-MESH-PARSE");
                continue;
            }
        };
        let overrides = overrides(sco, mesh);
        for model in &models {
            add_model(&mut out.tris, model, mesh_index, &overrides, base);
        }
    }
    out
}

fn push_unique(v: &mut Vec<String>, s: &str) {
    if !v.iter().any(|x| x == s) {
        v.push(s.to_string());
    }
}

/// Parses a mesh file and resolves keyed models; unresolved keyed models are dropped.
fn read_models(path: &Path, keyed: &mut KeyedSummary) -> Result<Vec<O3d>, ()> {
    let bytes = std::fs::read(path).map_err(|_| ())?;
    let mut models = opts_bevy::mesh::parse_mesh_file(path, &bytes).map_err(|_| ())?;
    let unreadable = |ms: &[O3d]| {
        ms.iter()
            .any(|m| matches!(m.vertex_layout(), VertexLayout::Unreadable))
    };
    if !unreadable(&models) {
        return Ok(models);
    }
    keyed.total += 1;
    for m in models.iter_mut() {
        if m.encryption_key.is_some() {
            opts_core::formats::o3d::exact::decode(m);
        }
    }
    if unreadable(&models) {
        keyed.unresolved += 1;
        models.retain(|m| !matches!(m.vertex_layout(), VertexLayout::Unreadable));
    } else {
        keyed.decoded += 1;
    }
    Ok(models)
}

/// The `[matl]` overrides of a mesh, as the tile loader builds them (text images left out).
fn overrides(sco: &SceneryObject, mesh: &ScoMesh) -> Vec<(TextureOverride, bool)> {
    mesh.materials
        .iter()
        .map(|m| {
            let text = m
                .text_texture
                .is_some_and(|i| sco.text_textures.get(i).is_some());
            (
                TextureOverride {
                    slot: m.material_index,
                    texture: m.texture.clone(),
                    alpha: m.alpha_mode,
                    alpha_variable: m.alpha_scale.clone(),
                    trans_map: None,
                    image: None,
                    light_map: None,
                    free_texture: None,
                    night_map: None,
                },
                text,
            )
        })
        .collect()
}

fn add_model(
    tris: &mut Vec<ExpectedTri>,
    model: &O3d,
    mesh: usize,
    overrides: &[(TextureOverride, bool)],
    base: bool,
) {
    let plain: Vec<TextureOverride> = overrides.iter().map(|(o, _)| o.clone()).collect();
    let cycled = matches!(model.vertex_layout(), VertexLayout::AxisCycled);
    let frame = (model.version == 0).then_some(model.transform).flatten();
    let object_space = |p: Vec3| {
        let raw = if cycled { Vec3::new(-p.y, p.x, p.z) } else { p };
        frame.map_or(raw, |m| {
            Vec3::new(
                raw.x * m[0][0] + raw.y * m[1][0] + raw.z * m[2][0] + m[3][0],
                raw.x * m[0][1] + raw.y * m[1][1] + raw.z * m[2][1] + m[3][1],
                raw.x * m[0][2] + raw.y * m[1][2] + raw.z * m[2][2] + m[3][2],
            )
        })
    };
    for (i, face) in model.indices.iter().enumerate() {
        let slot = model.face_materials.get(i).copied().unwrap_or(0) as usize;
        let over = find_override(&model.materials, slot, &plain);
        if over.is_some_and(TextureOverride::is_variable_hidden) {
            continue;
        }
        let text = over
            .and_then(|o| plain.iter().position(|p| std::ptr::eq(p, o)))
            .is_some_and(|i| overrides[i].1);
        let diffuse_alpha = model.materials.get(slot).map_or(1.0, |m| m.diffuse[3]);
        // `[matl_alpha] 0` is opaque (sco.md); text textures cover only their glyphs.
        let alpha =
            over.and_then(|o| o.alpha).is_some_and(|a| a != 0) || diffuse_alpha < 0.999 || text;
        let mut v = [Vec3::ZERO; 3];
        let mut ok = true;
        for (k, &idx) in face.iter().enumerate() {
            match model.vertices.get(idx as usize) {
                Some(vx) => v[k] = object_space(vx.position),
                None => ok = false,
            }
        }
        if ok {
            tris.push(ExpectedTri {
                v,
                flags: TriFlags {
                    base,
                    alpha,
                    text,
                    two_sided: false,
                },
                mesh,
            });
        }
    }
}

/// Two crossed vertical quads of `height` × `width`, standing on the origin (alpha-tested).
fn tree_billboard(height: f32, width: f32) -> Vec<ExpectedTri> {
    let hw = width * 0.5;
    let quad = |a: Vec3, b: Vec3, c: Vec3, d: Vec3| {
        let flags = TriFlags {
            base: true,
            alpha: true,
            text: false,
            two_sided: true,
        };
        [
            ExpectedTri {
                v: [a, b, c],
                flags,
                mesh: 0,
            },
            ExpectedTri {
                v: [a, c, d],
                flags,
                mesh: 0,
            },
        ]
    };
    let mut out = Vec::new();
    out.extend(quad(
        Vec3::new(-hw, 0.0, 0.0),
        Vec3::new(hw, 0.0, 0.0),
        Vec3::new(hw, height, 0.0),
        Vec3::new(-hw, height, 0.0),
    ));
    out.extend(quad(
        Vec3::new(0.0, 0.0, -hw),
        Vec3::new(0.0, 0.0, hw),
        Vec3::new(0.0, height, hw),
        Vec3::new(0.0, height, -hw),
    ));
    out
}

#[cfg(test)]
mod tests;
