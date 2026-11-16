//! `opts-tools l10n export`: writes a pack template or a translation as JSON or Fluent files.

use std::fmt::Write as _;

use super::*;

pub(super) fn export(args: &[String]) -> Result<Output, String> {
    let a = Args::parse(args, &["lang", "format", "pack", "out"], &["missing"])?;
    let lang = a.value("lang").ok_or("--lang <code> is required")?;
    if !pack::valid_code(lang) {
        return Err(format!(
            "`{lang}` is not a language tag (like `fr` or `pt-BR`)"
        ));
    }
    let format = a.value("format").unwrap_or("json");
    let reference = builtin::english();
    let mut translation = base_translation(lang);
    let mut meta = builtin::meta_of(lang);
    if let Some(dir) = a.value("pack") {
        let p = pack::load_pack_dir(Path::new(dir), &reference);
        translation.messages.extend(p.table.messages);
        translation.order.extend(p.table.order);
        if meta.is_none() {
            meta = p.meta;
        }
    }
    let opts = ExportOptions {
        only_missing: a.flag("missing"),
    };
    let meta = meta.unwrap_or_else(|| PackMeta {
        code: lang.to_string(),
        name: String::new(),
        native_name: String::new(),
        direction: Direction::Ltr,
        version: "0.1.0".into(),
        authors: Vec::new(),
        license: pack::EXPECTED_LICENSE.into(),
        base_version: reference.revision(),
    });
    let mut meta = meta;
    meta.base_version = reference.revision();
    let mut result = Output::default();
    match format {
        "json" => {
            let text = interchange::export_json(&reference, &translation, lang, &meta, opts);
            match a.value("out") {
                Some(path) => {
                    std::fs::write(path, &text).map_err(|e| io_err(Path::new(path), &e))?;
                    let _ = writeln!(result.err, "wrote {path}");
                }
                None => result.out = text,
            }
        }
        "ftl" => {
            let files = interchange::export_ftl(&reference, &translation, lang, opts);
            match a.value("out") {
                Some(dir) => {
                    std::fs::create_dir_all(dir).map_err(|e| io_err(Path::new(dir), &e))?;
                    for (name, text) in &files {
                        let p = Path::new(dir).join(name);
                        std::fs::write(&p, text).map_err(|e| io_err(&p, &e))?;
                        let _ = writeln!(result.err, "wrote {}", p.display());
                    }
                }
                None => {
                    result.out = files
                        .iter()
                        .map(|(_, t)| t.as_str())
                        .collect::<Vec<_>>()
                        .join("\n");
                }
            }
        }
        other => return Err(format!("unknown format `{other}` (json or ftl)")),
    }
    Ok(result)
}
