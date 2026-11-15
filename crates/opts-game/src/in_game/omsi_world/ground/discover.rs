//! Which layers a map paints and the painted masks of a tile.

use super::*;

/// The `[groundtex]` layers (`1..count`) that have a mask file in the map's `texture/map/` or
/// `texture/` folder (`<tile>.map.<n>.dds`, any case), ascending.
pub(super) fn painted_layers(map_dir: &std::path::Path, count: usize) -> Vec<usize> {
    let mut found = std::collections::BTreeSet::new();
    let dirs = std::fs::read_dir(map_dir)
        .into_iter()
        .flatten()
        .flatten()
        .filter(|e| {
            e.file_name()
                .to_string_lossy()
                .eq_ignore_ascii_case("texture")
        })
        .flat_map(|tex| {
            let sub = std::fs::read_dir(tex.path())
                .into_iter()
                .flatten()
                .flatten()
                .filter(|e| e.file_name().to_string_lossy().eq_ignore_ascii_case("map"))
                .map(|e| e.path());
            std::iter::once(tex.path()).chain(sub).collect::<Vec<_>>()
        })
        .collect::<Vec<_>>();
    for dir in dirs {
        for entry in std::fs::read_dir(dir).into_iter().flatten().flatten() {
            let name = entry.file_name().to_string_lossy().to_ascii_lowercase();
            let Some(stem) = name.strip_suffix(".dds") else {
                continue;
            };
            let Some((head, n)) = stem.rsplit_once('.') else {
                continue;
            };
            if head.ends_with(".map") {
                if let Ok(n) = n.parse::<usize>() {
                    if (1..count).contains(&n) {
                        found.insert(n);
                    }
                }
            }
        }
    }
    found.into_iter().collect()
}

/// The painted ground masks of a tile (`<tile>.<n>.dds`), by layer number.
pub(super) fn read_masks(
    job: &mut WorldLoadJob,
    file: &str,
    layer_total: usize,
) -> Vec<(usize, opts_bevy::ground::CpuImage)> {
    use opts_bevy::ground::CpuImage;
    let mut masks: Vec<(usize, CpuImage)> = Vec::new();
    for n in 1..layer_total {
        let mask = [
            format!("texture/map/{file}.{n}.dds"),
            format!("texture/{file}.{n}.dds"),
        ]
        .iter()
        .find_map(|rel| job.install.find_in(&job.config.map_dir, rel))
        .inspect(|p| {
            job.loading_deps
                .insert(opts_bevy::hot_reload::cache::normalize(p));
        })
        .and_then(|p| std::fs::read(p).ok())
        .and_then(|b| opts_bevy::load_bevy_image(&b).ok())
        .and_then(|img| CpuImage::from_image(&img));
        if let Some(mask) = mask {
            masks.push((n, mask));
        }
    }
    masks
}
