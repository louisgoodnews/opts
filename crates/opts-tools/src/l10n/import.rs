//! `opts-tools l10n import`: reads a JSON export and writes it as a pack.

use std::fmt::Write as _;

use super::scaffold::parse_pack_meta;
use super::*;

fn write_imported_pack(
    dir: &Path,
    accepted: &interchange::Imported,
    meta: &PackMeta,
    reference: &Reference,
    mut out: String,
) -> Result<Output, String> {
    std::fs::create_dir_all(dir).map_err(|e| io_err(dir, &e))?;
    let meta_path = dir.join("pack.json");
    std::fs::write(&meta_path, meta.to_json()).map_err(|e| io_err(&meta_path, &e))?;
    let _ = writeln!(out, "wrote {}", meta_path.display());
    for (name, body) in interchange::import_to_ftl(accepted) {
        let p = dir.join(name);
        std::fs::write(&p, body).map_err(|e| io_err(&p, &e))?;
        let _ = writeln!(out, "wrote {}", p.display());
    }
    let p = pack::load_pack_dir(dir, reference);
    let _ = writeln!(
        out,
        "{}: {:.1} % translated, {} error(s), {} warning(s)",
        dir.display(),
        p.stats.percent(),
        p.errors(),
        p.warnings()
    );
    Ok(Output {
        code: i32::from(p.errors() > 0),
        out,
        err: String::new(),
    })
}

pub(super) fn import(args: &[String]) -> Result<Output, String> {
    let a = Args::parse(args, &["root", "lang"], &["force", "lenient"])?;
    let [file] = a.positional.as_slice() else {
        return Err("import needs one interchange file".into());
    };
    let text = std::fs::read_to_string(file).map_err(|e| io_err(Path::new(file), &e))?;
    let imported = interchange::parse_json(&text);
    let lang = a
        .value("lang")
        .map(str::to_string)
        .or_else(|| imported.language.clone())
        .ok_or("the file has no `language`; pass --lang <code>")?;
    let reference = builtin::english();
    let checked = check::validate(&imported.table, file, &lang, &reference);
    let mut out = String::new();
    for i in &checked.issues {
        let _ = writeln!(out, "{i}");
    }
    let errors = checked
        .issues
        .iter()
        .filter(|i| i.severity == Severity::Error)
        .count();
    if errors > 0 && !a.flag("lenient") {
        let _ = writeln!(out, "{file}: {errors} error(s); nothing written (fix them, or use --lenient to skip the faulty messages)");
        return Ok(Output {
            code: 1,
            out,
            err: String::new(),
        });
    }
    let dir = root_of(&a)?.join(&lang);
    if dir.exists() && !a.flag("force") {
        return Err(format!(
            "{} exists; use --force to overwrite its files",
            dir.display()
        ));
    }
    let Some(meta) = parse_pack_meta(&imported, file, &lang, &mut out)? else {
        return Ok(Output {
            code: 1,
            out,
            err: String::new(),
        });
    };
    let mut accepted = imported;
    accepted.table = checked.table;
    write_imported_pack(&dir, &accepted, &meta, &reference, out)
}
