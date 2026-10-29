//! `opts-tools l10n new` and `opts-tools l10n import` commands for scaffolding and writing packs.

use super::*;

pub(super) fn new_cmd(args: &[String]) -> Result<Output, String> {
    let a = Args::parse(args, &["name", "native-name", "author", "root"], &["force"])?;
    let [code] = a.positional.as_slice() else {
        return Err("new needs one language tag, for example `opts-tools l10n new fr`".into());
    };
    if !pack::valid_code(code) {
        return Err(format!(
            "`{code}` is not a language tag (like `fr` or `pt-BR`)"
        ));
    }
    if builtin::CODES.contains(&code.as_str()) {
        return Err(format!(
            "`{code}` is built in; to improve it edit the source tables, or put a pack with that tag in the user folder to override single messages"
        ));
    }
    let dir = root_of(&a)?.join(code);
    if dir.exists() && !a.flag("force") {
        return Err(format!(
            "{} exists; use --force to overwrite its files",
            dir.display()
        ));
    }
    let reference = builtin::english();
    let meta = PackMeta {
        code: code.clone(),
        name: a.value("name").unwrap_or("").to_string(),
        native_name: a.value("native-name").unwrap_or("").to_string(),
        direction: Direction::Ltr,
        version: "0.1.0".into(),
        authors: a.all("author").into_iter().map(str::to_string).collect(),
        license: pack::EXPECTED_LICENSE.into(),
        base_version: reference.revision(),
    };
    std::fs::create_dir_all(&dir).map_err(|e| io_err(&dir, &e))?;
    let mut written = vec![dir.join("pack.json")];
    std::fs::write(&written[0], meta.to_json()).map_err(|e| io_err(&written[0], &e))?;
    for (name, text) in interchange::export_ftl(
        &reference,
        &Table::default(),
        code,
        ExportOptions::default(),
    ) {
        let p = dir.join(name);
        std::fs::write(&p, text).map_err(|e| io_err(&p, &e))?;
        written.push(p);
    }
    let mut out = String::new();
    for p in &written {
        let _ = writeln!(out, "wrote {}", p.display());
    }
    let _ = writeln!(
        out,
        "next: fill `name` and `native_name` in pack.json, translate the text after `=` in the .ftl files (keep keys and {{ $variables }}), then run `opts-tools l10n check {}`",
        dir.display()
    );
    Ok(Output {
        code: 0,
        out,
        err: String::new(),
    })
}

fn parse_pack_meta(
    imported: &interchange::Imported,
    file: &str,
    lang: &str,
    out: &mut String,
) -> Result<Option<PackMeta>, String> {
    let meta_text = imported
        .pack
        .as_ref()
        .map(|p| serde_json::to_string_pretty(p).unwrap_or_default())
        .ok_or("the file has no `pack` object (identity of the language); export a template with `l10n export` and fill it")?;
    let (meta, meta_issues) = pack::parse_meta(&meta_text, lang);
    let Some(mut meta) = meta else {
        for i in &meta_issues {
            let _ = writeln!(out, "{file} (pack): {}", i.message);
        }
        let _ = writeln!(
            out,
            "{file}: the `pack` object is incomplete; nothing written"
        );
        return Ok(None);
    };
    if let Some(b) = &imported.base_version {
        meta.base_version.clone_from(b);
    }
    Ok(Some(meta))
}

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
