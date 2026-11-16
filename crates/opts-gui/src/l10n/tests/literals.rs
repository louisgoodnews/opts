//! The ban on user-visible string literals outside the key tables: the scan of the source tree.

fn rust_files(dir: &std::path::Path, out: &mut Vec<std::path::PathBuf>) {
    for e in std::fs::read_dir(dir).into_iter().flatten().flatten() {
        let p = e.path();
        if p.is_dir() {
            rust_files(&p, out);
        } else if p.extension().is_some_and(|x| x == "rs") {
            out.push(p);
        }
    }
}

/// Files that legitimately hold literals: protocol vocabulary, registries, key tables, paths.
const LITERAL_OK_FILES: &[&str] = &[
    "actions.rs",              // `ui.action` spellings
    "ui_tree.rs",              // protocol role names
    "ui_tree/snapshot.rs",     // snapshot role names and state
    "ui_id.rs", "ui_id/assign.rs", "ui_id/sync.rs", // id syntax
    "showcase.rs",             // demo content of the gallery and the CI fixture
    "showcase/sections.rs", "showcase/sections/controls.rs", "showcase/sections/composites.rs", // gallery sections
    "not_yet.rs",              // developer registry text (the user-visible marker text is a key)
    "fonts.rs",                // asset paths
    "icons.rs",                // code points
    "l10n/keys.rs",            // key table
    "l10n/wizard_keys.rs",     // key table of the wizard, the patterns and the loading screen
    "l10n/wizard_keys/wiz/flow.rs", "l10n/wizard_keys/wiz/what.rs", // wizard keys: steps, vehicle, facts
    "l10n/wizard_keys/wiz/when.rs", "l10n/wizard_keys/wiz/review.rs", // wizard keys: time, weather, review
    "l10n/wizard_keys/load.rs", // loading screen keys
    "patterns/glyphs.rs",      // icon code points
    "l10n/mod.rs",             // `⟦key⟧` marker and the env name
    "l10n/pseudo.rs",          // transformation tables
    "l10n/fold.rs",            // diacritics folding tables
    "l10n/fluent.rs",          // parser
];

/// Lines whose literals are not user-visible.
fn line_is_exempt(line: &str) -> bool {
    [
        "Name::new",
        "tracing::",
        "debug!(",
        "info!(",
        "warn!(",
        "error!(",
        "expect(",
        "include_bytes!",
        "include_str!",
        "#[",
        "format!(\"{",
        "// l10n-ok",
    ]
    .iter()
    .any(|m| line.contains(m))
}

#[test]
fn no_user_visible_literals_outside_key_tables() {
    let src = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    let mut files = Vec::new();
    rust_files(&src, &mut files);
    let mut bad = Vec::new();
    for f in files {
        let rel = f
            .strip_prefix(&src)
            .unwrap_or(&f)
            .to_string_lossy()
            .replace('\\', "/");
        if LITERAL_OK_FILES.contains(&rel.as_str())
            || rel.ends_with("tests.rs")
            || rel.contains("/tests/")
            || rel.starts_with("testing/")
        {
            continue;
        }
        let text = std::fs::read_to_string(&f).unwrap_or_default();
        let body = text.split("#[cfg(test)]").next().unwrap_or("");
        for (n, line) in body.lines().enumerate() {
            let code = line.split("//").next().unwrap_or("");
            if line_is_exempt(line) || line.trim_start().starts_with("//") {
                continue;
            }
            let mut rest = code;
            while let Some(i) = rest.find('"') {
                let after = &rest[i + 1..];
                let Some(j) = after.find('"') else { break };
                let lit = &after[..j];
                if lit.chars().filter(|c| c.is_alphabetic()).count() >= 3
                    && lit.contains(char::is_alphabetic)
                {
                    bad.push(format!("{rel}:{}: {lit:?}", n + 1));
                }
                rest = &after[j + 1..];
            }
        }
    }
    assert!(
        bad.is_empty(),
        "user-visible string literals in opts-gui (use a message key, or mark `// l10n-ok: reason`):\n{}",
        bad.join("\n")
    );
}
