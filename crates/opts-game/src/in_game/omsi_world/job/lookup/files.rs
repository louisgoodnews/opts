//! Files of the load job: the vehicle and bus files, the sound configuration and its sound files.

use super::*;

impl WorldLoadJob {
    /// The `.bus` file the session's vehicle was loaded from.
    #[must_use]
    pub fn vehicle_file(&self) -> Option<&Path> {
        self.config.vehicle_path.as_deref()
    }

    /// The `.bus` file of a catalogue vehicle (`bus_file` is relative to the OMSI root).
    #[must_use]
    pub fn bus_file(&self, rel: &str) -> Option<PathBuf> {
        if rel.is_empty() {
            return None;
        }
        self.install.find_file(rel)
    }

    /// The parsed sound configuration that `rel` names (relative to the object's folder
    /// `base`), with the folder it was found in; `None` when it is missing or unreadable.
    pub fn sound_config(
        &mut self,
        base: &Path,
        rel: &str,
    ) -> Option<(Arc<opts_core::formats::sound::SoundConfig>, PathBuf)> {
        let path = self.install.find_in(base, rel)?;
        let dir = path.parent()?.to_path_buf();
        if let Some(c) = self.sound_cfgs.get(&path) {
            return c.clone().map(|c| (c, dir));
        }
        let parsed = std::fs::read(&path).ok().map(|bytes| {
            // Windows-1252 text; the keywords and file names are ASCII.
            let text: String = bytes.iter().map(|b| char::from(*b)).collect();
            Arc::new(opts_core::formats::sound::SoundConfig::parse(&text))
        });
        if let Some(c) = &parsed {
            for w in &c.warnings {
                tracing::debug!(file = %path.display(), "sound config: {w}");
            }
        }
        self.sound_cfgs.insert(path, parsed.clone());
        parsed.map(|c| (c, dir))
    }

    /// Like [`Self::sound_config`] without the cache, for callers that only hold `&self` (the
    /// AI vehicle types parse theirs once when they load).
    #[must_use]
    pub fn read_sound_config(
        &self,
        base: &Path,
        rel: &str,
    ) -> Option<(Arc<opts_core::formats::sound::SoundConfig>, PathBuf)> {
        let path = self.install.find_in(base, rel)?;
        let dir = path.parent()?.to_path_buf();
        let bytes = std::fs::read(&path).ok()?;
        let text: String = bytes.iter().map(|b| char::from(*b)).collect();
        Some((
            Arc::new(opts_core::formats::sound::SoundConfig::parse(&text)),
            dir,
        ))
    }

    /// The sound file a configuration names: relative to the configuration's folder (names are
    /// matched case-insensitively), else a file of that name in the shared `Sounds` folder.
    #[must_use]
    pub fn find_sound_file(&self, cfg_dir: &Path, name: &str) -> Option<PathBuf> {
        let name = name.trim().replace('\\', "/");
        self.install.find_in(cfg_dir, &name).or_else(|| {
            let base = name.rsplit('/').next().unwrap_or(&name);
            self.install
                .find_in(&self.install.root().join("Sounds"), base)
        })
    }

    /// A vehicle file named relative to `base` (`[couple_back]` and the like).
    #[must_use]
    pub fn find_vehicle_in(&self, base: &Path, rel: &str) -> Option<PathBuf> {
        self.install.find_in(base, rel)
    }
}
