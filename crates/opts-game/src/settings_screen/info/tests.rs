//! Unit tests for `super` (`info.rs`).

use super::*;
use opts_gui::l10n::Localization;

fn render(locale: &str, key: &str, args: &Args) -> String {
    let mut loc = Localization::new(locale);
    loc.extend(
        "en",
        include_str!("../../settings_registry/messages_en.ftl"),
    );
    loc.extend(
        "de",
        include_str!("../../settings_registry/messages_de.ftl"),
    );
    let a: Vec<(String, opts_gui::l10n::fluent::Arg)> = args
        .iter()
        .map(|(k, v)| ((*k).to_string(), v.clone().into()))
        .collect();
    loc.t_args(key, &a)
}

#[test]
fn the_catalogue_summary_names_each_kind_and_explains_files_versus_entries() {
    let counts = opts_catalogue::KindCounts {
        files: 679_543,
        entities: 75_313,
        maps: 43,
        vehicles: 947,
        scenery_objects: 20_000,
        splines: 1_500,
        other: 52_823,
    };
    let (kinds, total) = catalogue_args(&counts);
    for locale in ["en", "de"] {
        let line = render(locale, "settings-info-catalogue", &kinds);
        for n in ["43", "947", "20000", "1500", "52823"] {
            assert!(line.contains(n), "{locale}: `{line}` lacks {n}");
        }
        assert!(
            !line.contains("⟦"),
            "{locale}: missing variable in `{line}`"
        );
        let t = render(locale, "settings-info-catalogue-total", &total);
        assert!(t.contains("75313") && t.contains("679543"), "{locale}: {t}");
        assert!(!t.contains("⟦"));
    }
    assert!(render("en", "settings-info-catalogue", &kinds).contains("43 maps, 947 vehicles"));
    assert!(render("de", "settings-info-catalogue", &kinds).contains("43 Karten, 947 Fahrzeuge"));
}
