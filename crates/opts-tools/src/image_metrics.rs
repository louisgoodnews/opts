//! `opts-tools image-metrics`: measures screenshots and the difference of two of them.

use super::metrics;

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
pub(crate) fn image_metrics(args: &[String]) -> Result<(), String> {
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
