//! Developer tool: scans an OMSI installation's textures and reports how many DDS chains the
//! loader drops (Q-TQ03) and what generating mip chains costs.
//!
//! `cargo run --release -p opts-bevy --example texture_scan -- <dir> [--limit N] [--generate]`
//!
//! Reads files only; nothing is written. With `--generate` every decoded uncompressed texture
//! without a chain gets one (as the loader does) and the time is totalled.

use std::path::{Path, PathBuf};
use std::time::Instant;

use opts_bevy::texture::{generate_mip_chain, load_bevy_image, mip_stats, MipOptions};

fn walk(dir: &Path, out: &mut Vec<PathBuf>) {
    for entry in std::fs::read_dir(dir).into_iter().flatten().flatten() {
        let p = entry.path();
        if p.is_dir() {
            walk(&p, out);
        } else if p.extension().is_some_and(|e| {
            ["dds", "bmp", "tga", "jpg", "jpeg", "png"]
                .iter()
                .any(|x| e.eq_ignore_ascii_case(x))
        }) {
            out.push(p);
        }
    }
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let Some(root) = args.first() else {
        eprintln!("usage: texture_scan <dir> [--limit N] [--generate]");
        return;
    };
    let limit = args
        .iter()
        .position(|a| a == "--limit")
        .and_then(|i| args.get(i + 1))
        .and_then(|v| v.parse::<usize>().ok())
        .unwrap_or(usize::MAX);
    let generate = args.iter().any(|a| a == "--generate");
    let mut files = Vec::new();
    walk(Path::new(root), &mut files);
    files.sort();
    files.truncate(limit);

    let (mut decoded, mut failed, mut dropped, mut declared_chains) = (0u64, 0u64, 0u64, 0u64);
    let (mut decode_time, mut gen_time, mut gen_count, mut gen_bytes) =
        (0.0f64, 0.0f64, 0u64, 0u64);
    let mut dropped_names = Vec::new();
    for f in &files {
        let Ok(bytes) = std::fs::read(f) else {
            continue;
        };
        let declared = (bytes.starts_with(b"DDS ") && bytes.len() > 32)
            .then(|| u32::from_le_bytes([bytes[28], bytes[29], bytes[30], bytes[31]]))
            .unwrap_or(0);
        let t = Instant::now();
        let image = load_bevy_image(&bytes);
        decode_time += t.elapsed().as_secs_f64();
        let Ok(mut image) = image else {
            failed += 1;
            continue;
        };
        decoded += 1;
        if declared > 1 {
            declared_chains += 1;
            if image.texture_descriptor.mip_level_count <= 1 {
                dropped += 1;
                if dropped_names.len() < 10 {
                    dropped_names.push(f.display().to_string());
                }
            }
        }
        if generate {
            let t = Instant::now();
            let size = image.data.as_ref().map_or(0, Vec::len);
            if generate_mip_chain(&mut image, MipOptions::default()) {
                gen_time += t.elapsed().as_secs_f64();
                gen_count += 1;
                gen_bytes += size as u64;
            }
        }
    }
    println!("files {} decoded {decoded} failed {failed}", files.len());
    println!("decode time {decode_time:.2} s");
    println!("DDS with a declared chain {declared_chains}, chain dropped by the loader {dropped}");
    for n in dropped_names {
        println!("  e.g. {n}");
    }
    if generate {
        let ms = if gen_count > 0 {
            gen_time * 1000.0 / gen_count as f64
        } else {
            0.0
        };
        println!(
            "generated chains {gen_count} in {gen_time:.2} s = {ms:.2} ms per texture ({:.1} MB base data, {:.0} MB/s); stats {:?}",
            gen_bytes as f64 / 1e6,
            gen_bytes as f64 / 1e6 / gen_time.max(1e-9),
            mip_stats()
        );
    }
}
