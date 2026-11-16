//! Source lint of the wizard and loading modules: no hard-coded colours or visible literals.

use super::*;

/// Source text without its tests.
fn source_of(path: &str) -> String {
    let p = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join(path);
    let text = std::fs::read_to_string(p).unwrap_or_default();
    text.split("#[cfg(test)]").next().unwrap_or("").to_string()
}

#[test]
fn the_wizard_and_loading_modules_hold_no_hard_coded_colours_or_visible_literals() {
    let mut files: Vec<String> = std::fs::read_dir(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src/new_game_screen"),
    )
    .expect("module")
    .flatten()
    .map(|e| format!("src/new_game_screen/{}", e.file_name().to_string_lossy()))
    .filter(|f| f.ends_with(".rs"))
    .collect();
    files.extend(
        [
            "src/in_game/loading_ui.rs",
            "src/in_game/loading_pipeline.rs",
            "src/in_game/load_report.rs",
        ]
        .map(String::from),
    );
    let mut bad = Vec::new();
    for f in &files {
        for (n, line) in source_of(f).lines().enumerate() {
            let code = line.split("//").next().unwrap_or("");
            for banned in [
                "Color::srgb",
                "Color::rgb",
                "Color::hsl",
                "Color::BLACK",
                "Color::WHITE",
                "Srgba::",
                "LinearRgba::",
            ] {
                if code.contains(banned) {
                    bad.push(format!("{f}:{}: {}", n + 1, code.trim()));
                }
            }
            if let Some(i) = code.find("Text::new(\"") {
                let lit = &code[i + 11..];
                if lit
                    .chars()
                    .take_while(|c| *c != '"')
                    .filter(|c| c.is_alphabetic())
                    .count()
                    >= 3
                {
                    bad.push(format!("{f}:{}: literal text {}", n + 1, code.trim()));
                }
            }
        }
    }
    assert!(bad.is_empty(), "{}", bad.join("\n"));
}
