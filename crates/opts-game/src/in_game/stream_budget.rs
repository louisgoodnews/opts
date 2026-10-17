//! Frame-budget and prioritisation helpers for tile streaming (M-0089).
//!
//! Pure functions and small types without Bevy dependencies, so the streamer in `omsi_world`
//! (owned by the terrain/water work, see `docs/src/architecture/streaming-performance.md`) can adopt
//! them without this module touching those files, and a synthetic multi-tile test can measure the
//! work per frame.
//!
//! * [`FrameBudget`]: wall-clock budget of one frame; work items are started while it is not
//!   exhausted (at least one item per frame, so progress is guaranteed).
//! * [`wanted_tiles`]: the tiles a streamer should have resident, nearest and most-in-view first,
//!   with an optional look-ahead along the direction of travel.
//! * [`evictable`]: resident tiles beyond the radius plus hysteresis.
//! * [`Spread`]: queue that hands out at most `per_frame` items (despawn spreading).

use std::collections::VecDeque;
use std::time::{Duration, Instant};

/// Integer tile key `(x, y)`; `y` grows north.
pub type TileKey = (i32, i32);

/// Wall-clock budget of one frame for streaming work.
#[derive(Debug, Clone, Copy)]
pub struct FrameBudget {
    limit: Duration,
    start: Instant,
    started: u32,
}

impl FrameBudget {
    /// A budget of `limit` starting now.
    #[must_use]
    pub fn new(limit: Duration) -> Self {
        Self {
            limit,
            start: Instant::now(),
            started: 0,
        }
    }

    /// Whether another work item may start: always the first one of a frame (progress guarantee),
    /// afterwards while the elapsed time is below the limit. Call once per item you start.
    pub fn allow(&mut self) -> bool {
        let ok = self.started == 0 || self.start.elapsed() < self.limit;
        if ok {
            self.started += 1;
        }
        ok
    }

    /// Items allowed so far.
    #[must_use]
    pub fn started(&self) -> u32 {
        self.started
    }

    /// Time used so far.
    #[must_use]
    pub fn used(&self) -> Duration {
        self.start.elapsed()
    }
}

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

/// Queue that releases at most `per_frame` items per call (spreads despawns over frames).
#[derive(Debug, Clone)]
pub struct Spread<T> {
    items: VecDeque<T>,
    per_frame: usize,
}

impl<T> Spread<T> {
    /// A queue releasing `per_frame` (at least 1) items per [`Spread::take`].
    #[must_use]
    pub fn new(per_frame: usize) -> Self {
        Self {
            items: VecDeque::new(),
            per_frame: per_frame.max(1),
        }
    }

    /// Queues an item.
    pub fn push(&mut self, item: T) {
        self.items.push_back(item);
    }

    /// Items waiting.
    #[must_use]
    pub fn len(&self) -> usize {
        self.items.len()
    }

    /// Whether nothing waits.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    /// The items this frame handles.
    pub fn take(&mut self) -> Vec<T> {
        let n = self.per_frame.min(self.items.len());
        self.items.drain(..n).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn budget_always_allows_the_first_item() {
        let mut b = FrameBudget::new(Duration::ZERO);
        assert!(b.allow());
        assert!(!b.allow());
        assert_eq!(b.started(), 1);
    }

    #[test]
    fn budget_allows_many_cheap_items() {
        let mut b = FrameBudget::new(Duration::from_secs(60));
        assert!((0..100).all(|_| b.allow()));
    }

    #[test]
    fn heading_breaks_ties_between_equal_distances() {
        // camera at the centre of tile (0,0), heading east
        let cam = (0.5, 0.5);
        let fwd = (1.0, 0.0);
        let w = wanted_tiles((0, 0), 1, cam, fwd, 0, |k| k == (0, 0), |_| true);
        assert_eq!(w.len(), 8);
        assert_eq!(w[0], (1, 0), "tile ahead first: {w:?}");
        assert_eq!(
            w.last().unwrap().0,
            -1,
            "a tile behind the observer last: {w:?}"
        );
    }

    #[test]
    fn nearest_first_without_heading() {
        let w = wanted_tiles((5, 5), 1, (5.9, 5.5), (0.0, 0.0), 0, |_| false, |_| true);
        assert_eq!(w[0], (5, 5));
        assert_eq!(w.len(), 9);
    }

    #[test]
    fn missing_tiles_are_skipped_and_prefetch_follows_the_ring() {
        let w = wanted_tiles(
            (0, 0),
            1,
            (0.5, 0.5),
            (0.0, 1.0),
            2,
            |_| false,
            |k| k.0 != 1, // column x = 1 does not exist
        );
        assert!(w.iter().all(|k| k.0 != 1));
        // ring: 6 tiles (x in -1,0), prefetch rows at y = 2 and 3: x in -1,0 (1 missing)
        assert_eq!(w.len(), 6 + 4);
        assert!(w[..6].iter().all(|k| k.1.abs() <= 1), "ring first: {w:?}");
        assert!(w[6..].iter().all(|k| k.1 >= 2), "prefetch last: {w:?}");
    }

    #[test]
    fn eviction_has_hysteresis() {
        let res = [(0, 0), (2, 0), (3, 0), (0, -3)];
        let ev = evictable(res, (0, 0), 1, 1);
        assert_eq!(ev, vec![(3, 0), (0, -3)]);
    }

    #[test]
    fn spread_releases_a_bounded_number_per_frame() {
        let mut s = Spread::new(2);
        (0..5).for_each(|i| s.push(i));
        assert_eq!(s.take(), vec![0, 1]);
        assert_eq!(s.take(), vec![2, 3]);
        assert_eq!(s.take(), vec![4]);
        assert!(s.is_empty());
        assert!(s.take().is_empty());
    }

    /// Synthetic multi-tile map: a camera crossing 20 tiles with budgeted loading never starts more
    /// than `cap` tiles in a frame, and every tile on the way ends up resident.
    #[test]
    fn synthetic_drive_respects_the_per_frame_cap() {
        let cap = 1;
        let mut resident: std::collections::HashSet<TileKey> = std::collections::HashSet::new();
        let mut worst = 0;
        for frame in 0..400 {
            let cam = (frame as f32 * 0.05, 0.5);
            let centre = (cam.0.floor() as i32, 0);
            for k in evictable(resident.iter().copied(), centre, 1, 1) {
                resident.remove(&k);
            }
            let mut started = 0;
            for k in wanted_tiles(
                centre,
                1,
                cam,
                (1.0, 0.0),
                1,
                |k| resident.contains(&k),
                |_| true,
            ) {
                if started >= cap {
                    break;
                }
                resident.insert(k);
                started += 1;
            }
            worst = worst.max(started);
        }
        assert!(worst <= cap);
        assert!(resident.contains(&(20, 0)));
    }
}
