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

mod candidates;
mod measure;
mod report;

use std::path::PathBuf;

use candidates::pick_candidates;
use measure::measure;
use report::report;

#[derive(Default)]
pub(crate) struct Totals {
    pub(crate) textures: usize,
    pub(crate) base_pixels: u64,
    pub(crate) chain_pixels: u64,
    pub(crate) decode_s: f64,
    pub(crate) chain_s: f64,
    pub(crate) encode_s: [f64; 2],
    pub(crate) sq_err: [f64; 2],
    /// Alpha error and samples: index 0 = DXT1 with transparent texels, 1 = DXT3/5.
    pub(crate) alpha_sq_err: [[f64; 2]; 2],
    pub(crate) alpha_samples: [u64; 2],
    pub(crate) bc1_with_alpha: usize,
}

pub(crate) const ENCODERS: [&str; 2] = ["texpresso range fit", "texpresso cluster fit"];

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
