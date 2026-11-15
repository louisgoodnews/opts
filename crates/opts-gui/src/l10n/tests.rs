//! Tests of the localisation scaffold: completeness, fallback, plurals, pseudo-locale, live switch
//! and the ban on user-visible literals.

use super::*;

#[test]
fn every_code_key_exists_in_english_and_german() {
    let loc = Localization::new("en");
    for key in keys::ALL {
        assert!(loc.has("en", key), "en.ftl lacks `{key}`");
        assert!(loc.has("de", key), "de.ftl lacks `{key}`");
    }
}

#[test]
fn every_english_key_is_in_german_and_used_in_code() {
    let loc = Localization::new("en");
    for key in loc.english_keys() {
        assert!(loc.has("de", key), "de.ftl lacks `{key}`");
        assert!(
            keys::ALL.contains(&key) || wizard_keys::ALL.contains(&key),
            "`{key}` is in en.ftl but not in keys::ALL"
        );
    }
}

#[test]
fn fallback_goes_locale_language_english_then_marks_the_gap() {
    let mut loc = Localization::new("de-AT");
    assert_eq!(
        loc.t(keys::menu::SETTINGS),
        "Einstellungen",
        "region falls back to the language"
    );
    loc.set_locale("fr");
    assert_eq!(
        loc.t(keys::menu::SETTINGS),
        "Settings",
        "unknown language falls back to English"
    );
    assert_eq!(loc.t("no-such-key"), "⟦no-such-key⟧");
}

#[test]
fn plurals_and_variables_work_in_both_languages() {
    let mut loc = Localization::new("en");
    let n = |c: usize| vec![("count".to_string(), Arg::from(c))];
    assert_eq!(loc.t_args(keys::ITEMS, &n(1)), "1 item");
    assert_eq!(loc.t_args(keys::ITEMS, &n(3)), "3 items");
    loc.set_locale("de");
    assert_eq!(loc.t_args(keys::ITEMS, &n(1)), "1 Element");
    assert_eq!(loc.t_args(keys::ITEMS, &n(0)), "0 Elemente");
    let milestone = vec![("milestone".to_string(), Arg::from("M-0062"))];
    assert_eq!(
        loc.t_args(keys::NOT_AVAILABLE_REASON, &milestone),
        "Funktion noch nicht verfügbar! (M-0062)"
    );
}

#[test]
fn the_pseudo_locale_is_forty_percent_longer_and_keeps_variables() {
    let en = Localization::new("en");
    let xa = Localization::new(pseudo::PSEUDO_TAG);
    for key in keys::ALL {
        let a = en.t_args(
            key,
            &[
                ("milestone".into(), "M-1".into()),
                ("path".into(), "P".into()),
                ("count".into(), 2.into()),
            ],
        );
        let b = xa.t_args(
            key,
            &[
                ("milestone".into(), "M-1".into()),
                ("path".into(), "P".into()),
                ("count".into(), 2.into()),
            ],
        );
        let (la, lb) = (a.chars().count() as f32, b.chars().count() as f32);
        assert!(lb >= la * 1.4, "`{key}`: {la} -> {lb} ({a:?} -> {b:?})");
        assert!(
            b.starts_with('[') && b.ends_with(']'),
            "`{key}` is bracketed"
        );
    }
    assert!(xa
        .t_args(
            keys::NOT_AVAILABLE_REASON,
            &[("milestone".into(), "M-0062".into())]
        )
        .contains("M-0062"));
}

#[test]
fn the_exact_marker_wording_is_english() {
    assert_eq!(
        Localization::new("en").t(keys::NOT_AVAILABLE),
        "Feature not available yet!"
    );
}

#[test]
fn texts_follow_the_locale_live() {
    let mut app = crate::testing::headless_app();
    let e = app
        .world_mut()
        .spawn((Text::new("x"), L10nKey(keys::menu::SHUTDOWN)))
        .id();
    app.update();
    assert_eq!(
        app.world().get::<Text>(e).map(|t| t.0.as_str()),
        Some("Shutdown")
    );
    app.world_mut()
        .resource_mut::<Localization>()
        .set_locale("de");
    app.update();
    assert_eq!(
        app.world().get::<Text>(e).map(|t| t.0.as_str()),
        Some("Beenden")
    );
}

mod literals;
