//! The message tables that ship with the program, as the command-line tools see them.
//!
//! The game loads the very same files through `include_str!` in `opts-gui` and `opts-game`; the
//! test `builtin_lists_match_the_loaded_tables` in `opts-game` fails when this list and the
//! loaded tables drift apart (a new `.ftl` file must be added here).

use super::check::Reference;
use super::fluent::{self, Table};
use super::pack::{Direction, PackMeta};

/// English sources: `(pack file name, text)`. The name is also the file name a scaffold gets.
pub const ENGLISH: &[(&str, &str)] = &[
    ("gui.ftl", include_str!("../../opts-gui/src/l10n/en.ftl")),
    (
        "wizard.ftl",
        include_str!("../../opts-gui/src/l10n/wizard_en.ftl"),
    ),
    (
        "shell.ftl",
        include_str!("../../opts-game/src/shell/l10n/shell-en.ftl"),
    ),
    (
        "settings.ftl",
        include_str!("../../opts-game/src/settings_registry/messages_en.ftl"),
    ),
    (
        "settings-lighting.ftl",
        include_str!("../../opts-game/src/settings_registry/messages_lighting_en.ftl"),
    ),
    (
        "settings-keys.ftl",
        include_str!("../../opts-game/src/settings_registry/messages_keys_en.ftl"),
    ),
    (
        "hud.ftl",
        include_str!("../../opts-game/src/hud/l10n/hud-en.ftl"),
    ),
];

/// German sources, same names as [`ENGLISH`].
pub const GERMAN: &[(&str, &str)] = &[
    ("gui.ftl", include_str!("../../opts-gui/src/l10n/de.ftl")),
    (
        "wizard.ftl",
        include_str!("../../opts-gui/src/l10n/wizard_de.ftl"),
    ),
    (
        "shell.ftl",
        include_str!("../../opts-game/src/shell/l10n/shell-de.ftl"),
    ),
    (
        "settings.ftl",
        include_str!("../../opts-game/src/settings_registry/messages_de.ftl"),
    ),
    (
        "settings-lighting.ftl",
        include_str!("../../opts-game/src/settings_registry/messages_lighting_de.ftl"),
    ),
    (
        "settings-keys.ftl",
        include_str!("../../opts-game/src/settings_registry/messages_keys_de.ftl"),
    ),
    (
        "hud.ftl",
        include_str!("../../opts-game/src/hud/l10n/hud-de.ftl"),
    ),
];

/// The English reference.
#[must_use]
pub fn english() -> Reference {
    Reference::from_sources(ENGLISH)
}

/// The built-in table of `code` (`en`, `de`), merged over all files; `None` for other codes.
#[must_use]
pub fn table_of(code: &str) -> Option<Table> {
    let sources = match code {
        "en" => ENGLISH,
        "de" => GERMAN,
        _ => return None,
    };
    let mut table = Table::default();
    for (_, text) in sources {
        let t = fluent::parse(text);
        for k in &t.order {
            if !table.messages.contains_key(k) {
                table.order.push(k.clone());
            }
        }
        table.messages.extend(t.messages);
        table.lines.extend(t.lines);
        table.comments.extend(t.comments);
    }
    Some(table)
}

/// The identity of a built-in language.
#[must_use]
pub fn meta_of(code: &str) -> Option<PackMeta> {
    let (name, native) = match code {
        "en" => ("English", "English"),
        "de" => ("German", "Deutsch"),
        _ => return None,
    };
    Some(PackMeta {
        code: code.to_string(),
        name: name.to_string(),
        native_name: native.to_string(),
        direction: Direction::Ltr,
        version: env!("CARGO_PKG_VERSION").to_string(),
        authors: vec!["OPTS contributors".to_string()],
        license: "MIT OR Apache-2.0".to_string(),
        base_version: english().revision(),
    })
}

/// The codes of the built-in languages.
pub const CODES: &[&str] = &["en", "de"];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shipped_tables_parse_without_a_single_diagnostic() {
        for (lang, sources) in [("en", ENGLISH), ("de", GERMAN)] {
            for (name, text) in sources {
                let t = fluent::parse(text);
                assert!(
                    t.diagnostics.is_empty(),
                    "{lang} {name}: {:?}",
                    t.diagnostics
                );
            }
        }
    }

    #[test]
    fn german_passes_the_pack_checks_against_english() {
        let reference = english();
        let de = table_of("de").unwrap();
        let checked = super::super::check::validate(&de, "de", "de", &reference);
        let errors: Vec<_> = checked
            .issues
            .iter()
            .filter(|i| i.severity == super::super::check::Severity::Error)
            .collect();
        assert!(errors.is_empty(), "{errors:?}");
        let s = super::super::check::stats(&reference, &checked.table, "de");
        assert!(s.missing.is_empty(), "{:?}", s.missing);
        assert!(s.percent() > 98.0, "{}", s.percent());
    }
}
