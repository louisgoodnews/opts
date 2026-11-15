//! The files to measure: the files named in a list, or a deterministic sample of the chainless DXT files.

use std::path::{Path, PathBuf};
use std::time::Instant;

/// The files to measure: the ones named in `list`, else every n-th chainless DXT file.
pub(crate) fn pick_candidates(
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
