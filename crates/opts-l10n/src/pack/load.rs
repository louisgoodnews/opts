//! Reading, validating, and discovering language pack folders on disk.

use super::*;

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

fn read_text(path: &Path, shown: &str, issues: &mut Vec<Issue>) -> Option<String> {
    let fail = |issues: &mut Vec<Issue>, why: String| {
        issues.push(Issue::new(Severity::Error, "read", shown, None, None, why));
    };
    match std::fs::metadata(path) {
        Ok(m) if m.len() > MAX_FILE_BYTES => {
            fail(
                issues,
                format!(
                    "larger than {} KiB; the file is skipped",
                    MAX_FILE_BYTES / 1024
                ),
            );
            return None;
        }
        Ok(_) => {}
        Err(e) => {
            fail(issues, format!("cannot be read ({e}); the file is skipped"));
            return None;
        }
    }
    match std::fs::read(path) {
        Ok(bytes) => match String::from_utf8(bytes) {
            Ok(t) => Some(t.strip_prefix('\u{feff}').map_or(t.clone(), str::to_string)),
            Err(e) => {
                fail(
                    issues,
                    format!("is not UTF-8 text (invalid byte at offset {}); save it as UTF-8. The file is skipped", e.utf8_error().valid_up_to()),
                );
                None
            }
        },
        Err(e) => {
            fail(issues, format!("cannot be read ({e}); the file is skipped"));
            None
        }
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
    let meta_path = dir.join("pack.json");
    let meta = if meta_path.exists() {
        read_text(&meta_path, "pack.json", &mut issues).and_then(|t| {
            let (m, i) = parse_meta(&t, &folder);
            issues.extend(i);
            m
        })
    } else {
        issues.push(Issue::new(
            Severity::Error,
            "pack-json",
            "pack.json",
            None,
            None,
            "missing; create it with `opts-tools l10n new <code>` or copy it from another pack. The folder is ignored",
        ));
        None
    };
    let mut names: Vec<String> = std::fs::read_dir(dir)
        .into_iter()
        .flatten()
        .flatten()
        .filter(|e| e.path().is_file())
        .map(|e| e.file_name().to_string_lossy().into_owned())
        .filter(|n| {
            !n.starts_with('.') && (n.ends_with(".ftl") || n.ends_with(".json")) && n != "pack.json"
        })
        .collect();
    names.sort();
    if names.len() > MAX_FILES {
        issues.push(Issue::new(
            Severity::Warning,
            "read",
            ".",
            None,
            None,
            format!(
                "{} string files; only the first {MAX_FILES} are read",
                names.len()
            ),
        ));
        names.truncate(MAX_FILES);
    }
    let lang = meta
        .as_ref()
        .map_or(folder.as_str(), |m| m.code.as_str())
        .to_string();
    let mut table = Table::default();
    let mut origin: std::collections::HashMap<String, String> = std::collections::HashMap::new();
    let mut files = Vec::new();
    {
        for name in names {
            let Some(text) = read_text(&dir.join(&name), &name, &mut issues) else {
                continue;
            };
            files.push(name.clone());
            let parsed = if name.ends_with(".json") {
                let imported = interchange::parse_json(&text);
                if let (Some(l), Some(m)) = (&imported.language, &meta) {
                    if *l != m.code {
                        issues.push(Issue::new(
                            Severity::Warning,
                            "language-mismatch",
                            &name,
                            interchange::line_of_json_key(&text, "language", 0),
                            None,
                            format!("`language` is `{l}` but the pack is `{}`", m.code),
                        ));
                    }
                }
                imported.table
            } else {
                fluent::parse(&text)
            };
            let checked = check::validate(&parsed, &name, &lang, reference);
            issues.extend(checked.issues);
            for key in checked.table.order {
                if let Some(prev) = origin.insert(key.clone(), name.clone()) {
                    issues.push(Issue::new(
                        Severity::Warning,
                        "duplicate-key",
                        &name,
                        checked.table.lines.get(&key).copied(),
                        Some(&key),
                        format!("also defined in {prev}; this later definition wins"),
                    ));
                } else {
                    table.order.push(key.clone());
                }
                if let Some(p) = checked.table.messages.get(&key) {
                    table.messages.insert(key.clone(), p.clone());
                }
            }
        }
    }
    let (missing_fonts, rtl_text) = check::scripts_beyond_fonts(&table);
    if let Some(m) = &meta {
        if !missing_fonts.is_empty() {
            issues.push(Issue::new(
                Severity::Warning,
                "font-coverage",
                "pack.json",
                None,
                None,
                format!(
                    "the text uses {} which the bundled fonts (Latin, Greek, Cyrillic) do not cover; those characters will show as empty boxes",
                    missing_fonts.iter().copied().collect::<Vec<_>>().join(", ")
                ),
            ));
        }
        if m.direction == Direction::Rtl {
            issues.push(Issue::new(
                Severity::Warning,
                "rtl-unsupported",
                "pack.json",
                None,
                None,
                "right-to-left languages are not supported by the interface yet; the language is listed but cannot be selected",
            ));
        } else if rtl_text {
            issues.push(Issue::new(
                Severity::Warning,
                "direction",
                "pack.json",
                None,
                None,
                "the text is written right to left but `direction` is \"ltr\"; set \"rtl\" (not supported by the interface yet)",
            ));
        }
    }
    let revision = reference.revision();
    let outdated = meta
        .as_ref()
        .is_some_and(|m| !m.base_version.is_empty() && m.base_version != revision);
    if outdated {
        issues.push(Issue::new(
            Severity::Warning,
            "outdated",
            "pack.json",
            None,
            None,
            format!("made against English revision {} but the current one is {revision}; run `opts-tools l10n check` and translate the new or changed keys, then update `base_version`", meta.as_ref().map_or("", |m| m.base_version.as_str())),
        ));
    }
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
