//! M-0058 phase A measurement: what a mip chain costs for block-compressed (DXT) textures that
//! ship without one, per option of the milestone.
//!
//! ```text
//! cargo run --release -p opts-benchmark --example dxt_mips -- <omsi-root> [sample-size] [--list <file>]
//! ```
//!
//! Walks `<omsi-root>` for `.dds` files, keeps the DXT1/3/5 ones with a single level, and for a
//! deterministic sample (every n-th file in path order, or the files named in `--list`) measures
//! on one thread:
//!
//! * option 1: decode to RGBA8 + generate the sRGB-correct chain (time, VRAM factor);
//! * option 2: re-encode levels 1.. to the source's BC format with `texpresso` (range fit and
//!   cluster fit), time and PSNR of the re-encoded levels against the exact RGBA levels; level 0
//!   stays the original file data.
//!
//! The 2026-10-25 run also measured `intel_tex_2` (results in M-0058); it was dropped from the
//! example because its prebuilt C++ kernels do not link in the dev profile.
//!
//! Prints one summary block (no OMSI content is written anywhere).

use std::path::{Path, PathBuf};
use std::time::Instant;

use bevy::render::render_resource::TextureFormat;
use opts_bevy::texture::{
    dds_to_bevy_image, decode_to_rgba8, full_chain_levels, generate_mip_chain, level_size,
    looks_like_cutout, MipOptions,
};

#[derive(Default)]
struct Totals {
    textures: usize,
    base_pixels: u64,
    chain_pixels: u64,
    decode_s: f64,
    chain_s: f64,
    encode_s: [f64; 2],
    sq_err: [f64; 2],
    /// Alpha error and samples: index 0 = DXT1 with transparent texels, 1 = DXT3/5.
    alpha_sq_err: [[f64; 2]; 2],
    alpha_samples: [u64; 2],
    bc1_with_alpha: usize,
}

const ENCODERS: [&str; 2] = ["texpresso range fit", "texpresso cluster fit"];

fn main() -> anyhow::Result<()> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let root = PathBuf::from(
        args.first()
            .ok_or_else(|| anyhow::anyhow!("usage: dxt_mips <omsi-root> [n] [--list f]"))?,
    );
    let sample: usize = args.get(1).and_then(|s| s.parse().ok()).unwrap_or(400);
    let list = args
        .iter()
        .position(|a| a == "--list")
        .and_then(|i| args.get(i + 1));

    let candidates = pick_candidates(&root, sample, list)?;

    let mut t = Totals::default();
    for path in &candidates {
        measure(&mut t, path);
    }
    report(&t);
    Ok(())
}

/// The files to measure: the ones named in `list`, else every n-th chainless DXT file.
fn pick_candidates(
    root: &Path,
    sample: usize,
    list: Option<&String>,
) -> anyhow::Result<Vec<PathBuf>> {
    Ok(match list {
        Some(file) => std::fs::read_to_string(file)?
            .lines()
            .filter(|l| !l.trim().is_empty())
            .map(PathBuf::from)
            .collect::<Vec<_>>(),
        None => {
            let mut all = Vec::new();
            walk(root, &mut all);
            all.sort();
            let started = Instant::now();
            let chainless: Vec<PathBuf> = all.into_iter().filter(|p| is_chainless_dxt(p)).collect();
            println!(
                "chainless DXT files under the root: {} (header scan {:.1} s)",
                chainless.len(),
                started.elapsed().as_secs_f64()
            );
            let step = (chainless.len() / sample.max(1)).max(1);
            chainless.into_iter().step_by(step).take(sample).collect()
        }
    })
}

/// Decodes one texture, builds its chain and measures the encoders on levels 1..; skips files
/// that are not a single-level DXT1/3/5 texture.
fn measure(t: &mut Totals, path: &Path) {
    let Ok(bytes) = std::fs::read(path) else {
        return;
    };
    let Ok(image) = dds_to_bevy_image(&bytes) else {
        return;
    };
    let format = image.texture_descriptor.format;
    let tex_format = match format {
        TextureFormat::Bc1RgbaUnormSrgb => texpresso::Format::Bc1,
        TextureFormat::Bc2RgbaUnormSrgb => texpresso::Format::Bc2,
        TextureFormat::Bc3RgbaUnormSrgb => texpresso::Format::Bc3,
        _ => return,
    };
    if image.texture_descriptor.mip_level_count != 1 {
        return;
    }
    let (w, h) = (image.width(), image.height());
    if w.max(h) < 2 {
        return;
    }

    let started = Instant::now();
    let Some(mut rgba) = decode_to_rgba8(&image) else {
        return;
    };
    t.decode_s += started.elapsed().as_secs_f64();
    let cutout = rgba.data.as_deref().is_some_and(looks_like_cutout);
    let started = Instant::now();
    generate_mip_chain(
        &mut rgba,
        MipOptions {
            alpha_cutoff: cutout.then_some(0.5),
            weight_colour_by_alpha: false,
        },
    );
    t.chain_s += started.elapsed().as_secs_f64();
    let chain = rgba.data.as_deref().unwrap_or_default();
    let has_alpha = chain[..(w * h * 4) as usize]
        .chunks_exact(4)
        .any(|p| p[3] < 255);
    if tex_format == texpresso::Format::Bc1 && has_alpha {
        t.bc1_with_alpha += 1;
    }
    t.textures += 1;
    t.base_pixels += u64::from(w) * u64::from(h);
    encode_levels(t, tex_format, chain, (w, h), has_alpha);
}

