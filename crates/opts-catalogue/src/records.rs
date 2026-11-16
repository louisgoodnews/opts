//! Row types returned by the catalogue queries for menus.

/// A map row for menus.
#[derive(Debug, Clone, PartialEq)]
pub struct MapRecord {
    /// Entity id.
    pub entity_id: i64,
    /// Folder name under `maps/` (stable id).
    pub folder: String,
    /// Display name.
    pub friendly_name: String,
    /// Description.
    pub description: String,
    /// Number of tiles listed in `global.cfg`.
    pub tile_count: i64,
    /// Default depot.
    pub standard_depot: String,
    /// Year range text.
    pub years: String,
    /// Entry point names.
    pub entry_points: Vec<String>,
}

/// A vehicle row for menus.
#[derive(Debug, Clone, PartialEq)]
pub struct VehicleRecord {
    /// Entity id.
    pub entity_id: i64,
    /// `.bus` file stem (stable id).
    pub id: String,
    /// Marker path relative to the OMSI root.
    pub marker_file: String,
    /// Manufacturer.
    pub manufacturer: String,
    /// Model.
    pub model: String,
    /// Default livery.
    pub livery: String,
    /// Description.
    pub description: String,
    /// Curb mass in tonnes.
    pub mass_tonnes: Option<f64>,
    /// Vehicle class (`road`, `rail`, `aircraft`) when the content says it.
    pub class: Option<String>,
}
