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

/// Directories (below `src/`) and files of the UI.
const UI_PATHS: &[&str] = &[
    "settings_screen",
    "settings_registry",
    "home",
    "help",
    "pause",
    "shell",
    "first_run",
    "menu",
    "new_game_screen",
    "hud",
    "look",
    "save",
    "in_game/loading_ui.rs",
    "in_game/loading_pipeline.rs",
    "in_game/vehicle_menu.rs",
    "omsi_setup.rs",
    "settings_theme.rs",
    "control_ui.rs",
    "shutdown.rs",
];

/// What blocks: file system, SQLite, image decoding, directory probes and the helpers that wrap
/// them.
const FORBIDDEN: &[&str] = &[
    "std::fs",
    "File::open",
    "File::create",
    "File::options",
    "read_dir(",
    ".exists()",
    ".is_dir()",
    ".is_file()",
    "metadata(",
    "Connection::open",
    "Catalogue::open",
    "kind_counts(",
    ".totals()",
    ".last_scan()",
    "image::open",
    "known_omsi_root(",
    "locate_omsi_root(",
    "is_omsi_root(",
    "descramble_cache::",
    "discover_theme_files(",
    "load_theme_file(",
    "DriveHistory::load",
    ".save()",
    ".save_to(",
];

/// `(file below src/, forbidden text, why it is fine)`.
const ALLOWED: &[(&str, &str, &str)] = &[
    ("first_run/mod.rs", "std::fs", "welcome flag: read once at startup, one 20-byte write when the welcome is closed"),
    ("first_run/mod.rs", ".save()", "a confirmed OMSI folder: one user action, the failure is shown as a banner"),
    ("home/hero.rs", "known_omsi_root(", "inside the IoTaskPool task that decodes the picture"),
    ("home/mod.rs", "DriveHistory::load", "Startup system `load_drive_history`"),
    ("home/mod.rs", ".save()", "inside the `drive-history` writer thread"),
    ("hud/profile.rs", "std::fs", "user HUD profiles: read when the plugin is built (startup)"),
    ("hud/viewpoint.rs", ".save()", "stores the in-memory viewpoint; the file is a few hundred bytes, written on the user's click"),
    ("menu/background.rs", ".exists()", "`decode_map_picture`: called from a worker only"),
    ("menu/background.rs", "image::open", "`decode_map_picture`: called from a worker only"),
    ("new_game_screen/history.rs", "DriveHistory::load", "resource initialisation at startup"),
    ("new_game_screen/history.rs", ".save_to(", "the planner history: one small file, written on a click (star, start)"),
    ("new_game_screen/history.rs", ".save()", "same: the planner's own `save`"),
    ("new_game_screen/repaints.rs", "std::fs", "repaint discovery: runs inside the IoTaskPool task of `livery::request`"),
    ("new_game_screen/repaints.rs", ".is_dir()", "same task"),
    ("new_game_screen/repaints.rs", "read_dir(", "same task"),
    ("new_game_screen/repaints.rs", "known_omsi_root(", "same task"),
    ("new_game_screen/thumbnails.rs", "std::fs", "decode, cache and preview probing: inside IoTaskPool tasks (`request`, `request_probe`)"),
    ("new_game_screen/thumbnails.rs", "File::options", "same tasks"),
    ("new_game_screen/thumbnails.rs", "metadata(", "same tasks"),
    ("new_game_screen/thumbnails.rs", "image::open", "same tasks"),
    ("new_game_screen/thumbnails.rs", "read_dir(", "same tasks"),
    ("new_game_screen/thumbnails.rs", ".is_file()", "`Probe::resolve`: runs inside the task"),
    ("omsi_setup.rs", ".is_dir()", "Startup system"),
    ("omsi_setup.rs", "is_omsi_root(", "Startup system"),
    ("omsi_setup.rs", ".save()", "Startup system"),
    ("settings_screen/content.rs", "descramble_cache::", "inside the `content-job` thread (`run_then_invalidate`)"),
    ("settings_screen/info.rs", "descramble_cache::", "`directory()` only builds a path from the environment"),
    ("settings_screen/input.rs", ".save()", "`SettingsApi::save`: hands the copy to the settings writer thread"),
    ("settings_screen/scan_status.rs", "std::fs", "`folder_changed_since`: called by the facts worker only"),
    ("settings_screen/scan_status.rs", "read_dir(", "same"),
    ("settings_screen/scan_status.rs", "metadata(", "same"),
    ("settings_screen/scan_status.rs", ".is_dir()", "same"),
    ("settings_theme.rs", "load_theme_file(", "the user picked another theme file: one small file, only when that setting changes"),
    ("shutdown.rs", ".save()", "application exit"),
];

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
        let Some(body) = production(file) else { continue };
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
    assert!(stale.is_empty(), "stale allowances, delete them:\n{}", stale.join("\n"));
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
