//! Parsing a mesh file (`.o3d` or DirectX `.x`) into the `.o3d` models the loaders share.

use super::frames::collect_frame_submeshes;
use super::*;

/// Parses an `.o3d` or DirectX `.x` mesh file into one or more [`O3d`] models.
/// Shared by the asset loaders and the runtime content loader.
///
/// `.x` sub-meshes are normalised into the `.o3d` representation so both formats share the same
/// handedness conversion and material handling.
pub fn parse_mesh_file(path: &std::path::Path, bytes: &[u8]) -> Result<Vec<O3d>, String> {
    let is_x = path
        .extension()
        .is_some_and(|e| e.to_string_lossy().eq_ignore_ascii_case("x"));
    if !is_x {
        return opts_core::formats::o3d::parse(bytes)
            .map(|m| vec![m])
            .map_err(|e| e.to_string());
    }
    let x = opts_core::formats::x::parse_x(bytes).map_err(|e| e.to_string())?;
    let mut subs: Vec<(&opts_core::formats::x::XSubMesh, Option<[[f32; 4]; 4]>)> =
        x.top_level_meshes.iter().map(|m| (m, None)).collect();
    for frame in &x.frames {
        collect_frame_submeshes(frame, None, &mut subs);
    }
    // One file-wide material palette: the sub-meshes' material lists are concatenated in file
    // order (as a loader that collapses the frame hierarchy into one mesh does) and each
    // sub-mesh's slots are offset into it, so `[matl]` entries (which name a native texture and
    // its occurrence) see the materials of the whole file, as for an `.o3d`. With per-sub-mesh
    // palettes an override for the text quad of a two-frame street sign also matched the plate's
    // slot 0 through the slot-index fallback of `find_override` (Berlin-Spandau report, System
    // 2 item 3). Equal materials of different sub-meshes stay separate slots:
    // `Ruede/Tree_Street_01.sco` gives `[matl_alpha] 1` to occurrence 1 of `Tree_Street_01.tga`,
    // which is the material of the file's third frame (`Plane`), not of the second (`Tube`).
    let mut palette: Vec<&opts_core::formats::x::XMaterial> = Vec::new();
    let mut remaps: Vec<Vec<u16>> = Vec::with_capacity(subs.len());
    for (sub, _) in &subs {
        let local = if sub.materials.is_empty() {
            &x.global_materials
        } else {
            &sub.materials
        };
        let offset = palette.len();
        palette.extend(local.iter());
        remaps.push(
            (0..local.len())
                .map(|i| u16::try_from(offset + i).unwrap_or(u16::MAX))
                .collect(),
        );
    }
    let materials: Vec<opts_core::formats::o3d::Material> = palette
        .iter()
        .map(|m| opts_core::formats::o3d::Material {
            diffuse: m.diffuse,
            specular: m.specular,
            emissive: m.emissive,
            power: m.power,
            textures: m.texture_filename.clone().into_iter().collect(),
        })
        .collect();
    Ok(subs
        .into_iter()
        .zip(remaps)
        .map(|((sub, transform), remap)| O3d {
            version: 0,
            header_flags: 0,
            vertices: sub
                .vertices
                .iter()
                .map(|v| opts_core::formats::o3d::Vertex {
                    position: v.position,
                    normal: v.normal,
                    uv: v.uv,
                    color: None,
                })
                .collect(),
            indices: sub.indices.clone(),
            // A slot outside the sub-mesh's own list stays out of the palette (as before).
            face_materials: sub
                .face_materials
                .iter()
                .map(|&m| {
                    remap.get(m as usize).copied().unwrap_or_else(|| {
                        u16::try_from(materials.len() + m as usize).unwrap_or(u16::MAX)
                    })
                })
                .collect(),
            materials: materials.clone(),
            transform,
            unknown_chunks: Vec::new(),
            encryption_key: None,
            exact_decoded: false,
        })
        .collect())
}
