//! Queries on the overview: stop lookup, the extent, progress and the roads near a point.

use super::*;

impl OverviewRoads {
    /// Position and heading of the bus stop object with ID code `id`, once its tile was read.
    #[must_use]
    pub fn stop(&self, id: u64) -> Option<(DVec2, f64)> {
        self.stops.get(&id).copied()
    }

    /// The box around every lane read so far (south-west and north-east corner), if any.
    #[must_use]
    pub fn extent(&self) -> Option<(DVec2, DVec2)> {
        let mut lo = DVec2::splat(f64::MAX);
        let mut hi = DVec2::splat(f64::MIN);
        for tile in self.tiles.values() {
            for r in &tile.roads {
                for p in [
                    r.geometry.point_at(0.0),
                    r.geometry.point_at(r.geometry.length),
                ] {
                    lo = lo.min(p);
                    hi = hi.max(p);
                }
            }
        }
        (lo.x <= hi.x).then_some((lo, hi))
    }

    /// Whether the reader was started.
    #[must_use]
    pub fn started(&self) -> bool {
        self.started
    }

    /// Whether every tile was read (or there is nothing to read).
    #[must_use]
    pub fn complete(&self) -> bool {
        self.started && self.done >= self.total
    }

    /// Percent of the tiles read, 0 to 100.
    #[must_use]
    pub fn percent(&self) -> u32 {
        if self.total == 0 {
            return 100;
        }
        ((self.done * 100) / self.total).min(100) as u32
    }

    /// The roads of tiles for which `skip` is false that may touch the circle.
    pub fn roads_near(
        &self,
        centre: DVec2,
        radius_m: f64,
        skip: impl Fn((i32, i32)) -> bool,
        out: &mut Vec<MapRoad>,
    ) {
        for (key, tile) in &self.tiles {
            let (lo, hi) = tile.bounds;
            if hi.x < centre.x - radius_m
                || lo.x > centre.x + radius_m
                || hi.y < centre.y - radius_m
                || lo.y > centre.y + radius_m
                || skip(*key)
            {
                continue;
            }
            out.extend(
                tile.roads
                    .iter()
                    .filter(|r| may_touch(&r.geometry, centre, radius_m)),
            );
        }
    }
}
