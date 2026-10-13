//! `opts-tools l10n ...`: templates, checks and statistics for language packs.
//!
//! ```text
//! opts-tools l10n export --lang <code> [--format json|ftl] [--pack <dir>] [--missing] [--out <path>]
//! opts-tools l10n check  <pack dir | file.json | file.ftl> [--lang <code>] [--json] [--strict] [--min-percent N]
//! opts-tools l10n new    <code> [--name N] [--native-name N] [--author A]... [--root <dir>] [--force]
//! opts-tools l10n import <file.json> [--root <dir>] [--force] [--lenient]
//! opts-tools l10n stats  [--root <dir>] [--json]
//! ```
//!
//! Exit codes: 0 fine, 1 the check found errors (or warnings with `--strict`, or the percentage
//! is below `--min-percent`), 2 wrong usage. Every command prints JSON with `--json` (`export`
//! with `--format json` always does). The same code that the game uses to load packs does the
//! checking, so a pack that passes here loads there.

use std::fmt::Write as _;
use std::path::{Path, PathBuf};

use opts_core::l10n::builtin;
use opts_core::l10n::check::{self, Issue, Reference, Severity};
use opts_core::l10n::fluent::{self, Table};
use opts_core::l10n::interchange::{self, ExportOptions};
use opts_core::l10n::pack::{self, Direction, PackMeta};

/// What a command produced.
#[derive(Debug, Default)]
pub struct Output {
    /// Process exit code.
    pub code: i32,
    /// Standard output.
    pub out: String,
    /// Standard error.
    pub err: String,
}

impl Output {
    fn usage(msg: impl Into<String>) -> Self {
        Self {
            code: 2,
            out: String::new(),
            err: format!("l10n: {}\n{USAGE}\n", msg.into()),
        }
    }
}

/// The usage text.
pub const USAGE: &str = "usage: opts-tools l10n export --lang <code> [--format json|ftl] [--pack <dir>] [--missing] [--out <path>]\n       opts-tools l10n check <pack dir|file.json|file.ftl> [--lang <code>] [--json] [--strict] [--min-percent N]\n       opts-tools l10n new <code> [--name N] [--native-name N] [--author A] [--root <dir>] [--force]\n       opts-tools l10n import <file.json> [--root <dir>] [--force] [--lenient]\n       opts-tools l10n stats [--root <dir>] [--json]";

/// Parsed command line: positionals, flags with a value, boolean flags.
#[derive(Debug, Default)]
struct Args {
    positional: Vec<String>,
    values: Vec<(String, String)>,
    flags: Vec<String>,
}

impl Args {
    fn parse(args: &[String], value_flags: &[&str], bool_flags: &[&str]) -> Result<Self, String> {
        let mut a = Self::default();
        let mut it = args.iter();
        while let Some(x) = it.next() {
            if let Some(name) = x.strip_prefix("--") {
                if value_flags.contains(&name) {
                    let v = it.next().ok_or_else(|| format!("--{name} needs a value"))?;
                    a.values.push((name.to_string(), v.clone()));
                } else if bool_flags.contains(&name) {
                    a.flags.push(name.to_string());
                } else {
                    return Err(format!("unknown option --{name}"));
                }
            } else {
                a.positional.push(x.clone());
            }
        }
        Ok(a)
    }

    fn value(&self, name: &str) -> Option<&str> {
        self.values
            .iter()
            .rev()
            .find(|(n, _)| n == name)
            .map(|(_, v)| v.as_str())
    }

    fn all(&self, name: &str) -> Vec<&str> {
        self.values
            .iter()
            .filter(|(n, _)| n == name)
            .map(|(_, v)| v.as_str())
            .collect()
    }

    fn flag(&self, name: &str) -> bool {
        self.flags.iter().any(|f| f == name)
    }
}

