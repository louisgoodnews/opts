//! Restore planning: which sections a file restores and in what order, what preflight reports, and what carries over.

use super::*;

/// What a restore will do with a file, decided before anything is applied.
#[derive(Debug, Clone, Default)]
pub struct RestorePlan {
    /// `(section, data at the registered version)` ordered by stage, then `order`.
    pub steps: Vec<(SaveSection, Value)>,
    /// Section ids in the file that this build does not know (kept, ignored).
    pub unknown: Vec<String>,
    /// Sections the file lacks (they keep their defaults).
    pub missing: Vec<String>,
    /// Problems that did not stop the plan: a section from a newer version, a failed migration.
    pub warnings: Vec<String>,
}

/// Matches the sections of `file` with the registry: migrates older ones, sets aside unknown,
/// newer and unmigratable ones.
#[must_use]
pub fn plan_restore(sections: &[SaveSection], file: &SaveFile) -> RestorePlan {
    let mut plan = RestorePlan::default();
    for s in sections {
        let Some(blob) = file.sections.get(s.id) else {
            plan.missing.push(s.id.to_owned());
            continue;
        };
        let data = if blob.version == s.version {
            Ok(blob.data.clone())
        } else if blob.version > s.version {
            Err(format!(
                "section `{}` is version {} but this build reads {}: ignored",
                s.id, blob.version, s.version
            ))
        } else if let Some(migrate) = s.migrate {
            migrate(blob.version, blob.data.clone()).map_err(|e| {
                format!(
                    "section `{}` v{} cannot be migrated: {e}",
                    s.id, blob.version
                )
            })
        } else {
            Err(format!(
                "section `{}` is version {} and has no migration to {}: ignored",
                s.id, blob.version, s.version
            ))
        };
        match data {
            Ok(d) => plan.steps.push((*s, d)),
            Err(w) => {
                plan.warnings.push(w);
            }
        }
    }
    plan.unknown = file
        .sections
        .keys()
        .filter(|k| !sections.iter().any(|s| s.id == k.as_str()))
        .cloned()
        .collect();
    plan.steps.sort_by_key(|(s, _)| (s.stage, s.order));
    plan
}

/// Every content problem the sections of `file` report.
#[must_use]
pub fn preflight(sections: &[SaveSection], file: &SaveFile, env: &CheckEnv) -> Vec<Issue> {
    plan_restore(sections, file)
        .steps
        .iter()
        .filter_map(|(s, data)| s.check.map(|check| check(data, env)))
        .flatten()
        .collect()
}

impl RestorePlan {
    /// The steps that have a `prepare`, in `order`.
    #[must_use]
    pub fn prepare_steps(&self) -> Vec<&(SaveSection, Value)> {
        let mut steps: Vec<_> = self
            .steps
            .iter()
            .filter(|(s, _)| s.prepare.is_some())
            .collect();
        steps.sort_by_key(|(s, _)| s.order);
        steps
    }
}

/// Copies the sections of `old` that this build did not capture into `new` (unknown ones and
/// newer versions of known ones), so overwriting a save never destroys another build's data.
pub fn carry_over_foreign(sections: &[SaveSection], old: &SaveFile, new: &mut SaveFile) {
    for (id, blob) in &old.sections {
        let known = sections.iter().find(|s| s.id == id.as_str());
        let foreign = known.is_none_or(|s| blob.version > s.version);
        if foreign && !new.sections.contains_key(id) {
            new.sections.insert(id.clone(), blob.clone());
        }
    }
}
