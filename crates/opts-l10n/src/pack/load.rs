//! Reading, validating, and discovering language pack folders on disk.

use super::*;

mod files;
mod parse;
mod warnings;

use files::*;
use parse::*;
use warnings::*;

/// A pack read from disk: its identity, the messages that passed the checks, and every finding.
#[derive(Debug, Clone)]
pub struct Pack {
    /// The folder.
    pub dir: PathBuf,
    /// The folder name.
    pub folder: String,
    /// The identity; `None` when `pack.json` is missing or wrong (the pack is then unusable).
    pub meta: Option<PackMeta>,
    /// The accepted messages of all string files.
    pub table: Table,
    /// The string files read.
    pub files: Vec<String>,
    /// All findings.
    pub issues: Vec<Issue>,
    /// Completeness against the English reference.
    pub stats: Stats,
    /// Scripts the text needs that the bundled fonts lack.
    pub missing_fonts: BTreeSet<&'static str>,
    /// `true` when `base_version` differs from the current English revision.
    pub outdated: bool,
}

impl Pack {
    /// Number of errors.
    #[must_use]
    pub fn errors(&self) -> usize {
        self.issues
            .iter()
            .filter(|i| i.severity == Severity::Error)
            .count()
    }

    /// Number of warnings.
    #[must_use]
    pub fn warnings(&self) -> usize {
        self.issues
            .iter()
            .filter(|i| i.severity == Severity::Warning)
            .count()
    }

    /// `true` when the pack has an identity and can be offered.
    #[must_use]
    pub fn usable(&self) -> bool {
        self.meta.is_some()
    }
}

/// Reads and checks the pack in `dir` against `reference`.
#[must_use]
pub fn load_pack_dir(dir: &Path, reference: &Reference) -> Pack {
    let folder = dir
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default();
    let mut issues = Vec::new();
    let meta = load_meta(dir, &folder, &mut issues);
    let names = list_pack_files(dir, &mut issues);
    let lang = meta
        .as_ref()
        .map_or(folder.as_str(), |m| m.code.as_str())
        .to_string();
    let mut table = Table::default();
    let mut origin: std::collections::HashMap<String, String> = std::collections::HashMap::new();
    let mut files = Vec::new();
    for name in names {
        let Some(text) = read_text(&dir.join(&name), &name, &mut issues) else {
            continue;
        };
        files.push(name.clone());
        let parsed = parse_pack_file(&name, &text, meta.as_ref(), &mut issues);
        let checked = check::validate(&parsed, &name, &lang, reference);
        merge_file_messages(&name, checked, &mut origin, &mut table, &mut issues);
    }
    let (missing_fonts, outdated) =
        check_pack_warnings(meta.as_ref(), &table, reference, &mut issues);
    let stats = check::stats(reference, &table, &lang);
    Pack {
        dir: dir.to_path_buf(),
        folder,
        meta,
        table,
        files,
        issues,
        stats,
        missing_fonts,
        outdated,
    }
}

/// Reads every pack folder directly below `root`. A missing `root` means no packs. Hidden folders
/// are skipped; at most [`MAX_PACKS`] are read.
#[must_use]
pub fn discover(root: &Path, reference: &Reference) -> Vec<Pack> {
    let mut dirs: Vec<PathBuf> = std::fs::read_dir(root)
        .into_iter()
        .flatten()
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.is_dir())
        .filter(|p| {
            p.file_name()
                .is_some_and(|n| !n.to_string_lossy().starts_with('.'))
        })
        .collect();
    dirs.sort();
    dirs.truncate(MAX_PACKS);
    dirs.iter().map(|d| load_pack_dir(d, reference)).collect()
}

/// The folder packs are read from: `<user data dir>/lang`, or `$OPTS_LANG_DIR` (tests and
/// automation; on Windows and macOS the user data folder cannot be redirected otherwise).
#[must_use]
pub fn default_root() -> Option<PathBuf> {
    std::env::var_os("OPTS_LANG_DIR")
        .map(PathBuf::from)
        .or_else(|| {
            directories::ProjectDirs::from("org", "opts", "opts").map(|d| d.data_dir().join("lang"))
        })
}
