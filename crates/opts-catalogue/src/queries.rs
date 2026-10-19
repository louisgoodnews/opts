//! Listing maps, vehicles, markers and totals.

use super::*;

impl Catalogue {
    /// All maps, alphabetically by display name.
    ///
    /// # Errors
    /// SQLite failures.
    pub fn maps(&self) -> Result<Vec<MapRecord>> {
        self.with(|c| {
            let mut st = c.prepare(
                "SELECT e.entity_id, d.dir_name, m.friendly_name, m.description, m.tile_count,
                        m.standard_depot, m.years, m.entry_points
                 FROM entities e JOIN map_info m ON m.entity_id = e.entity_id
                 JOIN directories d ON d.dir_id = e.entity_dir_id
                 WHERE e.category = 'map' ORDER BY m.friendly_name COLLATE NOCASE",
            )?;
            let rows = st.query_map([], |r| {
                let eps: String = r.get(7)?;
                Ok(MapRecord {
                    entity_id: r.get(0)?,
                    folder: r.get(1)?,
                    friendly_name: r.get(2)?,
                    description: r.get(3)?,
                    tile_count: r.get(4)?,
                    standard_depot: r.get(5)?,
                    years: r.get(6)?,
                    entry_points: eps.lines().map(str::to_string).collect(),
                })
            })?;
            rows.collect()
        })
    }

    /// Player-selectable vehicles (`is_playable = 1`, primary entities only).
    ///
    /// # Errors
    /// SQLite failures.
    pub fn playable_vehicles(&self) -> Result<Vec<VehicleRecord>> {
        self.with(|c| {
            let mut st = c.prepare(
                "SELECT e.entity_id, f.rel_path, f.file_name, v.manufacturer, v.model, v.livery,
                        v.description, v.mass_tonnes, e.vehicle_class
                 FROM entities e JOIN vehicle_info v ON v.entity_id = e.entity_id
                 JOIN files f ON f.file_id = e.marker_file_id
                 WHERE e.category = 'vehicle' AND e.is_playable = 1 AND e.alias_of_id IS NULL
                 ORDER BY v.manufacturer COLLATE NOCASE, v.model COLLATE NOCASE",
            )?;
            let rows = st.query_map([], |r| {
                let file: String = r.get(2)?;
                Ok(VehicleRecord {
                    entity_id: r.get(0)?,
                    marker_file: r.get(1)?,
                    id: file
                        .rsplit_once('.')
                        .map_or(file.clone(), |(s, _)| s.to_string()),
                    manufacturer: r.get(3)?,
                    model: r.get(4)?,
                    livery: r.get(5)?,
                    description: r.get(6)?,
                    mass_tonnes: r.get(7)?,
                    class: r.get(8)?,
                })
            })?;
            rows.collect()
        })
    }

    /// Every rail vehicle file (`.bus` and `.ovh`, playable or not, primary entities only), by
    /// path: the cars a train formation can be made of.
    ///
    /// # Errors
    /// SQLite failures.
    pub fn rail_vehicles(&self) -> Result<Vec<VehicleRecord>> {
        self.with(|c| {
            let mut st = c.prepare(
                "SELECT e.entity_id, f.rel_path, f.file_name, v.manufacturer, v.model, v.livery,
                        v.description, v.mass_tonnes, e.vehicle_class
                 FROM entities e JOIN vehicle_info v ON v.entity_id = e.entity_id
                 JOIN files f ON f.file_id = e.marker_file_id
                 WHERE e.category = 'vehicle' AND e.vehicle_class = 'rail' AND e.alias_of_id IS NULL
                 ORDER BY f.rel_path COLLATE NOCASE",
            )?;
            let rows = st.query_map([], |r| {
                let file: String = r.get(2)?;
                Ok(VehicleRecord {
                    entity_id: r.get(0)?,
                    marker_file: r.get(1)?,
                    id: file
                        .rsplit_once('.')
                        .map_or(file.clone(), |(s, _)| s.to_string()),
                    manufacturer: r.get(3)?,
                    model: r.get(4)?,
                    livery: r.get(5)?,
                    description: r.get(6)?,
                    mass_tonnes: r.get(7)?,
                    class: r.get(8)?,
                })
            })?;
            rows.collect()
        })
    }

    /// The train consist files (`.zug`) below `Trains/`, by path (relative to the OMSI root).
    ///
    /// # Errors
    /// SQLite failures.
    pub fn train_consists(&self) -> Result<Vec<String>> {
        self.with(|c| {
            let mut st = c.prepare(
                "SELECT rel_path FROM files
                 WHERE extension = 'zug' AND rel_path LIKE 'Trains/%'
                 ORDER BY rel_path COLLATE NOCASE",
            )?;
            let rows = st.query_map([], |r| r.get::<_, String>(0))?;
            rows.collect()
        })
    }

    /// Relative path of the `.bus` marker for a vehicle id (file stem), any case.
    ///
    /// # Errors
    /// SQLite failures.
    pub fn vehicle_marker(&self, id: &str) -> Result<Option<String>> {
        let like = format!("%/{id}.bus");
        self.with(|c| {
            c.query_row(
                "SELECT rel_path FROM files WHERE rel_path LIKE ?1 ESCAPE '\\' COLLATE NOCASE AND extension='bus' LIMIT 1",
                [like.replace('_', "\\_")],
                |r| r.get(0),
            )
            .optional()
        })
    }

    /// Relative paths of all `.hum` human profiles.
    ///
    /// # Errors
    /// SQLite failures.
    pub fn human_markers(&self) -> Result<Vec<String>> {
        self.with(|c| {
            let mut st = c.prepare(
                "SELECT f.rel_path FROM entities e JOIN files f ON f.file_id = e.marker_file_id
                 WHERE e.category = 'human' ORDER BY f.rel_path COLLATE NOCASE",
            )?;
            let rows = st.query_map([], |r| r.get(0))?;
            rows.collect()
        })
    }

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
