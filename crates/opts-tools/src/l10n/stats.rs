//! The `stats` command: completeness of every language against English.

use super::*;

struct Row {
    code: String,
    name: String,
    native: String,
    builtin: bool,
    stats: check::Stats,
    errors: usize,
    warnings: usize,
    outdated: bool,
    dir: String,
}

fn collect_rows(root: &Path, reference: &Reference) -> Vec<Row> {
    let mut rows = Vec::new();
    for code in builtin::CODES {
        let table = base_translation(code);
        let m = builtin::meta_of(code).unwrap_or_else(|| unreachable!());
        rows.push(Row {
            code: (*code).to_string(),
            name: m.name,
            native: m.native_name,
            builtin: true,
            stats: check::stats(reference, &table, code),
            errors: 0,
            warnings: 0,
            outdated: false,
            dir: String::new(),
        });
    }
    for p in &pack::discover(root, reference) {
        let (code, name, native) = p.meta.as_ref().map_or_else(
            || (p.folder.clone(), String::new(), String::new()),
            |m| (m.code.clone(), m.name.clone(), m.native_name.clone()),
        );
        rows.push(Row {
            code,
            name,
            native,
            builtin: false,
            stats: p.stats.clone(),
            errors: p.errors(),
            warnings: p.warnings(),
            outdated: p.outdated,
            dir: p.dir.display().to_string(),
        });
    }
    rows
}

fn format_json(root: &Path, revision: &str, reference: &Reference, rows: &[Row]) -> String {
    let langs: Vec<String> = rows
        .iter()
        .map(|r| {
            format!(
                "{{\"code\":{},\"name\":{},\"native_name\":{},\"builtin\":{},\"total\":{},\"translated\":{},\"copied\":{},\"missing\":{},\"percent\":{:.1},\"errors\":{},\"warnings\":{},\"outdated\":{},\"dir\":{}}}",
                q(&r.code),
                q(&r.name),
                q(&r.native),
                r.builtin,
                r.stats.total,
                r.stats.translated,
                r.stats.copied,
                r.stats.missing.len(),
                r.stats.percent(),
                r.errors,
                r.warnings,
                r.outdated,
                q(&r.dir)
            )
        })
        .collect();
    let mut out = String::new();
    let _ = writeln!(
        out,
        "{{\"root\":{},\"base_version\":{},\"total\":{},\"languages\":[{}]}}",
        q(&root.display().to_string()),
        q(revision),
        reference.len(),
        langs.join(",")
    );
    out
}

fn format_human(root: &Path, revision: &str, reference: &Reference, rows: &[Row]) -> String {
    let mut out = String::new();
    let _ = writeln!(
        out,
        "English revision {revision}, {} messages; packs in {}",
        reference.len(),
        root.display()
    );
    let _ = writeln!(
        out,
        "{:<10} {:<22} {:>8} {:>7} {:>8} {:>6} {:>6}",
        "code", "name", "done", "copied", "missing", "errors", "warn"
    );
    for r in rows {
        let _ = writeln!(
            out,
            "{:<10} {:<22} {:>7.1}% {:>7} {:>8} {:>6} {:>6}{}{}",
            r.code,
            if r.native.is_empty() {
                r.name.clone()
            } else {
                format!("{} ({})", r.name, r.native)
            },
            r.stats.percent(),
            r.stats.copied,
            r.stats.missing.len(),
            r.errors,
            r.warnings,
            if r.builtin { "  built in" } else { "" },
            if r.outdated { "  outdated" } else { "" }
        );
    }
    out
}

pub(super) fn stats(args: &[String]) -> Result<Output, String> {
    let a = Args::parse(args, &["root"], &["json"])?;
    let root = root_of(&a)?;
    let reference = builtin::english();
    let revision = reference.revision();
    let rows = collect_rows(&root, &reference);
    let out = if a.flag("json") {
        format_json(&root, &revision, &reference, &rows)
    } else {
        format_human(&root, &revision, &reference, &rows)
    };
    Ok(Output {
        code: 0,
        out,
        err: String::new(),
    })
}
