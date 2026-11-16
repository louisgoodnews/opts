//! What `opts-tools l10n check` found, rendered as text or JSON.

use std::fmt::Write as _;

use super::*;


/// What `check` found, for printing.
pub(super) struct Findings {
    pub(super) label: String,
    pub(super) lang: String,
    pub(super) meta: Option<PackMeta>,
    pub(super) stats: check::Stats,
    pub(super) issues: Vec<Issue>,
    pub(super) outdated: bool,
    pub(super) revision: String,
}

impl Findings {
    pub(super) fn count(&self, s: Severity) -> usize {
        self.issues.iter().filter(|i| i.severity == s).count()
    }

    pub(super) fn to_json(&self) -> String {
        let issues: Vec<String> = self
            .issues
            .iter()
            .map(|i| i.to_json().to_string())
            .collect();
        let meta = self.meta.as_ref().map_or("null".to_string(), |m| {
            serde_json::json!({
                "code": m.code, "name": m.name, "native_name": m.native_name,
                "direction": m.direction.name(), "version": m.version,
                "authors": m.authors, "license": m.license, "base_version": m.base_version,
            })
            .to_string()
        });
        format!(
            "{{\"target\":{},\"language\":{},\"pack\":{},\"total\":{},\"present\":{},\"translated\":{},\"copied\":{},\"missing\":{},\"unknown\":{},\"percent\":{:.1},\"errors\":{},\"warnings\":{},\"outdated\":{},\"base_version_current\":{},\"missing_keys\":{},\"issues\":[{}]}}\n",
            q(&self.label),
            q(&self.lang),
            meta,
            self.stats.total,
            self.stats.present,
            self.stats.translated,
            self.stats.copied,
            self.stats.missing.len(),
            self.stats.unknown.len(),
            self.stats.percent(),
            self.count(Severity::Error),
            self.count(Severity::Warning),
            self.outdated,
            q(&self.revision),
            serde_json::to_string(&self.stats.missing).unwrap_or_else(|_| "[]".into()),
            issues.join(",")
        )
    }

    pub(super) fn to_text(&self) -> String {
        let mut s = String::new();
        for i in &self.issues {
            let _ = writeln!(s, "{i}");
        }
        let _ = writeln!(
            s,
            "{}: language `{}`: {} of {} translated ({:.1} %), {} copied from English, {} missing; {} error(s), {} warning(s){}",
            self.label,
            self.lang,
            self.stats.translated,
            self.stats.total,
            self.stats.percent(),
            self.stats.copied,
            self.stats.missing.len(),
            self.count(Severity::Error),
            self.count(Severity::Warning),
            if self.outdated { "; made against an older English text" } else { "" }
        );
        s
    }
}
