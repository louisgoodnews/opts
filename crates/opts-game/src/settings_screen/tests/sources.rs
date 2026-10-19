//! Source scans: colours, message keys and locales.

use super::*;

fn module_sources() -> Vec<(String, String)> {
    let base = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    let mut out = Vec::new();
    for dir in ["settings_screen", "settings_registry"] {
        for e in std::fs::read_dir(base.join(dir))
            .into_iter()
            .flatten()
            .flatten()
        {
            let p = e.path();
            let name = p
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("")
                .to_string();
            if name.ends_with(".rs") && name != "tests.rs" {
                let text = std::fs::read_to_string(&p).unwrap_or_default();
                out.push((
                    format!("{dir}/{name}"),
                    text.split("#[cfg(test)]").next().unwrap_or("").to_string(),
                ));
            }
        }
    }
    out
}

/// The CI rule "no hard-coded colours or literals" extended to the settings modules.
#[test]
fn settings_modules_have_no_hardcoded_colours_or_literal_texts() {
    let banned = [
        "Color::srgb",
        "Color::rgb",
        "Color::hsl",
        "Color::WHITE",
        "Color::BLACK",
        "Srgba::",
        "LinearRgba::",
        "Color::hex",
        "BackgroundColor(",
        "TextColor(Color",
    ];
    let mut bad = Vec::new();
    for (file, body) in module_sources() {
        for (n, line) in body.lines().enumerate() {
            let code = line.split("//").next().unwrap_or("");
            if banned.iter().any(|b| code.contains(b)) {
                bad.push(format!(
                    "{file}:{}: hard-coded colour: {}",
                    n + 1,
                    code.trim()
                ));
            }
            // A text built from a literal sentence instead of a message key.
            for marker in ["Text::new(\"", "ToastBuilder::new(\""] {
                if let Some(i) = code.find(marker) {
                    let lit: String = code[i + marker.len()..]
                        .chars()
                        .take_while(|c| *c != '"')
                        .collect();
                    if lit.contains(' ') || lit.chars().any(char::is_uppercase) {
                        bad.push(format!("{file}:{}: literal text {lit:?}", n + 1));
                    }
                }
            }
        }
    }
    assert!(bad.is_empty(), "{}", bad.join("\n"));
}

#[test]
fn every_message_key_the_page_uses_exists_in_both_languages() {
    let loc = crate::settings_registry::localization("en");
    let mut missing = Vec::new();
    for (file, body) in module_sources() {
        for line in body.lines() {
            let code = line.split("//").next().unwrap_or("");
            for part in code.split('"').skip(1).step_by(2) {
                let is_key = (part.starts_with("settings-") || part.starts_with("set-"))
                    && part
                        .chars()
                        .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-');
                if is_key && !(loc.has("en", part) && loc.has("de", part)) {
                    missing.push(format!("{file}: {part}"));
                }
            }
        }
    }
    assert!(missing.is_empty(), "{missing:?}");
}

#[test]
fn both_languages_have_exactly_the_same_settings_messages() {
    let en = opts_gui::l10n::fluent::parse(include_str!("../../settings_registry/messages_en.ftl"));
    let de = opts_gui::l10n::fluent::parse(include_str!("../../settings_registry/messages_de.ftl"));
    let mut a: Vec<_> = en.messages.keys().cloned().collect();
    let mut b: Vec<_> = de.messages.keys().cloned().collect();
    a.sort();
    b.sort();
    let only_en: Vec<_> = a.iter().filter(|k| !b.contains(k)).collect();
    let only_de: Vec<_> = b.iter().filter(|k| !a.contains(k)).collect();
    assert!(
        only_en.is_empty() && only_de.is_empty(),
        "only en: {only_en:?}, only de: {only_de:?}"
    );
}

/// Pseudo-locale: the lengthened texts of a page must still be laid out inside the window.
#[test]
fn the_pseudo_locale_page_stays_inside_the_window_width() {
    let (mut app, _, _) = page_app("pseudo", "interface");
    app.world_mut()
        .resource_mut::<opts_gui::l10n::Localization>()
        .set_locale("en-XA");
    for _ in 0..4 {
        app.update();
    }
    let mut q = app
        .world_mut()
        .query::<(&ComputedNode, &UiGlobalTransform, &Text)>();
    let mut wide = Vec::new();
    for (node, tf, text) in q.iter(app.world()) {
        let right = tf.translation.x + node.size().x / 2.0;
        // The test window is 1280 wide at scale 1 in `layout_app`.
        if right > 1281.0 * node.inverse_scale_factor().recip() {
            wide.push((text.0.clone(), right));
        }
    }
    assert!(wide.is_empty(), "texts beyond the window: {wide:?}");
}