/// Re-encodes levels 1.. of the chain with every encoder and accumulates time and error.
fn encode_levels(
    t: &mut Totals,
    tex_format: texpresso::Format,
    chain: &[u8],
    size: (u32, u32),
    has_alpha: bool,
) {
    let (w, h) = size;
    let bc1 = tex_format == texpresso::Format::Bc1;
    let mut offset = (w * h * 4) as usize;
    for level in 1..full_chain_levels(w, h) {
        let (lw, lh) = level_size(w, h, level);
        let (lw, lh) = (lw as usize, lh as usize);
        let src = &chain[offset..offset + lw * lh * 4];
        offset += lw * lh * 4;
        t.chain_pixels += (lw * lh) as u64;
        for e in 0..ENCODERS.len() {
            let started = Instant::now();
            let params = texpresso::Params {
                algorithm: if e == 0 {
                    texpresso::Algorithm::RangeFit
                } else {
                    texpresso::Algorithm::ClusterFit
                },
                ..Default::default()
            };
            let mut blocks = vec![0u8; tex_format.compressed_size(lw, lh)];
            tex_format.compress(src, lw, lh, params, &mut blocks);
            t.encode_s[e] += started.elapsed().as_secs_f64();
            let mut decoded = vec![0u8; lw * lh * 4];
            tex_format.decompress(&blocks, lw, lh, &mut decoded);
            for (a, b) in src.chunks_exact(4).zip(decoded.chunks_exact(4)) {
                for c in 0..3 {
                    let d = f64::from(a[c]) - f64::from(b[c]);
                    t.sq_err[e] += d * d;
                }
                if has_alpha {
                    let d = f64::from(a[3]) - f64::from(b[3]);
                    t.alpha_sq_err[usize::from(!bc1)][e] += d * d;
                }
            }
        }
        if has_alpha {
            t.alpha_samples[usize::from(!bc1)] += (lw * lh) as u64;
        }
    }
}

fn report(t: &Totals) {
    let mp = t.base_pixels as f64 / 1e6;
    println!(
        "sampled textures: {} ({:.1} Mpx base, {:.1} Mpx in levels 1..)",
        t.textures,
        mp,
        t.chain_pixels as f64 / 1e6
    );
    println!(
        "DXT1 textures with transparent texels: {}",
        t.bc1_with_alpha
    );
    let per = |s: f64| s * 1000.0 / t.textures.max(1) as f64;
    println!(
        "option 1 decode:  {:.2} ms/texture ({:.0} Mpx/s)",
        per(t.decode_s),
        mp / t.decode_s.max(1e-9)
    );
    println!("option 1 chain:   {:.2} ms/texture", per(t.chain_s));
    for (e, name) in ENCODERS.iter().enumerate() {
        let psnr = |sq: f64, n: f64| {
            if sq == 0.0 {
                f64::INFINITY
            } else {
                10.0 * (255.0f64 * 255.0 / (sq / n)).log10()
            }
        };
        println!(
            "option 2 encode, {name}: {:.2} ms/texture ({:.1} MB/s of RGBA input), PSNR rgb {:.2} dB, alpha DXT1 {:.2} dB, alpha DXT3/5 {:.2} dB",
            per(t.encode_s[e]),
            t.chain_pixels as f64 * 4.0 / 1e6 / t.encode_s[e].max(1e-9),
            psnr(t.sq_err[e], t.chain_pixels as f64 * 3.0),
            psnr(t.alpha_sq_err[0][e], t.alpha_samples[0].max(1) as f64),
            psnr(t.alpha_sq_err[1][e], t.alpha_samples[1].max(1) as f64),
        );
    }
}

fn walk(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        match entry.file_type() {
            Ok(ft) if ft.is_dir() => walk(&path, out),
            Ok(ft)
                if ft.is_file()
                    && path
                        .extension()
                        .is_some_and(|e| e.eq_ignore_ascii_case("dds")) =>
            {
                out.push(path);
            }
            _ => {}
        }
    }
}

/// DXT1/3/5 with a mip count of 0 or 1, from the 128-byte header alone.
fn is_chainless_dxt(path: &Path) -> bool {
    use std::io::Read;
    let mut header = [0u8; 128];
    let Ok(mut file) = std::fs::File::open(path) else {
        return false;
    };
    if file.read_exact(&mut header).is_err() || &header[0..4] != b"DDS " {
        return false;
    }
    let flags = u32::from_le_bytes(header[8..12].try_into().unwrap_or_default());
    let mips = u32::from_le_bytes(header[28..32].try_into().unwrap_or_default());
    let fourcc = &header[84..88];
    let has_mips = flags & 0x2_0000 != 0 && mips > 1;
    matches!(fourcc, b"DXT1" | b"DXT3" | b"DXT5") && !has_mips
}
