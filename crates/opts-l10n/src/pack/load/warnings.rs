//! Pack-level warnings.

use super::*;

pub(super) fn check_pack_warnings(
    meta: Option<&PackMeta>,
    table: &Table,
    reference: &Reference,
    issues: &mut Vec<Issue>,
) -> (BTreeSet<&'static str>, bool) {
    let (missing_fonts, rtl_text) = check::scripts_beyond_fonts(table);
    if let Some(m) = meta {
        if !missing_fonts.is_empty() {
            issues.push(Issue::new(
                Severity::Warning,
                "font-coverage",
                "pack.json",
                None,
                None,
                format!(
                    "the text uses {} which the bundled fonts (Latin, Greek, Cyrillic) do not cover; those characters will show as empty boxes",
                    missing_fonts.iter().copied().collect::<Vec<_>>().join(", ")
                ),
            ));
        }
        if m.direction == Direction::Rtl {
            issues.push(Issue::new(
                Severity::Warning,
                "rtl-unsupported",
                "pack.json",
                None,
                None,
                "right-to-left languages are not supported by the interface yet; the language is listed but cannot be selected",
            ));
        } else if rtl_text {
            issues.push(Issue::new(
                Severity::Warning,
                "direction",
                "pack.json",
                None,
                None,
                "the text is written right to left but `direction` is \"ltr\"; set \"rtl\" (not supported by the interface yet)",
            ));
        }
    }
    let revision = reference.revision();
    let outdated = meta.is_some_and(|m| !m.base_version.is_empty() && m.base_version != revision);
    if outdated {
        issues.push(Issue::new(
            Severity::Warning,
            "outdated",
            "pack.json",
            None,
            None,
            format!(
                "made against English revision {} but the current one is {revision}; run `opts-tools l10n check` and translate the new or changed keys, then update `base_version`",
                meta.map_or("", |m| m.base_version.as_str())
            ),
        ));
    }
    (missing_fonts, outdated)
}
