//! Reading the files of a pack folder and its metadata.

use super::*;

pub(super) fn read_text(path: &Path, shown: &str, issues: &mut Vec<Issue>) -> Option<String> {
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

pub(super) fn load_meta(dir: &Path, folder: &str, issues: &mut Vec<Issue>) -> Option<PackMeta> {
    let meta_path = dir.join("pack.json");
    if meta_path.exists() {
        read_text(&meta_path, "pack.json", issues).and_then(|t| {
            let (m, i) = parse_meta(&t, folder);
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
    }
}

pub(super) fn list_pack_files(dir: &Path, issues: &mut Vec<Issue>) -> Vec<String> {
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
    names
}
