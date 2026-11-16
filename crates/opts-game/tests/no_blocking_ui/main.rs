//! M-0088 source lint: the UI modules never touch the file system or a database on the main
//! thread.
//!
//! Opening *Settings -> Content* once froze the application for seconds (catalogue counts, a
//! walk over the OMSI folder, a texture cache listing, all inside the system that builds the
//! page). The rule (`docs/src/contributing/standards.md`, "No blocking work on the main thread"):
//! UI systems read memory only; everything else goes through a worker (`content_facts`, an
//! `IoTaskPool` task, a thread with a channel) and arrives as a resource or message.
//!
//! This test scans the source of the UI modules for the calls that block and fails for each one
//! that is not on [`ALLOWED`]. Every allowance names why it is fine: it runs on a worker, at
//! startup, at exit, or is a tiny user-initiated write. A stale allowance fails too, so the
//! list cannot rot.

use std::path::{Path, PathBuf};

mod lists;
use lists::{ALLOWED, FORBIDDEN, UI_PATHS};

fn src() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("src")
}

fn files(path: &Path, out: &mut Vec<PathBuf>) {
    if path.is_file() {
        out.push(path.to_path_buf());
    } else if let Ok(rd) = std::fs::read_dir(path) {
        for e in rd.flatten() {
            files(&e.path(), out);
        }
    }
}

/// The production part of a file: no test files, nothing from `#[cfg(test)]` on, no comments.
fn production(path: &Path) -> Option<String> {
    let name = path.file_name()?.to_str()?;
    if name == "tests.rs" || path.components().any(|c| c.as_os_str() == "tests") {
        return None;
    }
    let text = std::fs::read_to_string(path).ok()?;
    let cut = text.find("#[cfg(test)]").unwrap_or(text.len());
    Some(
        text[..cut]
            .lines()
            .filter(|l| !l.trim_start().starts_with("//"))
            .collect::<Vec<_>>()
            .join("\n"),
    )
}

#[test]
fn ui_modules_do_not_block_the_main_thread() {
    let mut all = Vec::new();
    for p in UI_PATHS {
        files(&src().join(p), &mut all);
    }
    assert!(all.len() > 50, "the lint found only {} files", all.len());
    let mut used = vec![false; ALLOWED.len()];
    let mut violations = Vec::new();
    for file in &all {
        let Some(body) = production(file) else {
            continue;
        };
        let rel = file
            .strip_prefix(src())
            .unwrap_or(file)
            .to_string_lossy()
            .replace('\\', "/");
        for needle in FORBIDDEN {
            if !body.contains(needle) {
                continue;
            }
            match ALLOWED
                .iter()
                .position(|(f, n, _)| *f == rel && n == needle)
            {
                Some(i) => used[i] = true,
                None => violations.push(format!("{rel}: `{needle}`")),
            }
        }
    }
    assert!(
        violations.is_empty(),
        "blocking calls in UI code (move them to a worker, see content_facts and \\
         docs/src/contributing/standards.md):\n{}",
        violations.join("\n")
    );
    let stale: Vec<_> = ALLOWED
        .iter()
        .zip(&used)
        .filter(|(_, u)| !**u)
        .map(|((f, n, _), _)| format!("{f}: `{n}`"))
        .collect();
    assert!(
        stale.is_empty(),
        "stale allowances, delete them:\n{}",
        stale.join("\n")
    );
}

/// The lint itself works: a file with a forbidden call is caught.
#[test]
fn the_lint_sees_a_forbidden_call() {
    let dir = std::env::temp_dir().join(format!("opts-lint-{}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("dir");
    let file = dir.join("bad.rs");
    std::fs::write(&file, "fn f() { let _ = std::fs::read_dir(\".\"); }\n#[cfg(test)]\nmod t { fn g() { std::fs::read(\"x\"); } }").expect("write");
    let body = production(&file).expect("body");
    assert!(body.contains("read_dir("));
    assert!(!body.contains("fs::read(\""), "test code is not linted");
    let _ = std::fs::remove_dir_all(&dir);
}
