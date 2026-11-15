use super::*;

#[test]
fn a_pack_adds_a_language_and_gaps_show_english() {
    let root = scratch("add");
    write(&root, "fr/pack.json", &meta("fr", "French", "Français", ""));
    write(
        &root,
        "fr/gui.ftl",
        "widget-clear = Effacer\nmenu-settings = Paramètres\n",
    );
    let mut loc = full_localization();
    assert_eq!(loc.load_packs(&root), 1);
    loc.set_locale("fr");
    assert_eq!(loc.t(CLEAR), "Effacer");
    assert_eq!(loc.t(SETTINGS), "Paramètres");
    // Not translated: English, per key, and the gap is counted.
    assert_eq!(loc.t("menu-shutdown"), "Shutdown");
    let fr = loc
        .languages()
        .into_iter()
        .find(|l| l.code == "fr")
        .expect("fr listed");
    assert_eq!(fr.native_name, "Français");
    assert_eq!(fr.translated, 2);
    assert_eq!(fr.missing, fr.total - 2);
    assert!(fr.percent() > 0.0 && fr.percent() < 5.0);
    assert!(fr.selectable && !fr.builtin && fr.from_pack);
    let report = loc.language_report("fr").expect("report");
    assert!(report.missing_keys.contains(&"menu-shutdown".to_string()));
    assert!(!report.missing_keys.contains(&CLEAR.to_string()));
    // A region falls back to the pack's language.
    loc.set_locale("fr-CA");
    assert_eq!(loc.t(CLEAR), "Effacer");
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn a_pack_with_a_built_in_tag_overrides_single_messages() {
    let root = scratch("override");
    write(&root, "de/pack.json", &meta("de", "German", "Deutsch", ""));
    write(&root, "de/mine.ftl", "widget-clear = Leeren\n");
    let mut loc = full_localization();
    let before = loc.clone();
    loc.load_packs(&root);
    loc.set_locale("de");
    assert_eq!(loc.t(CLEAR), "Leeren");
    assert_eq!(
        loc.t(SETTINGS),
        before.format_in("de", SETTINGS),
        "the rest stays German"
    );
    let de = loc
        .languages()
        .into_iter()
        .find(|l| l.code == "de")
        .expect("de");
    assert!(de.builtin && de.from_pack);
    assert!(de.percent() > 95.0);
    // Dropping the packs restores the shipped text.
    loc.clear_packs();
    assert_eq!(loc.t(CLEAR), before.format_in("de", CLEAR));
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn broken_files_never_crash_and_good_messages_survive() {
    let root = scratch("broken");
    // 1. good pack with one syntax error, one unknown key, one lost variable, one empty message
    write(&root, "xx/pack.json", &meta("xx", "Test", "Tëst", ""));
    write(
        &root,
        "xx/a.ftl",
        "widget-clear = Gut\nwidget-not-available-yet = { $oops\nnot-a-key = x\nwidget-not-available-yet-reason = ohne Variable\nmenu-shutdown =\n",
    );
    // 2. a binary file, a file that is not UTF-8, an oversized one
    std::fs::write(root.join("xx/b.ftl"), [0xff, 0xfe, 0x00, 0x80]).expect("binary");
    write(&root, "xx/c.json", "{ this is not json");
    let big = "a = b\n".repeat(500_000);
    write(&root, "xx/big.ftl", &big);
    // 3. pack.json problems
    write(&root, "nopack/x.ftl", "widget-clear = x\n");
    write(&root, "badjson/pack.json", "{ nope");
    write(&root, "badjson/x.ftl", "widget-clear = x\n");
    write(&root, "wrongcode/pack.json", &meta("yy", "Y", "Y", ""));
    // 4. things that are not packs
    write(&root, ".hidden/pack.json", &meta("zz", "Z", "Z", ""));
    write(&root, "readme.txt", "hello");
    let mut loc = full_localization();
    let usable = loc.load_packs(&root);
    assert_eq!(usable, 1, "only xx is usable");
    loc.set_locale("xx");
    assert_eq!(loc.t(CLEAR), "Gut");
    assert_eq!(
        loc.t("widget-not-available-yet"),
        "Feature not available yet!",
        "skipped: English"
    );
    assert_eq!(loc.t("menu-shutdown"), "Shutdown");
    let r = loc.language_report("xx").expect("report");
    let codes: Vec<(&str, &str)> = r.issues.iter().map(|i| (i.file.as_str(), i.code)).collect();
    for want in [
        ("a.ftl", "syntax"),
        ("a.ftl", "unknown-key"),
        ("a.ftl", "variable-mismatch"),
        ("a.ftl", "empty"),
        ("b.ftl", "read"),
        ("c.json", "syntax"),
        ("big.ftl", "read"),
    ] {
        assert!(codes.contains(&want), "{want:?} not in {codes:?}");
    }
    let syntax = r
        .issues
        .iter()
        .find(|i| i.code == "syntax" && i.file == "a.ftl")
        .expect("syntax");
    assert_eq!(syntax.line, Some(2), "line of the faulty message");
    assert_eq!(syntax.key.as_deref(), Some("widget-not-available-yet"));
    let problems: Vec<String> = loc.pack_problems().into_iter().map(|(f, _)| f).collect();
    assert!(problems.contains(&"nopack".to_string()), "{problems:?}");
    assert!(problems.contains(&"badjson".to_string()), "{problems:?}");
    assert!(problems.contains(&"wrongcode".to_string()), "{problems:?}");
    assert!(!problems.iter().any(|p| p.starts_with('.')));
    // The missing folder and an empty folder are fine too.
    assert_eq!(loc.load_packs(&root.join("does-not-exist")), 0);
    assert_eq!(loc.t(CLEAR), "Clear", "the old pack is gone after a rescan");
    let _ = std::fs::remove_dir_all(&root);
}
