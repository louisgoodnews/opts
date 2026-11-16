//! The viewpoint store as a Bevy resource: the file it lives in, and the load, reload and save of it.

use super::*;

/// The store and where it lives (`None` when no user data directory is known).
#[derive(Resource, Debug, Clone, Default)]
pub struct Viewpoints {
    /// The file; tests point it at a temporary directory.
    pub path: Option<PathBuf>,
    /// What the file holds (as last read or written).
    pub store: ViewpointStore,
}

impl Viewpoints {
    /// Reads the default file.
    #[must_use]
    pub fn load() -> Self {
        let path = ViewpointStore::path();
        let store = path
            .as_deref()
            .map(ViewpointStore::load_from)
            .unwrap_or_default();
        Self { path, store }
    }

    /// Re-reads the file.
    pub fn reload(&mut self) {
        if let Some(p) = &self.path {
            self.store = ViewpointStore::load_from(p);
        }
    }

    /// Stores a viewpoint (for the vehicle, or for its whole model) and writes the file.
    ///
    /// # Errors
    /// The I/O error when the file cannot be written; the in-memory store is updated anyway.
    pub fn save(
        &mut self,
        key: &str,
        model: Option<&str>,
        index: usize,
        vp: Viewpoint,
        whole_model: bool,
    ) -> std::io::Result<()> {
        // Another run may have written other vehicles meanwhile: merge into the file's content.
        let mut store = self
            .path
            .as_deref()
            .map_or_else(|| self.store.clone(), ViewpointStore::load_from);
        match (whole_model, model) {
            (true, Some(m)) => store.set_model(key, m, index, vp),
            _ => store.set_vehicle(key, model, index, vp),
        }
        self.store = store;
        match &self.path {
            Some(p) => self.store.save_to(p),
            None => Ok(()),
        }
    }
}
