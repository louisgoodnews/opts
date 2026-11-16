//! `opts-tools l10n check`: validates a pack folder or a single JSON or Fluent file.

use super::findings::Findings;
use super::*;

pub(super) fn check_cmd(args: &[String]) -> Result<Output, String> {
    let a = Args::parse(args, &["lang", "min-percent"], &["json", "strict"])?;
    let [target] = a.positional.as_slice() else {
        return Err("check needs exactly one pack folder or file".into());
    };
    let path = Path::new(target);
    let reference = builtin::english();
    let revision = reference.revision();
    let findings = if path.is_dir() {
        let p = pack::load_pack_dir(path, &reference);
        let lang = p.meta.as_ref().map_or(p.folder.clone(), |m| m.code.clone());
        Findings {
            label: target.clone(),
            lang,
            meta: p.meta,
            stats: p.stats,
            issues: p.issues,
            outdated: p.outdated,
            revision,
        }
    } else if path.is_file() {
        file_findings(path, target, a.value("lang"), &reference, revision)?
    } else {
        return Err(format!("{target}: no such file or folder"));
    };
    let min = a
        .value("min-percent")
        .map(|v| {
            v.parse::<f64>()
                .map_err(|_| format!("--min-percent: `{v}` is not a number"))
        })
        .transpose()?;
    let (errors, warnings) = (
        findings.count(Severity::Error),
        findings.count(Severity::Warning),
    );
    let failed = errors > 0
        || (a.flag("strict") && warnings > 0)
        || min.is_some_and(|m| findings.stats.percent() < m);
    Ok(Output {
        code: i32::from(failed),
        out: if a.flag("json") {
            findings.to_json()
        } else {
            findings.to_text()
        },
        err: String::new(),
    })
}

fn file_findings(
    path: &Path,
    shown: &str,
    lang: Option<&str>,
    reference: &Reference,
    revision: String,
) -> Result<Findings, String> {
    let text = std::fs::read_to_string(path).map_err(|e| io_err(path, &e))?;
    let (parsed, declared, base) = if path.extension().is_some_and(|e| e == "json") {
        let i = interchange::parse_json(&text);
        (i.table, i.language, i.base_version)
    } else {
        (fluent::parse(&text), None, None)
    };
    let lang = lang
        .map(str::to_string)
        .or(declared)
        .ok_or("--lang <code> is required for a file without a `language` field")?;
    let checked = check::validate(&parsed, shown, &lang, reference);
    let mut issues = checked.issues;
    let outdated = base.as_ref().is_some_and(|b| *b != revision);
    if outdated {
        issues.push(Issue::new(
            Severity::Warning,
            "outdated",
            shown,
            None,
            None,
            "the file was exported from an older English text; export a fresh template with `--missing` for the new keys",
        ));
    }
    let stats = check::stats(reference, &checked.table, &lang);
    Ok(Findings {
        label: shown.to_string(),
        lang,
        meta: None,
        stats,
        issues,
        outdated,
        revision,
    })
}