/// Runs `opts-tools l10n <args>`.
#[must_use]
pub fn execute(args: &[String]) -> Output {
    let Some((sub, rest)) = args.split_first() else {
        return Output::usage("a subcommand is required");
    };
    let result = match sub.as_str() {
        "export" => export(rest),
        "check" => check_cmd(rest),
        "new" => new_cmd(rest),
        "import" => import(rest),
        "stats" => stats(rest),
        other => Err(format!("unknown subcommand `{other}`")),
    };
    result.unwrap_or_else(Output::usage)
}

fn q(s: &str) -> String {
    serde_json::to_string(s).unwrap_or_else(|_| "\"\"".into())
}

fn root_of(a: &Args) -> Result<PathBuf, String> {
    a.value("root")
        .map(PathBuf::from)
        .or_else(pack::default_root)
        .ok_or_else(|| "no user data directory; pass --root <dir>".to_string())
}

fn io_err(path: &Path, e: &std::io::Error) -> String {
    format!("{}: {e}", path.display())
}

/// The translation of `lang` known without a pack folder: the built-in table, if any.
fn base_translation(lang: &str) -> Table {
    builtin::table_of(lang).unwrap_or_default()
}

fn export(args: &[String]) -> Result<Output, String> {
    let a = Args::parse(args, &["lang", "format", "pack", "out"], &["missing"])?;
    let lang = a.value("lang").ok_or("--lang <code> is required")?;
    if !pack::valid_code(lang) {
        return Err(format!("`{lang}` is not a language tag (like `fr` or `pt-BR`)"));
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

/// What `check` found, for printing.
struct Findings {
    label: String,
    lang: String,
    meta: Option<PackMeta>,
    stats: check::Stats,
    issues: Vec<Issue>,
    outdated: bool,
    revision: String,
}

impl Findings {
    fn count(&self, s: Severity) -> usize {
        self.issues.iter().filter(|i| i.severity == s).count()
    }

    fn to_json(&self) -> String {
        let issues: Vec<String> = self.issues.iter().map(|i| i.to_json().to_string()).collect();
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

    fn to_text(&self) -> String {
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

fn check_cmd(args: &[String]) -> Result<Output, String> {
    let a = Args::parse(args, &["lang", "min-percent"], &["json", "strict"])?;
    let [target] = a.positional.as_slice() else {
        return Err("check needs exactly one pack folder or file".into());
    };
    let path = Path::new(target);
    let reference = builtin::english();
    let revision = reference.revision();
    let findings = if path.is_dir() {
        let p = pack::load_pack_dir(path, &reference);
        let lang = p
            .meta
            .as_ref()
            .map_or(p.folder.clone(), |m| m.code.clone());
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
        .map(|v| v.parse::<f64>().map_err(|_| format!("--min-percent: `{v}` is not a number")))
        .transpose()?;
    let (errors, warnings) = (findings.count(Severity::Error), findings.count(Severity::Warning));
    let failed = errors > 0
        || (a.flag("strict") && warnings > 0)
        || min.is_some_and(|m| findings.stats.percent() < m);
    Ok(Output {
        code: i32::from(failed),
        out: if a.flag("json") { findings.to_json() } else { findings.to_text() },
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

fn new_cmd(args: &[String]) -> Result<Output, String> {
    let a = Args::parse(args, &["name", "native-name", "author", "root"], &["force"])?;
    let [code] = a.positional.as_slice() else {
        return Err("new needs one language tag, for example `opts-tools l10n new fr`".into());
    };
    if !pack::valid_code(code) {
        return Err(format!("`{code}` is not a language tag (like `fr` or `pt-BR`)"));
    }
    if builtin::CODES.contains(&code.as_str()) {
        return Err(format!(
            "`{code}` is built in; to improve it edit the source tables, or put a pack with that tag in the user folder to override single messages"
        ));
    }
    let dir = root_of(&a)?.join(code);
    if dir.exists() && !a.flag("force") {
        return Err(format!("{} exists; use --force to overwrite its files", dir.display()));
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
    for (name, text) in interchange::export_ftl(&reference, &Table::default(), code, ExportOptions::default()) {
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

fn import(args: &[String]) -> Result<Output, String> {
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
        return Ok(Output { code: 1, out, err: String::new() });
    }
    let dir = root_of(&a)?.join(&lang);
    if dir.exists() && !a.flag("force") {
        return Err(format!("{} exists; use --force to overwrite its files", dir.display()));
    }
    // pack.json from the file's `pack` object, else from nothing.
    let meta_text = imported
        .pack
        .as_ref()
        .map(|p| serde_json::to_string_pretty(p).unwrap_or_default())
        .ok_or("the file has no `pack` object (identity of the language); export a template with `l10n export` and fill it")?;
    let (meta, meta_issues) = pack::parse_meta(&meta_text, &lang);
    let Some(mut meta) = meta else {
        for i in &meta_issues {
            let _ = writeln!(out, "{file} (pack): {}", i.message);
        }
        let _ = writeln!(out, "{file}: the `pack` object is incomplete; nothing written");
        return Ok(Output { code: 1, out, err: String::new() });
    };
    if let Some(b) = &imported.base_version {
        meta.base_version.clone_from(b);
    }
    let mut accepted = imported;
    accepted.table = checked.table;
    std::fs::create_dir_all(&dir).map_err(|e| io_err(&dir, &e))?;
    let meta_path = dir.join("pack.json");
    std::fs::write(&meta_path, meta.to_json()).map_err(|e| io_err(&meta_path, &e))?;
    let _ = writeln!(out, "wrote {}", meta_path.display());
    for (name, body) in interchange::import_to_ftl(&accepted) {
        let p = dir.join(name);
        std::fs::write(&p, body).map_err(|e| io_err(&p, &e))?;
        let _ = writeln!(out, "wrote {}", p.display());
    }
    let p = pack::load_pack_dir(&dir, &reference);
    let _ = writeln!(
        out,
        "{}: {:.1} % translated, {} error(s), {} warning(s)",
        dir.display(),
        p.stats.percent(),
        p.errors(),
        p.warnings()
    );
    Ok(Output { code: i32::from(p.errors() > 0), out, err: String::new() })
}

fn stats(args: &[String]) -> Result<Output, String> {
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
        rows.push((code, name, native, false, p.stats.clone(), p.errors(), p.warnings(), p.outdated, p.dir.display().to_string()));
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
        let _ = writeln!(out, "English revision {revision}, {} messages; packs in {}", reference.len(), root.display());
        let _ = writeln!(out, "{:<10} {:<22} {:>8} {:>7} {:>8} {:>6} {:>6}", "code", "name", "done", "copied", "missing", "errors", "warn");
        for (c, n, nn, b, s, e, w, o, _) in &rows {
            let _ = writeln!(
                out,
                "{:<10} {:<22} {:>7.1}% {:>7} {:>8} {:>6} {:>6}{}{}",
                c,
                if nn.is_empty() { n.clone() } else { format!("{n} ({nn})") },
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
    Ok(Output { code: 0, out, err: String::new() })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn scratch(name: &str) -> PathBuf {
        let dir = std::env::var_os("HOME")
            .map(PathBuf::from)
            .unwrap_or_default()
            .join(".local/share/opts/benchmark/langpacks/tools-tests")
            .join(name);
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn run(args: &[&str]) -> Output {
        execute(&args.iter().map(|s| (*s).to_string()).collect::<Vec<_>>())
    }

    #[test]
    fn new_check_and_stats_work_together() {
        let root = scratch("new-check");
        let r = root.to_str().unwrap();
        let o = run(&["new", "fr", "--root", r, "--name", "French", "--native-name", "Français", "--author", "Tester"]);
        assert_eq!(o.code, 0, "{}{}", o.out, o.err);
        let pack = root.join("fr");
        let c = run(&["check", pack.to_str().unwrap(), "--json"]);
        let v: serde_json::Value = serde_json::from_str(&c.out).unwrap();
        assert_eq!(c.code, 0, "{}", c.out);
        assert_eq!(v["errors"], 0);
        // One-word texts may legitimately equal English (names, loanwords), so an untouched
        // scaffold still counts them; every sentence is a copy.
        assert!(v["percent"].as_f64().unwrap() < 50.0, "{}", c.out);
        assert!(v["copied"].as_u64().unwrap() > 200, "{}", c.out);
        assert_eq!(v["outdated"], false);
        // Translate one message and break another.
        let settings = std::fs::read_to_string(pack.join("gui.ftl")).unwrap();
        let edited = settings
            .replace("widget-clear = Clear", "widget-clear = Effacer")
            .replace("widget-not-available-yet = Feature not available yet!", "widget-not-available-yet = { $oops");
        std::fs::write(pack.join("gui.ftl"), edited).unwrap();
        let c = run(&["check", pack.to_str().unwrap()]);
        assert_eq!(c.code, 1);
        assert!(c.out.contains("gui.ftl:"), "{}", c.out);
        assert!(c.out.contains("error[syntax]"), "{}", c.out);
        let s = run(&["stats", "--root", r, "--json"]);
        let v: serde_json::Value = serde_json::from_str(&s.out).unwrap();
        let langs = v["languages"].as_array().unwrap();
        assert_eq!(langs.len(), 3);
        assert_eq!(langs[2]["code"], "fr");
        assert!(langs[2]["translated"].as_u64().unwrap() >= 1);
        assert_eq!(langs[1]["code"], "de");
        let again = run(&["new", "fr", "--root", r]);
        assert_eq!(again.code, 2, "refuses to overwrite");
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn export_import_round_trip_through_the_command_line() {
        let root = scratch("round-trip");
        let json = root.join("de.json");
        let o = run(&["export", "--lang", "de", "--out", json.to_str().unwrap()]);
        assert_eq!(o.code, 0, "{}", o.err);
        // The pack object of a built-in language is complete, so import works as is.
        let packs = root.join("packs");
        let i = run(&["import", json.to_str().unwrap(), "--root", packs.to_str().unwrap()]);
        assert_eq!(i.code, 0, "{}", i.out);
        let c = run(&["check", packs.join("de").to_str().unwrap(), "--json", "--min-percent", "98"]);
        assert_eq!(c.code, 0, "{}", c.out);
        let v: serde_json::Value = serde_json::from_str(&c.out).unwrap();
        assert_eq!(v["missing"], 0);
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn a_filled_json_template_is_checked_without_a_pack_folder() {
        let root = scratch("template");
        let o = run(&["export", "--lang", "fr", "--missing"]);
        assert_eq!(o.code, 0);
        let v: serde_json::Value = serde_json::from_str(&o.out).unwrap();
        assert_eq!(v["format"], "opts-l10n/1");
        let mut v = v;
        v["messages"]["widget-clear"]["text"] = "Effacer".into();
        v["messages"]["widget-items"]["forms"]["one"] = "{ $count } élément".into();
        v["messages"]["widget-items"]["forms"]["many"] = "{ $count } éléments".into();
        v["messages"]["widget-items"]["forms"]["other"] = "{ $count } éléments".into();
        v["messages"]["menu-shutdown"]["text"] = "Quitter { $unknown }".into();
        let f = root.join("fr.json");
        std::fs::write(&f, serde_json::to_string_pretty(&v).unwrap()).unwrap();
        let c = run(&["check", f.to_str().unwrap(), "--json"]);
        assert_eq!(c.code, 1);
        let r: serde_json::Value = serde_json::from_str(&c.out).unwrap();
        assert_eq!(r["errors"], 1, "{}", c.out);
        assert_eq!(r["issues"][0]["code"], "variable-mismatch");
        assert_eq!(r["issues"][0]["key"], "menu-shutdown");
        assert!(r["issues"][0]["line"].as_u64().unwrap() > 5);
        assert_eq!(r["translated"], 2);
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn usage_errors_exit_with_two() {
        assert_eq!(run(&[]).code, 2);
        assert_eq!(run(&["frobnicate"]).code, 2);
        assert_eq!(run(&["export"]).code, 2);
        assert_eq!(run(&["check", "/no/such/place"]).code, 2);
        assert_eq!(run(&["new", "BAD CODE"]).code, 2);
        assert_eq!(run(&["new", "de"]).code, 2);
    }
}
