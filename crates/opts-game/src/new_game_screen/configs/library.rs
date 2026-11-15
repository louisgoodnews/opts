//! The saved bus configurations the wizard keeps: the library, its file and its draft name.

use super::*;

/// The saved configurations as the wizard keeps them during a session.
#[derive(Resource, Debug, Clone)]
pub struct ConfigLibrary {
    /// The configurations (file `bus_configurations.json` beside the settings database).
    pub store: BusConfigurations,
    /// Where they are saved; `None` keeps them in memory only (tests, no config directory).
    pub path: Option<PathBuf>,
    /// Why the file could not be used (damaged, newer version); a newer file is never written.
    pub problem: Option<StoreProblem>,
    /// The text of the name field.
    pub draft_name: String,
    /// The configuration whose *Delete* was pressed once.
    pub pending_delete: Option<String>,
    /// The configuration just loaded, whose repaint and options still wait for the vehicle's
    /// repaint data to be checked against (see [`finish_load`]).
    pub pending_check: Option<String>,
}

impl Default for ConfigLibrary {
    fn default() -> Self {
        Self::at(BusConfigurations::path())
    }
}

impl ConfigLibrary {
    /// A library stored at `path` (or only in memory).
    #[must_use]
    pub fn at(path: Option<PathBuf>) -> Self {
        let (store, problem) = match path.as_deref().map(BusConfigurations::load_from) {
            Some(Ok(s)) => (s, None),
            Some(Err(p)) => (BusConfigurations::default(), Some(p)),
            None => (BusConfigurations::default(), None),
        };
        Self {
            store,
            path,
            problem,
            draft_name: String::new(),
            pending_delete: None,
            pending_check: None,
        }
    }

    /// Writes the list. Nothing is written over a file of a newer version.
    ///
    /// # Errors
    /// The I/O error of the write, or a refusal to overwrite a newer file.
    pub fn save(&mut self) -> std::io::Result<()> {
        if let Some(StoreProblem::Newer(v)) = &self.problem {
            return Err(std::io::Error::other(format!(
                "the configurations file has version {v}; it is not overwritten"
            )));
        }
        let Some(p) = &self.path else {
            return Ok(());
        };
        self.store.save_to(p)?;
        // A damaged file was moved aside by the write: the list is a good one now.
        self.problem = None;
        Ok(())
    }
}
