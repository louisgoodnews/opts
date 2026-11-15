//! The disk cache of thumbnails: the cache file of a request, the loader and the size-capped prune.

use super::*;

pub(super) fn fnv(text: &str) -> u64 {
    text.bytes().fold(0xcbf2_9ce4_8422_2325, |h, b| {
        (h ^ u64::from(b)).wrapping_mul(0x0100_0000_01b3)
    })
}

/// Cache file of a request: the key, the source's identity (path, size, modification time) and
/// the target size, so a changed picture is rebuilt.
pub(super) fn cache_file(cache: &Path, key: &str, src: &Path, size: (u32, u32)) -> Option<PathBuf> {
    let meta = std::fs::metadata(src).ok()?;
    let stamp = meta
        .modified()
        .ok()
        .and_then(|t| t.duration_since(SystemTime::UNIX_EPOCH).ok())
        .map_or(0, |d| d.as_secs());
    let id = format!(
        "{key}|{}|{}|{stamp}|{}x{}",
        src.display(),
        meta.len(),
        size.0,
        size.1
    );
    Some(cache.join(format!("{:016x}.png", fnv(&id))))
}

/// Decodes `src` to `size` (cropped to fill), through the cache.
pub fn load_thumb(
    cache: &Path,
    key: &str,
    src: &Path,
    size: (u32, u32),
    cap_bytes: u64,
) -> Option<Thumb> {
    let file = cache_file(cache, key, src, size)?;
    if let Ok(img) = image::open(&file) {
        // A hit counts as use: the file's time moves forward for the least-recently-used rule.
        let _ = std::fs::File::options()
            .append(true)
            .open(&file)
            .and_then(|f| f.set_modified(SystemTime::now()));
        return Some(pixels(&img));
    }
    let img = image::open(src).ok()?;
    let small = img.resize_to_fill(size.0, size.1, image::imageops::FilterType::Triangle);
    if std::fs::create_dir_all(cache).is_ok() && small.save(&file).is_ok() {
        prune(cache, cap_bytes);
    }
    Some(pixels(&small))
}

pub fn prune(cache: &Path, cap_bytes: u64) {
    let Ok(read) = std::fs::read_dir(cache) else {
        return;
    };
    let mut files: Vec<(SystemTime, u64, PathBuf)> = read
        .flatten()
        .filter(|e| e.path().extension().is_some_and(|x| x == "png"))
        .filter_map(|e| {
            let m = e.metadata().ok()?;
            Some((m.modified().ok()?, m.len(), e.path()))
        })
        .collect();
    let mut total: u64 = files.iter().map(|f| f.1).sum();
    files.sort_by_key(|f| f.0);
    for (_, len, path) in files {
        if total <= cap_bytes {
            break;
        }
        if std::fs::remove_file(&path).is_ok() {
            total = total.saturating_sub(len);
        }
    }
}
