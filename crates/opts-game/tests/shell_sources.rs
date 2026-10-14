//! Source rules for the shell modules (`docs/src/roadmap/phase-9/expandability.md` section 7):
//! no user-visible string literals, no literal colours, every registered key translated, and the
//! size limits of `docs/src/contributing/standards.md`.

use std::path::{Path, PathBuf};

const MODULES: &[&str] = &[
    "shell",
    "home",
    "hud",
    "help",
    "first_run",
    "pause",
    "session_summary",
    "save",
];

fn rust_files(dir: &Path, out: &mut Vec<PathBuf>) {
    for entry in std::fs::read_dir(dir).into_iter().flatten().flatten() {
        let path = entry.path();
        if path.is_dir() {
            rust_files(&path, out);
        } else if path.extension().is_some_and(|e| e == "rs") {
            out.push(path);
        }
    }
}

fn sources() -> Vec<(String, String)> {
    let src = Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    let mut files = Vec::new();
    for module in MODULES {
        rust_files(&src.join(module), &mut files);
    }
    files
        .into_iter()
        .filter(|f| f.file_name().is_some_and(|n| n != "tests.rs"))
        .map(|f| {
            let text = std::fs::read_to_string(&f).expect("readable source");
            (
                f.strip_prefix(&src)
                    .expect("below src")
                    .display()
                    .to_string(),
                text,
            )
        })
        .collect()
}

/// The code of a file: everything before its test module.
fn code(text: &str) -> &str {
    text.split("#[cfg(test)]").next().unwrap_or(text)
}

#[test]
fn no_user_visible_string_literals_in_the_shell_modules() {
    let needles = [
        "Text::new(\"",
        "ButtonBuilder::filled(\"",
        "ButtonBuilder::tonal(\"",
        "ButtonBuilder::outlined(\"",
        "ButtonBuilder::text(\"",
        "with_placeholder(\"",
        "ToastBuilder::new(\"",
    ];
    let mut bad = Vec::new();
    for (name, text) in sources() {
        if name.ends_with("keys.rs") {
            continue; // the key tables may hold literals
        }
        for (n, line) in code(&text).lines().enumerate() {
            if !line.contains("l10n-ok") && needles.iter().any(|p| line.contains(p)) {
                bad.push(format!("{name}:{}: {}", n + 1, line.trim()));
            }
        }
    }
    assert!(bad.is_empty(), "use message keys:\n{}", bad.join("\n"));
}

#[test]
fn no_literal_colours_in_the_shell_modules() {
    let needles = [
        "Color::srgb",
        "Color::srgba",
        "Color::hsl",
        "Color::hsla",
        "Color::WHITE",
        "Color::BLACK",
        "Srgba::",
        "LinearRgba",
    ];
    let mut bad = Vec::new();
    for (name, text) in sources() {
        for (n, line) in code(&text).lines().enumerate() {
            if needles.iter().any(|p| line.contains(p)) {
                bad.push(format!("{name}:{}: {}", n + 1, line.trim()));
            }
        }
    }
    assert!(bad.is_empty(), "use role tokens:\n{}", bad.join("\n"));
}

#[test]
fn files_stay_within_500_lines_and_have_a_module_header() {
    let mut bad = Vec::new();
    for (name, text) in sources() {
        let code_lines = text.lines().count();
        if code_lines > 500 {
            bad.push(format!("{name}: {code_lines} lines"));
        }
        if !text.starts_with("//!") {
            bad.push(format!("{name}: no `//!` header"));
        }
    }
    assert!(bad.is_empty(), "{}", bad.join("\n"));
}
