//! The compiled script set of a `.sco` object, forked per placed object.

use super::*;

impl WorldLoadJob {
    /// The compiled script set of `sco` (`None` when it has no scripts or string lists).
    pub(in crate) fn script_template(
        &mut self,
        sco_path: &Path,
        sco: &SceneryObject,
    ) -> Option<Arc<ScriptTemplate>> {
        if sco.scripts.is_empty() && sco.stringvarnamelists.is_empty() {
            return None;
        }
        if let Some(t) = self.scenery.templates.get(sco_path) {
            return t.clone();
        }
        let dir = sco_path.parent().map(Path::to_path_buf).unwrap_or_default();
        let set = load_scenery_scripts(sco, |rel| {
            crate::script_file::read_script(self.install.find_in(&dir, rel))
        });
        let bad = problems(&set.diagnostics);
        self.scenery.stats.script_sets += 1;
        if bad > 0 {
            self.scenery.stats.script_sets_with_problems += 1;
            tracing::warn!(sco = %sco_path.display(), problems = bad, first = ?set.diagnostics.first().map(|d| &d.message), "scenery script set has problems");
        }
        let template = Arc::new(ScriptTemplate {
            engine: Mutex::new(set.engine),
            string_names: set.string_names,
            diagnostics: set.diagnostics,
            has_scripts: !sco.scripts.is_empty(),
        });
        self.scenery
            .templates
            .insert(sco_path.to_path_buf(), Some(template.clone()));
        Some(template)
    }
}
