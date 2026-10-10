//! OPTS asset toolsuite: `opts-tools inspect <file>...` validates OMSI files with the real parsers.
//!
//! The format is chosen by file extension (`.o3d`, `.x`, `.sco`, `.sli`, `.oft`, `.terrain`,
//! `.map` tiles). Each file prints one line; the exit code is 1 if any file failed.

use std::path::Path;
use std::process::ExitCode;

use opts_core::formats::map::{MapTerrain, MapTile};
use opts_core::formats::oft::OftFile;
use opts_core::formats::scenery::{SceneryObject, SplineProfile};

/// Parses `path` and returns a one-line summary.
fn inspect(path: &Path) -> Result<String, String> {
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

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let Some((cmd, files)) = args.split_first() else {
        eprintln!("usage: opts-tools inspect <file>...");
        return ExitCode::from(2);
    };
    if cmd != "inspect" || files.is_empty() {
        eprintln!("usage: opts-tools inspect <file>...");
        return ExitCode::from(2);
    }
    let mut failed = 0;
    for f in files {
        match inspect(Path::new(f)) {
            Ok(summary) => println!("OK    {f}: {summary}"),
            Err(e) => {
                failed += 1;
                println!("FAIL  {f}: {e}");
            }
        }
    }
    if failed == 0 {
        ExitCode::SUCCESS
    } else {
        ExitCode::FAILURE
    }
}
