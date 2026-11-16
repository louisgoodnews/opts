//! Profile and scenery object lookups on `WorldLoadJob`: the parsed `.sli` and `.sco` files, cached.

use super::*;

impl WorldLoadJob {
    pub(crate) fn sli(&mut self, rel: &str) -> Option<Arc<SplineProfile>> {
        let path = self.install.find_file(rel).or_else(|| {
            let found = self.install.find_relocated(rel)?;
            tracing::warn!(sli = rel, found = %found.display(), "spline profile found under another folder name");
            Some(found)
        });
        let Some(path) = path else {
            tracing::error!(at = %opts_bevy::position_tag(), sli = rel, "spline profile not found");
            return None;
        };
        self.loading_deps
            .insert(opts_bevy::hot_reload::cache::normalize(&path));
        if let Some(c) = self.sli_cache.get(&path) {
            return c.clone();
        }
        let parsed = std::fs::read(&path)
            .map_err(|e| e.to_string())
            .and_then(|b| SplineProfile::parse(&b).map_err(|e| e.to_string()));
        let out = match parsed {
            Ok(p) => {
                tracing::debug!(path = %path.display(), profiles = p.graphical_profiles.len(), paths = p.paths.len(), "loaded sli");
                Some(Arc::new(p))
            }
            Err(e) => {
                tracing::error!(at = %opts_bevy::position_tag(), path = %path.display(), "sli parse failed: {e}");
                None
            }
        };
        self.sli_cache.insert(path, out.clone());
        out
    }

    pub(crate) fn sco(&mut self, rel: &str) -> Option<(PathBuf, Arc<SceneryObject>)> {
        let Some(path) = self.install.find_file(rel).or_else(|| {
            let found = self.install.find_relocated(rel)?;
            tracing::warn!(sco = rel, found = %found.display(), "scenery object found under another folder name");
            Some(found)
        }) else {
            tracing::error!(at = %opts_bevy::position_tag(), sco = rel, "scenery object not found");
            return None;
        };
        self.loading_deps
            .insert(opts_bevy::hot_reload::cache::normalize(&path));
        if let Some(c) = self.sco_cache.get(&path) {
            return c.clone().map(|o| (path, o));
        }
        let parsed = std::fs::read(&path)
            .map_err(|e| e.to_string())
            .and_then(|b| SceneryObject::parse(&b).map_err(|e| e.to_string()));
        let out = match parsed {
            Ok(o) => {
                tracing::debug!(path = %path.display(), meshes = o.meshes.len(), "loaded sco");
                // A bent crossing's paths and arms follow its base surface.
                let o = Arc::new(o);
                match self.crossing_base(&path, &o) {
                    Some((_, base)) => Some(Arc::new(o.bent(&base))),
                    None => Some(o),
                }
            }
            Err(e) => {
                tracing::error!(at = %opts_bevy::position_tag(), path = %path.display(), "sco parse failed: {e}");
                None
            }
        };
        self.sco_cache.insert(path.clone(), out.clone());
        out.map(|o| (path, o))
    }
}
