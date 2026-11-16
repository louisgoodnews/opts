//! Vehicle listings and the marker lookup.

use super::*;

impl Catalogue {
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
}
