//! `opts-tools inspect <file>...`: parses an OMSI file with the real parser and summarises it.

use std::path::Path;

use opts_core::formats::map::{MapTerrain, MapTile};
use opts_core::formats::oft::OftFile;
use opts_core::formats::scenery::{SceneryObject, SplineProfile};

/// Parses `path` and returns a one-line summary.
pub(crate) fn inspect(path: &Path) -> Result<String, String> {
    let bytes = std::fs::read(path).map_err(|e| format!("read failed: {e}"))?;
    let ext = path
        .extension()
        .map(|e| e.to_string_lossy().to_ascii_lowercase())
        .unwrap_or_default();
    match ext.as_str() {
        "o3d" => {
            let m = opts_core::formats::o3d::parse(&bytes).map_err(|e| e.to_string())?;
            Ok(format!(
                "o3d v{}: {} vertices, {} triangles, {} materials{}",
                m.version,
                m.vertex_count(),
                m.triangle_count(),
                m.material_count(),
                if m.encryption_key.is_some() {
                    ", keyed"
                } else {
                    ""
                }
            ))
        }
        "x" => {
            let m = opts_core::formats::x::parse_x(&bytes).map_err(|e| e.to_string())?;
            Ok(format!(
                "x: {} frames, {} top-level meshes",
                m.frames.len(),
                m.top_level_meshes.len()
            ))
        }
        "sco" => {
            let o = SceneryObject::parse(&bytes).map_err(|e| e.to_string())?;
            Ok(format!(
                "sco: {} meshes, {} text textures",
                o.meshes.len(),
                o.text_textures.len()
            ))
        }
        "sli" => {
            let p = SplineProfile::parse(&bytes).map_err(|e| e.to_string())?;
            Ok(format!(
                "sli: {} graphical profiles, {} height profiles",
                p.graphical_profiles.len(),
                p.height_profiles.len()
            ))
        }
        "oft" => {
            let f = OftFile::parse(&bytes)?;
            let glyphs: usize = f.fonts.iter().map(|x| x.chars.len()).sum();
            Ok(format!("oft: {} fonts, {glyphs} glyphs", f.fonts.len()))
        }
        "terrain" => {
            let t = MapTerrain::parse(&bytes).map_err(|e| e.to_string())?;
            Ok(format!("terrain: {0}x{0} cells", t.grid_cells))
        }
        "map" => {
            let t = MapTile::parse(&bytes).map_err(|e| e.to_string())?;
            Ok(format!(
                "tile: {} objects, {} splines",
                t.objects.len(),
                t.splines.len()
            ))
        }
        other => Err(format!("no parser for extension `.{other}`")),
    }
}
