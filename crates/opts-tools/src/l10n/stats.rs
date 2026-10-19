//! The `stats` command: completeness of every language against English.

use super::*;

pub(super) fn stats(args: &[String]) -> Result<Output, String> {
    let a = Args::parse(args, &["root"], &["json"])?;
    let root = root_of(&a)?;
    let reference = builtin::english();
    let revision = reference.revision();
    let packs = pack::discover(&root, &reference);
    // (code, name, native, builtin, stats, errors, warnings, outdated, folder)
    let mut rows = Vec::new();
    for code in builtin::CODES {
        let table = base_translation(code);
        let m = builtin::meta_of(code).unwrap_or_else(|| unreachable!());
        rows.push((
            (*code).to_string(),
            m.name,
            m.native_name,
            true,
            check::stats(&reference, &table, code),
            0,
            0,
            false,
            String::new(),
        ));
    }
    for p in &packs {
        let (code, name, native) = p.meta.as_ref().map_or_else(
            || (p.folder.clone(), String::new(), String::new()),
            |m| (m.code.clone(), m.name.clone(), m.native_name.clone()),
        );
        rows.push((
            code,
            name,
            native,
            false,
            p.stats.clone(),
            p.errors(),
            p.warnings(),
            p.outdated,
            p.dir.display().to_string(),
        ));
    }
    let mut out = String::new();
    if a.flag("json") {
        let langs: Vec<String> = rows
            .iter()
            .map(|(c, n, nn, b, s, e, w, o, d)| {
                format!(
                    "{{\"code\":{},\"name\":{},\"native_name\":{},\"builtin\":{b},\"total\":{},\"translated\":{},\"copied\":{},\"missing\":{},\"percent\":{:.1},\"errors\":{e},\"warnings\":{w},\"outdated\":{o},\"dir\":{}}}",
                    q(c), q(n), q(nn), s.total, s.translated, s.copied, s.missing.len(), s.percent(), q(d)
                )
            })
            .collect();
        let _ = writeln!(
            out,
            "{{\"root\":{},\"base_version\":{},\"total\":{},\"languages\":[{}]}}",
            q(&root.display().to_string()),
            q(&revision),
            reference.len(),
            langs.join(",")
        );
    } else {
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
        for (c, n, nn, b, s, e, w, o, _) in &rows {
            let _ = writeln!(
                out,
                "{:<10} {:<22} {:>7.1}% {:>7} {:>8} {:>6} {:>6}{}{}",
                c,
                if nn.is_empty() {
                    n.clone()
                } else {
                    format!("{n} ({nn})")
                },
                s.percent(),
                s.copied,
                s.missing.len(),
                e,
                w,
                if *b { "  built in" } else { "" },
                if *o { "  outdated" } else { "" }
            );
        }
    }
    Ok(Output {
        code: 0,
        out,
        err: String::new(),
    })
}
