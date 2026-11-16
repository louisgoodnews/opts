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

mod args;
mod check_cmd;
mod export;
mod findings;
mod import;
mod scaffold;
mod stats;

use args::Args;
use check_cmd::check_cmd;
use export::export;
use import::import;
use scaffold::new_cmd;
use stats::stats;

#[cfg(test)]
mod tests;
