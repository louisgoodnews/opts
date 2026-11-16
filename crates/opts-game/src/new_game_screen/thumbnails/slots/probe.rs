//! What a thumbnail key asks for, and where its picture is on disk: the map's `picture.jpg` and the
//! preview image next to a vehicle.

use super::*;

/// A preview image the content provides for a vehicle, if any.
pub fn vehicle_preview(root: &Path, bus_file: &str) -> Option<PathBuf> {
    let bus = root.join(bus_file);
    let dir = bus.parent()?;
    for folder in [Some(dir), dir.parent()].into_iter().flatten() {
        for entry in std::fs::read_dir(folder).ok()?.flatten() {
            let name = entry.file_name().to_string_lossy().to_lowercase();
            let image_ext = [".jpg", ".jpeg", ".png"].iter().any(|e| name.ends_with(e));
            let preview = ["picture", "preview", "thumbnail"]
                .iter()
                .any(|w| name.starts_with(w));
            if image_ext && preview {
                return Some(entry.path());
            }
        }
    }
    None
}

/// What to look for when a thumbnail is asked for; resolved on a worker thread.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Probe {
    /// `picture.jpg` of a map folder.
    Map(String),
    /// A preview image next to a vehicle's `.bus` file (path relative to the OMSI root).
    Vehicle(String),
}

impl Probe {
    /// The file on disk, if there is one. Touches the file system.
    pub fn resolve(&self, root: &Path) -> Option<PathBuf> {
        match self {
            Self::Map(folder) => {
                let p = root.join("maps").join(folder).join("picture.jpg");
                p.is_file().then_some(p)
            }
            Self::Vehicle(bus_file) => vehicle_preview(root, bus_file),
        }
    }
}

/// What thumbnail key `key` (`map:<folder>`, `veh:<id>`) needs; no file system access.
pub fn probe_of(key: &str, catalogue: &GameCatalogue) -> Option<Probe> {
    if let Some(folder) = key.strip_prefix("map:") {
        return Some(Probe::Map(folder.to_string()));
    }
    let id = key.strip_prefix("veh:")?;
    let v = catalogue.vehicles.iter().find(|v| v.id == id)?;
    (!v.bus_file.is_empty()).then(|| Probe::Vehicle(v.bus_file.clone()))
}

/// The source file of thumbnail key `key` (`map:<folder>`, `veh:<id>`).
pub fn source_of(key: &str, catalogue: &GameCatalogue, root: &Path) -> Option<PathBuf> {
    probe_of(key, catalogue)?.resolve(root)
}
