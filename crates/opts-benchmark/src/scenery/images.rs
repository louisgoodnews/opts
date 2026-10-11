//! Images and repro folders for non-pass objects (or all, on request).
//!
//! `images/<key>/v1_rgb.png` (the object over mid grey), `v1_mask.png` (rendered mask),
//! `v1_expected.png` (expected mask), `v1_diff.png` (white: both, red: rendered only, blue:
//! expected only), the same for `v2`; `repro/<key>/` holds the generated map folder.
//! Everything lands in the run directory, never in the repository.

use std::path::Path;

use image::{Rgb, RgbImage};

use super::framing::IMAGE_SIZE;
use super::object::Analysis;
use super::raster::Mask;
use super::worker::Harness;
use super::{Record, Verdict};

/// Writes images and the repro folder when the keep mode asks for them.
pub fn keep(h: &Harness, record: &mut Record, analysis: &Analysis) {
    let wanted = match h.config.keep_images.as_str() {
        "all" => true,
        "none" => false,
        _ => record.verdict != Verdict::Pass,
    };
    if record.verdict != Verdict::Pass {
        let dest = h.config.run_dir.join("repro").join(&record.key);
        if h.map.copy_to(&dest).is_ok() {
            record.repro = Some(format!(
                "XDG_CONFIG_HOME=$(mktemp -d) opts-game --autostart --map-path {} --omsi-path {} --camera free",
                dest.display(),
                h.config.omsi_root.display()
            ));
        }
    }
    if !wanted {
        return;
    }
    let dir = h.config.run_dir.join("images").join(&record.key);
    if let Err(e) = write_all(&dir, analysis) {
        tracing::warn!(dir = %dir.display(), "cannot write images: {e}");
    }
}

fn write_all(dir: &Path, a: &Analysis) -> image::ImageResult<()> {
    std::fs::create_dir_all(dir)?;
    for (i, (matte, exp)) in a.mattes.iter().zip(&a.expected).enumerate() {
        let v = format!("v{}", i + 1);
        let size = IMAGE_SIZE;
        let rgb = RgbImage::from_fn(size, size, |x, y| {
            let idx = (y * size + x) as usize;
            if matte.mask.px[idx] {
                Rgb(matte.rgb[idx])
            } else {
                Rgb([128, 128, 128])
            }
        });
        rgb.save(dir.join(format!("{v}_rgb.png")))?;
        mask_image(&matte.mask).save(dir.join(format!("{v}_mask.png")))?;
        mask_image(&exp.all).save(dir.join(format!("{v}_expected.png")))?;
        diff_image(&matte.mask, &exp.all).save(dir.join(format!("{v}_diff.png")))?;
    }
    Ok(())
}

fn mask_image(m: &Mask) -> RgbImage {
    RgbImage::from_fn(m.width as u32, m.height as u32, |x, y| {
        if m.get(x as usize, y as usize) {
            Rgb([255, 255, 255])
        } else {
            Rgb([0, 0, 0])
        }
    })
}

fn diff_image(r: &Mask, e: &Mask) -> RgbImage {
    RgbImage::from_fn(r.width as u32, r.height as u32, |x, y| {
        match (r.get(x as usize, y as usize), e.get(x as usize, y as usize)) {
            (true, true) => Rgb([255, 255, 255]),
            (true, false) => Rgb([255, 40, 40]),
            (false, true) => Rgb([40, 90, 255]),
            (false, false) => Rgb([0, 0, 0]),
        }
    })
}
