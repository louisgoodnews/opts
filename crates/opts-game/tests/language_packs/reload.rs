//! Reloading packs, right-to-left and uncovered scripts, and plurals.


use super::*;


#[test]
pub(super) fn reload_picks_up_changes_and_a_removed_active_pack_falls_back_to_english() {
    let root = scratch("reload");
    let mut loc = full_localization();
    assert_eq!(language_packs::scan_dir(&mut loc, &root).usable, 0);
    write(
        &root,
        "it/pack.json",
        &meta("it", "Italian", "Italiano", ""),
    );
    write(&root, "it/s.ftl", "widget-clear = Cancella\n");
    assert_eq!(language_packs::scan_dir(&mut loc, &root).usable, 1);
    loc.set_locale("it");
    assert_eq!(loc.t(CLEAR), "Cancella");
    write(&root, "it/s.ftl", "widget-clear = Pulisci\n");
    language_packs::scan_dir(&mut loc, &root);
    assert_eq!(loc.t(CLEAR), "Pulisci");
    std::fs::remove_dir_all(root.join("it")).expect("remove");
    language_packs::scan_dir(&mut loc, &root);
    assert_eq!(loc.locale(), "en", "the active language is gone");
    assert_eq!(loc.t(CLEAR), "Clear");
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
pub(super) fn right_to_left_and_uncovered_scripts_are_reported_not_hidden() {
    let root = scratch("rtl");
    write(
        &root,
        "ar/pack.json",
        &meta("ar", "Arabic", "العربية", ",\"direction\":\"rtl\""),
    );
    write(&root, "ar/s.ftl", "widget-clear = مسح\n");
    write(&root, "ja/pack.json", &meta("ja", "Japanese", "日本語", ""));
    write(&root, "ja/s.ftl", "widget-clear = クリア\n");
    let mut loc = full_localization();
    loc.load_packs(&root);
    let l = loc.languages();
    let ar = l.iter().find(|x| x.code == "ar").expect("ar");
    assert!(!ar.selectable);
    assert!(ar.missing_fonts.iter().any(|s| s == "Arabic"));
    assert!(!loc.can_select("ar"));
    let ja = l.iter().find(|x| x.code == "ja").expect("ja");
    assert!(
        ja.selectable,
        "left to right is selectable even without a font"
    );
    assert!(ja.missing_fonts.iter().any(|s| s == "Kana"));
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
pub(super) fn plurals_follow_the_rules_of_the_pack_language() {
    let root = scratch("plural");
    write(&root, "ru/pack.json", &meta("ru", "Russian", "Русский", ""));
    write(
        &root,
        "ru/s.ftl",
        "widget-items =\n    { $count ->\n        [one] { $count } элемент\n        [few] { $count } элемента\n        [many] { $count } элементов\n       *[other] { $count } элемента\n    }\n",
    );
    let mut loc = full_localization();
    loc.load_packs(&root);
    loc.set_locale("ru");
    let n = |c: usize| vec![("count".to_string(), c.into())];
    assert_eq!(loc.t_args("widget-items", &n(1)), "1 элемент");
    assert_eq!(loc.t_args("widget-items", &n(3)), "3 элемента");
    assert_eq!(loc.t_args("widget-items", &n(5)), "5 элементов");
    assert_eq!(loc.t_args("widget-items", &n(21)), "21 элемент");
    let _ = std::fs::remove_dir_all(&root);
}
