//! Hard-coded colours in the library source outside the token and theme files.

use bevy::prelude::*;
use std::path::Path;

/// Hard-coded colours in library source outside the token and theme files.
pub fn hardcoded_colours(src_dir: &Path) -> Vec<String> {
    const BANNED: &[&str] = &[
        "Color::srgb",
        "Color::srgba",
        "Color::rgb",
        "Color::rgba",
        "Color::hsl",
        "Color::hsla",
        "Color::linear",
        "Color::BLACK",
        "Color::WHITE",
        "Srgba::",
        "LinearRgba::",
        "Color::hex",
    ];
    fn walk(dir: &Path, out: &mut Vec<std::path::PathBuf>) {
        for e in std::fs::read_dir(dir).into_iter().flatten().flatten() {
            let p = e.path();
            if p.is_dir() {
                walk(&p, out);
            } else if p.extension().is_some_and(|x| x == "rs") {
                out.push(p);
            }
        }
    }
    let mut files = Vec::new();
    walk(src_dir, &mut files);
    let mut bad = Vec::new();
    for f in files {
        let rel = f
            .strip_prefix(src_dir)
            .unwrap_or(&f)
            .to_string_lossy()
            .replace('\\', "/");
        if rel.starts_with("theme/") || rel.starts_with("tokens/") || rel.starts_with("testing/") {
            continue;
        }
        let text = std::fs::read_to_string(&f).unwrap_or_default();
        let body = text.split("#[cfg(test)]").next().unwrap_or("");
        for (n, line) in body.lines().enumerate() {
            let code = line.split("//").next().unwrap_or("");
            if BANNED.iter().any(|b| code.contains(b)) {
                bad.push(format!("{rel}:{}: {}", n + 1, code.trim()));
            }
        }
    }
    bad
}
