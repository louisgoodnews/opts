//! Scan totals, kind counts and category diagnostics.

use super::*;

impl Catalogue {
    /// Unix time of the last completed scan, if any.
    ///
    /// # Errors
    /// SQLite failures.
    pub fn last_scan(&self) -> Result<Option<i64>> {
        Ok(self
            .meta("last_scan_completed_at")?
            .and_then(|v| v.parse().ok()))
    }

    /// `(files, entities)` counted by the last scan.
    ///
    /// # Errors
    /// SQLite failures.
    pub fn totals(&self) -> Result<(i64, i64)> {
        let get = |k: &str| {
            self.meta(k)
                .map(|v| v.and_then(|v| v.parse().ok()).unwrap_or(0))
        };
        Ok((get("total_files")?, get("total_entities")?))
    }

    /// The catalogue broken down by kind, with the same rules the menus use: `maps` and
    /// `vehicles` are exactly the lists of [`Catalogue::maps`] and [`Catalogue::playable_vehicles`]
    /// (what Home and the wizard show); `entities` counts every entry of every category.
    ///
    /// # Errors
    /// SQLite failures.
    pub fn kind_counts(&self) -> Result<KindCounts> {
        let (files, _) = self.totals()?;
        let (maps, vehicles, entities) = self.with(|c| {
            let maps: i64 = c.query_row(
                "SELECT count(*) FROM entities e JOIN map_info m ON m.entity_id = e.entity_id
                 JOIN directories d ON d.dir_id = e.entity_dir_id WHERE e.category = 'map'",
                [],
                |r| r.get(0),
            )?;
            let vehicles: i64 = c.query_row(
                "SELECT count(*) FROM entities e JOIN vehicle_info v ON v.entity_id = e.entity_id
                 JOIN files f ON f.file_id = e.marker_file_id
                 WHERE e.category = 'vehicle' AND e.is_playable = 1 AND e.alias_of_id IS NULL",
                [],
                |r| r.get(0),
            )?;
            let entities: i64 = c.query_row("SELECT count(*) FROM entities", [], |r| r.get(0))?;
            Ok((maps, vehicles, entities))
        })?;
        let by_category = self.category_counts()?;
        let of = |name: &str| {
            by_category
                .iter()
                .find(|(c, _)| c == name)
                .map_or(0, |(_, n)| *n)
        };
        let (scenery_objects, splines) = (of("sceneryobject"), of("spline"));
        Ok(KindCounts {
            files,
            entities,
            maps,
            vehicles,
            scenery_objects,
            splines,
            other: (entities - maps - vehicles - scenery_objects - splines).max(0),
        })
    }

    /// Counts per entity category (diagnostics).
    ///
    /// # Errors
    /// SQLite failures.
    pub fn category_counts(&self) -> Result<Vec<(String, i64)>> {
        self.with(|c| {
            let mut st = c.prepare(
                "SELECT category, count(*) FROM entities GROUP BY category ORDER BY category",
            )?;
            let rows = st.query_map([], |r| Ok((r.get(0)?, r.get(1)?)))?;
            rows.collect()
        })
    }
}
