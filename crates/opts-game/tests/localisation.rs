//! German and English are complete over **every** message table the application loads (the
//! library's, the wizard's, the shell's and the settings registry's), the two languages agree on
//! their variables, and the pseudo-locale `en-XA` really lengthens every message.

use bevy::prelude::*;
use opts_gui::l10n::{Localization, PSEUDO_TAG};

/// The localisation as the application assembles it: the library tables plus what the shell and
/// the settings registry add.
fn whole_application() -> Localization {
    opts_game::isolation::isolate_user_dirs();
    let mut app = App::new();
    app.add_plugins(MinimalPlugins)
        .add_plugins(bevy::state::app::StatesPlugin)
        .add_plugins(bevy::input::InputPlugin)
        .add_plugins(AssetPlugin::default())
        .init_asset::<Image>()
        .insert_resource(opts_game::settings::GameSettings::default())
        .init_state::<opts_game::app_state::AppState>()
        .add_plugins(opts_gui::OptsGuiPlugin)
        .add_plugins(opts_game::shell::ShellPlugin);
    let mut loc = app.world().resource::<Localization>().clone();
    opts_game::settings_registry::add_messages(&mut loc);
    loc
}

#[test]
fn german_and_english_have_exactly_the_same_keys() {
    let loc = whole_application();
    let en = loc.keys_of("en");
    let de = loc.keys_of("de");
    assert!(en.len() > 300, "only {} English messages", en.len());
    let missing_de: Vec<_> = en.iter().filter(|k| !de.contains(k)).collect();
    let missing_en: Vec<_> = de.iter().filter(|k| !en.contains(k)).collect();
    assert!(
        missing_de.is_empty(),
        "not translated into German: {missing_de:?}"
    );
    assert!(
        missing_en.is_empty(),
        "German keys without English source: {missing_en:?}"
    );
}

#[test]
fn german_and_english_use_the_same_variables() {
    let loc = whole_application();
    let mut bad = Vec::new();
    for key in loc.keys_of("en") {
        let (en, de) = (loc.variables_of("en", key), loc.variables_of("de", key));
        if en != de {
            bad.push(format!("{key}: en {en:?} vs de {de:?}"));
        }
    }
    assert!(bad.is_empty(), "{}", bad.join("\n"));
}

#[test]
fn no_message_is_empty_or_a_placeholder() {
    let loc = whole_application();
    for locale in ["en", "de"] {
        for key in loc.keys_of(locale) {
            let text = loc.format_in(locale, key);
            assert!(!text.trim().is_empty(), "{locale}: `{key}` is empty");
            assert!(
                !text.contains("⟦"),
                "{locale}: `{key}` resolves to a gap marker"
            );
            for marker in ["TODO", "FIXME", "XXX", "lorem"] {
                assert!(
                    !text.contains(marker),
                    "{locale}: `{key}` contains {marker}"
                );
            }
        }
    }
}

/// Messages that are the same in both languages on purpose (names, symbols, loanwords). Every
/// other identical pair is an untranslated string. Keep the list short and justified.
const SAME_IN_BOTH: &[&str] = &[];

#[test]
fn german_is_translated_not_copied() {
    let loc = whole_application();
    let mut same = Vec::new();
    for key in loc.keys_of("en") {
        let (en, de) = (loc.format_in("en", key), loc.format_in("de", key));
        // One word or less, or digits and symbols only: names and units are legitimately equal.
        let wordy = en.split_whitespace().count() >= 2 && en.chars().any(char::is_alphabetic);
        if wordy && en == de && !SAME_IN_BOTH.contains(&key) {
            same.push(format!("{key} = {en}"));
        }
    }
    assert!(
        same.is_empty(),
        "identical in English and German:\n{}",
        same.join("\n")
    );
}

#[test]
fn the_pseudo_locale_covers_everything_and_lengthens_by_at_least_forty_percent() {
    let loc = whole_application();
    assert!(loc.available().contains(&PSEUDO_TAG));
    let mut short = Vec::new();
    for key in loc.keys_of("en") {
        assert!(loc.has(PSEUDO_TAG, key), "pseudo lacks `{key}`");
        let (en, xa) = (loc.format_in("en", key), loc.format_in(PSEUDO_TAG, key));
        // Messages with a plural selector pad each variant; compare the rendered text.
        let visible = |t: &str| {
            let mut n = 0;
            let mut depth = 0;
            for c in t.chars() {
                match c {
                    '{' => depth += 1,
                    '}' => depth -= 1,
                    _ if depth == 0 => n += 1,
                    _ => {}
                }
            }
            n
        };
        let (a, b) = (visible(&en), visible(&xa));
        // brackets and padding: [ text ~~~ ]
        if a > 0 && (b as f32) < a as f32 * 1.4 {
            short.push(format!("{key}: {a} -> {b}"));
        }
    }
    assert!(
        short.is_empty(),
        "pseudo-locale too short:\n{}",
        short.join("\n")
    );
}
