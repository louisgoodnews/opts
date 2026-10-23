//! The parking bays of a map: which vehicle a bay holds and whether it holds one at all.

use super::*;

impl WorldLoadJob {
    /// Reads the parking list with this selector (`0` = `parklist_p.txt`), once.
    fn parklist(&mut self, selector: u32) -> &[String] {
        if !self.parklists.contains_key(&selector) {
            let name = parklist_file_name(selector);
            let list = match self
                .data_file(&name)
                .map(|p| std::fs::read(&p).map(|b| (p, b)))
            {
                Some(Ok((p, bytes))) => {
                    self.loading_deps
                        .insert(opts_bevy::hot_reload::cache::normalize(&p));
                    let text: String = bytes.iter().map(|&b| char::from(b)).collect();
                    text.lines()
                        .map(|l| l.trim().to_string())
                        .filter(|l| !l.is_empty())
                        .collect()
                }
                Some(Err(e)) => {
                    tracing::error!(at = %opts_bevy::position_tag(), "{name} unreadable: {e}");
                    Vec::new()
                }
                None if selector == 0 => {
                    tracing::error!(at = %opts_bevy::position_tag(), map = %self.config.map_dir.display(), "map has parking bays but no parklist_p.txt");
                    Vec::new()
                }
                None => {
                    tracing::warn!(map = %self.config.map_dir.display(), "parking bays select {name}, which the map lacks: parklist_p.txt instead");
                    self.parklist(0).to_vec()
                }
            };
            self.parklists.insert(selector, list);
        }
        self.parklists.get(&selector).map_or(&[], Vec::as_slice)
    }

    /// Picks the parked vehicle for a parking-bay marker.
    ///
    /// The marker's first string value selects the list (blank or `0`: `parklist_p.txt`, `N`:
    /// `parklist_p_N.txt`); one `.sco` per line, duplicates raise the pick probability
    /// (`docs/src/formats/map/lists.md`). The draw is a hash of the marker's id code and position
    /// (the instances of an attachment share the id), so a bay shows the same car on every load.
    /// `None` for a bay that stays empty (`gameplay.parked`).
    pub(in super::super::super) fn parked_vehicle(
        &mut self,
        marker: &MapObjectInstance,
    ) -> Option<String> {
        let selector = parklist_selector(marker.string_vars.first().map(String::as_str));
        let permille = u64::from(self.parked_permille);
        let list = self.parklist(selector);
        if list.is_empty() {
            return None;
        }
        let cm = |v: f32| (f64::from(v) * 100.0).round() as i64 as u64;
        let mut h = marker.id_code;
        for v in [cm(marker.position.x), cm(marker.position.y)] {
            h = h.rotate_left(21) ^ v;
        }
        // splitmix64 finaliser: spreads consecutive ids evenly.
        h = h.wrapping_add(0x9E37_79B9_7F4A_7C15);
        h = (h ^ (h >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        h = (h ^ (h >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        h ^= h >> 31;
        // The bay is occupied with the setting's probability: a second draw from the same hash,
        // so the same bays stay empty on every load and a lower share only empties more of them.
        let occupied = bay_occupied(h, permille);
        occupied.then(|| list[(h % list.len() as u64) as usize].clone())
    }
}

/// Whether the parking bay with the draw `hash` holds a vehicle at `permille` of the bays: the
/// bays that stay empty at a higher share are a subset of those empty at a lower one.
fn bay_occupied(hash: u64, permille: u64) -> bool {
    (hash >> 40) % 1000 < permille
}

#[cfg(test)]
mod parked_tests {
    use super::bay_occupied;

    #[test]
    fn the_share_of_occupied_bays_follows_the_setting_and_only_grows() {
        let hashes: Vec<u64> = (0..20_000u64)
            .map(|i| {
                let mut h = i.wrapping_add(0x9E37_79B9_7F4A_7C15);
                h = (h ^ (h >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
                h = (h ^ (h >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
                h ^ (h >> 31)
            })
            .collect();
        let count = |p: u64| hashes.iter().filter(|h| bay_occupied(**h, p)).count();
        assert_eq!(count(0), 0);
        assert_eq!(count(1000), hashes.len());
        let half = count(500) as f64 / hashes.len() as f64;
        assert!((half - 0.5).abs() < 0.02, "{half}");
        assert!(hashes
            .iter()
            .all(|h| !bay_occupied(*h, 300) || bay_occupied(*h, 600)));
    }
}
