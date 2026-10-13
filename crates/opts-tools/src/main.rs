//! OPTS asset toolsuite.
//!
//! - `opts-tools inspect <file>...` validates OMSI files with the real parsers. The format is
//!   chosen by file extension (`.o3d`, `.x`, `.sco`, `.sli`, `.oft`, `.terrain`, `.map` tiles).
//!   Each file prints one line; the exit code is 1 if any file failed.
//! - `opts-tools l10n export|check|new|import|stats` prepares, checks and measures language packs
//!   (see [`l10n`] and `docs/src/architecture/localisation-packs.md`).
//! - `opts-tools image-metrics [--crop x,y,w,h] [--json] <a.png> [<b.png>]` measures screenshots
//!   (saturation, luminance, high-frequency energy) and the difference of two of them; see
//!   [`metrics`].

mod l10n;
mod metrics;

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

/// Loads an image as RGB8, optionally cropped.
fn load_rgb(path: &str, crop: Option<metrics::Crop>) -> Result<image::RgbImage, String> {
    let img = image::open(path)
        .map_err(|e| format!("{path}: {e}"))?
        .to_rgb8();
    match crop {
        Some(c) => c.apply(&img).map_err(|e| format!("{path}: {e}")),
        None => Ok(img),
    }
}

/// `image-metrics` with its arguments; prints one block per image and the difference.
fn image_metrics(args: &[String]) -> Result<(), String> {
    let mut crop = None;
    let mut json = false;
    let mut files = Vec::new();
    let mut it = args.iter();
    while let Some(a) = it.next() {
        match a.as_str() {
            "--crop" => {
                let v = it.next().ok_or("--crop needs x,y,w,h")?;
                crop = Some(metrics::Crop::parse(v)?);
            }
            "--json" => json = true,
            f => files.push(f),
        }
    }
    if files.is_empty() || files.len() > 2 {
        return Err("expected one or two image files".into());
    }
    let images: Vec<_> = files
        .iter()
        .map(|f| load_rgb(f, crop))
        .collect::<Result<_, _>>()?;
    let mut out = Vec::new();
    for (f, img) in files.iter().zip(&images) {
        let m = metrics::measure(img);
        if json {
            out.push(format!(
                "{{\"file\":{f:?},\"pixels\":{},\"saturation\":{:.4},\"luma_mean\":{:.4},\"luma_std\":{:.4},\"luma_min\":{:.4},\"laplacian\":{:.5}}}",
                m.pixels, m.saturation, m.luma_mean, m.luma_std, m.luma_min, m.laplacian
            ));
        } else {
            println!(
                "{f}\n  pixels {}\n  saturation {:.4}\n  luma mean {:.4}  std {:.4}  min {:.4}\n  laplacian {:.5}",
                m.pixels, m.saturation, m.luma_mean, m.luma_std, m.luma_min, m.laplacian
            );
        }
    }
    if let [a, b] = &images[..] {
        let d = metrics::difference(a, b)?;
        if json {
            out.push(format!(
                "{{\"difference\":{{\"mean_abs\":{:.5},\"rmse\":{:.5},\"psnr\":{}}}}}",
                d.mean_abs,
                d.rmse,
                if d.psnr.is_finite() {
                    format!("{:.2}", d.psnr)
                } else {
                    "null".into()
                }
            ));
        } else {
            println!(
                "difference\n  mean abs {:.5}  rmse {:.5}  psnr {:.2} dB",
                d.mean_abs, d.rmse, d.psnr
            );
        }
    }
    if json {
        println!("[{}]", out.join(","));
    }
    Ok(())
}

const USAGE: &str = "usage: opts-tools inspect <file>...\n       opts-tools image-metrics [--crop x,y,w,h] [--json] <a.png> [<b.png>]\n       opts-tools l10n export|check|new|import|stats ... (see `opts-tools l10n`)";

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let Some((cmd, files)) = args.split_first() else {
        eprintln!("{USAGE}");
        return ExitCode::from(2);
    };
    if cmd == "l10n" {
        let o = l10n::execute(files);
        print!("{}", o.out);
        eprint!("{}", o.err);
        return ExitCode::from(u8::try_from(o.code).unwrap_or(2));
    }
    if cmd == "image-metrics" {
        return match image_metrics(files) {
            Ok(()) => ExitCode::SUCCESS,
            Err(e) => {
                eprintln!("image-metrics: {e}\n{USAGE}");
                ExitCode::from(2)
            }
        };
    }
    if cmd != "inspect" || files.is_empty() {
        eprintln!("{USAGE}");
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
