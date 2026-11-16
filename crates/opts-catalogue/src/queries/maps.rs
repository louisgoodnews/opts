//! Map and entity listings for the menus.

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
}
