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

use opts_l10n::builtin;
use opts_l10n::check::{self, Issue, Reference, Severity};
use opts_l10n::fluent::{self, Table};
use opts_l10n::interchange::{self, ExportOptions};
use opts_l10n::pack::{self, Direction, PackMeta};

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

mod scaffold;
mod stats;
use scaffold::{import, new_cmd};
use stats::stats;

#[cfg(test)]
mod tests;
