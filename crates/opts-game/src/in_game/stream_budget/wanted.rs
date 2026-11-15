//! Which tiles to stream in and out: distance, heading and hysteresis.

use super::TileKey;

/// Chebyshev distance of two tile keys.
#[must_use]
pub fn chebyshev(a: TileKey, b: TileKey) -> i32 {
    (a.0 - b.0).abs().max((a.1 - b.1).abs())
}

/// Sort key of a wanted tile: smaller loads first.
///
/// `offset` is the vector from the camera to the tile centre in tile units, `forward` the
/// camera's (or travel) direction on the ground plane (x east, y north; need not be normalised).
/// A tile straight ahead counts as up to 40 % nearer than one behind the observer, so with equal
/// distance the tile the observer is heading for is built first.
#[must_use]
pub fn priority(offset: (f32, f32), forward: (f32, f32)) -> f32 {
    let d = offset.0.hypot(offset.1);
    let f = forward.0.hypot(forward.1);
    if d < 1e-4 || f < 1e-4 {
        return d;
    }
    let cos = (offset.0 * forward.0 + offset.1 * forward.1) / (d * f);
    d * (1.0 - 0.4 * cos)
}

/// Tiles to have resident, best first.
///
/// All tiles within Chebyshev `radius` of `centre` for which `exists` holds and which are not
/// `resident`, sorted by [`priority`] (`cam` is the camera position in tile units relative to the
/// same origin as the keys, so a key's centre is `(x + 0.5, y + 0.5)`). With `ahead > 0` and a
/// non-zero `forward`, the tiles up to `ahead` steps beyond the radius in the dominant travel
/// direction (one row of three tiles per step) are appended after the ring: prefetch.
pub fn wanted_tiles(
    centre: TileKey,
    radius: i32,
    cam: (f32, f32),
    forward: (f32, f32),
    ahead: i32,
    resident: impl Fn(TileKey) -> bool,
    exists: impl Fn(TileKey) -> bool,
) -> Vec<TileKey> {
    let mut keys: Vec<TileKey> = Vec::new();
    for dy in -radius..=radius {
        for dx in -radius..=radius {
            keys.push((centre.0 + dx, centre.1 + dy));
        }
    }
    if ahead > 0 && (forward.0 != 0.0 || forward.1 != 0.0) {
        let (sx, sy) = if forward.0.abs() >= forward.1.abs() {
            (forward.0.signum() as i32, 0)
        } else {
            (0, forward.1.signum() as i32)
        };
        for step in 1..=ahead {
            for lateral in -1..=1 {
                let (ox, oy) = if sx != 0 {
                    (sx * (radius + step), lateral)
                } else {
                    (lateral, sy * (radius + step))
                };
                keys.push((centre.0 + ox, centre.1 + oy));
            }
        }
    }
    let ring = (2 * radius + 1).pow(2) as usize;
    let mut near: Vec<(f32, TileKey)> = Vec::new();
    let mut prefetch: Vec<(f32, TileKey)> = Vec::new();
    for (i, k) in keys.into_iter().enumerate() {
        if resident(k) || !exists(k) {
            continue;
        }
        let off = (k.0 as f32 + 0.5 - cam.0, k.1 as f32 + 0.5 - cam.1);
        let entry = (priority(off, forward), k);
        if i < ring {
            near.push(entry);
        } else {
            prefetch.push(entry);
        }
    }
    near.sort_by(|a, b| a.0.total_cmp(&b.0));
    prefetch.sort_by(|a, b| a.0.total_cmp(&b.0));
    near.into_iter().chain(prefetch).map(|(_, k)| k).collect()
}

/// Resident tiles to unload: beyond `radius + hysteresis` (Chebyshev) of the centre. The
/// prefetch rows are inside this limit as long as `ahead <= hysteresis`.
pub fn evictable(
    resident: impl IntoIterator<Item = TileKey>,
    centre: TileKey,
    radius: i32,
    hysteresis: i32,
) -> Vec<TileKey> {
    resident
        .into_iter()
        .filter(|k| chebyshev(*k, centre) > radius + hysteresis)
        .collect()
}

