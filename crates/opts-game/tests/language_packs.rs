//! Language packs: discovery in a folder, override of a built-in language, per-key fallback to
//! English, tolerance of broken files, reload, the language list and the control methods.
//!
//! Packs are written below `~/.local/share/opts/benchmark/langpacks/` (never `/tmp`, never the
//! user's own folders); every test cleans its folder.

use std::path::{Path, PathBuf};

use bevy::prelude::*;
use opts_bevy::control::state::{ControlRegistry, Outcome};
use opts_core::l10n::pack::EXPECTED_LICENSE;
use opts_game::language_packs;
use opts_game::settings::GameSettings;
use opts_game::settings_registry::{SettingChanged, SettingsCapabilities, SettingsRegistry};
use opts_gui::l10n::Localization;
use serde_json::{json, Value};

fn scratch(name: &str) -> PathBuf {
    let dir = std::env::var_os("HOME")
        .map(PathBuf::from)
        .expect("HOME")
        .join(".local/share/opts/benchmark/langpacks/tests")
        .join(name);
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("scratch dir");
    dir
}

fn meta(code: &str, name: &str, native: &str, extra: &str) -> String {
    format!(
        "{{\"code\":\"{code}\",\"name\":\"{name}\",\"native_name\":\"{native}\",\"version\":\"1\",\"authors\":[\"Tester\"],\"license\":\"{EXPECTED_LICENSE}\"{extra}}}"
    )
}

fn write(dir: &Path, rel: &str, text: &str) {
    let p = dir.join(rel);
    std::fs::create_dir_all(p.parent().expect("parent")).expect("mkdir");
    std::fs::write(p, text).expect("write");
}

/// A localisation with every table the application loads.
fn full_localization() -> Localization {
    let mut loc = Localization::new("en");
    opts_game::settings_registry::add_messages(&mut loc);
    opts_game::hud::add_messages(&mut loc);
    loc
}

const CLEAR: &str = "widget-clear";
const SETTINGS: &str = "menu-settings";

#[test]
fn a_pack_adds_a_language_and_gaps_show_english() {
    let root = scratch("add");
    write(&root, "fr/pack.json", &meta("fr", "French", "Français", ""));
    write(&root, "fr/gui.ftl", "widget-clear = Effacer\nmenu-settings = Paramètres\n");
    let mut loc = full_localization();
    assert_eq!(loc.load_packs(&root), 1);
    loc.set_locale("fr");
    assert_eq!(loc.t(CLEAR), "Effacer");
    assert_eq!(loc.t(SETTINGS), "Paramètres");
    // Not translated: English, per key, and the gap is counted.
    assert_eq!(loc.t("menu-shutdown"), "Shutdown");
    let fr = loc.languages().into_iter().find(|l| l.code == "fr").expect("fr listed");
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
    assert_eq!(loc.t(SETTINGS), before.format_in("de", SETTINGS), "the rest stays German");
    let de = loc.languages().into_iter().find(|l| l.code == "de").expect("de");
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
    assert_eq!(loc.t("widget-not-available-yet"), "Feature not available yet!", "skipped: English");
    assert_eq!(loc.t("menu-shutdown"), "Shutdown");
    let r = loc.language_report("xx").expect("report");
    let codes: Vec<(&str, &str)> = r
        .issues
        .iter()
        .map(|i| (i.file.as_str(), i.code))
        .collect();
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
    let syntax = r.issues.iter().find(|i| i.code == "syntax" && i.file == "a.ftl").expect("syntax");
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

#[test]
fn reload_picks_up_changes_and_a_removed_active_pack_falls_back_to_english() {
    let root = scratch("reload");
    let mut loc = full_localization();
    assert_eq!(language_packs::scan_dir(&mut loc, &root).usable, 0);
    write(&root, "it/pack.json", &meta("it", "Italian", "Italiano", ""));
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
fn right_to_left_and_uncovered_scripts_are_reported_not_hidden() {
    let root = scratch("rtl");
    write(&root, "ar/pack.json", &meta("ar", "Arabic", "العربية", ",\"direction\":\"rtl\""));
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
    assert!(ja.selectable, "left to right is selectable even without a font");
    assert!(ja.missing_fonts.iter().any(|s| s == "Kana"));
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn plurals_follow_the_rules_of_the_pack_language() {
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

fn call(world: &mut World, method: &str, params: Value) -> Result<Value, opts_cli::protocol::RpcError> {
    let mut registry = ControlRegistry::default();
    language_packs::register(&mut registry);
    let handler = registry.get(method).expect("registered");
    match handler(world, &params) {
        Outcome::Done(r) => r,
        Outcome::Wait(_) => panic!("answers at once"),
    }
}

#[test]
fn the_control_methods_list_set_reload_and_report() {
    let root = scratch("control");
    // `directories` reads XDG_DATA_HOME on every call; this is the only test that sets it.
    std::env::set_var("XDG_DATA_HOME", &root);
    let lang = root.join("opts/lang");
    write(&lang, "fr/pack.json", &meta("fr", "French", "Français", ""));
    write(&lang, "fr/s.ftl", "widget-clear = Effacer\n");
    write(&lang, "ar/pack.json", &meta("ar", "Arabic", "العربية", ",\"direction\":\"rtl\""));
    write(&lang, "ar/s.ftl", "widget-clear = مسح\n");
    write(&lang, "broken/x.ftl", "a = b\n");

    let mut world = World::new();
    let mut s = GameSettings::default();
    s.db_path_override = Some(root.join("settings.db"));
    world.insert_resource(s);
    world.insert_resource(SettingsRegistry::with_builtin());
    world.insert_resource(SettingsCapabilities::default());
    world.init_resource::<Messages<SettingChanged>>();
    world.insert_resource(full_localization());

    let r = call(&mut world, "l10n.reload", json!({})).expect("reload");
    assert_eq!(r["usable"], 2);
    assert_eq!(r["broken"], 1);
    assert_eq!(r["problems"][0]["folder"], "broken");
    assert_eq!(r["problems"][0]["issues"][0]["code"], "pack-json");

    let l = call(&mut world, "l10n.languages", json!({})).expect("languages");
    let codes: Vec<&str> = l["languages"]
        .as_array()
        .expect("list")
        .iter()
        .map(|x| x["code"].as_str().expect("code"))
        .collect();
    assert_eq!(codes, ["en", "de", "ar", "fr"]);
    assert_eq!(l["current"], "en");
    assert_eq!(l["languages"][3]["native_name"], "Français");
    assert_eq!(l["languages"][3]["translated"], 1);
    assert_eq!(l["languages"][0]["current"], true);

    let set = call(&mut world, "l10n.set", json!({"code": "fr"})).expect("set");
    assert_eq!(set["current"], "fr");
    assert_eq!(world.resource::<Localization>().t(CLEAR), "Effacer");
    assert_eq!(
        world.resource::<GameSettings>().raw("interface.language").as_deref(),
        Some("fr"),
        "saved like a settings change"
    );
    let rtl = call(&mut world, "l10n.set", json!({"code": "ar"})).expect_err("rtl refused");
    assert_eq!(rtl.code, opts_cli::protocol::code::UNSUPPORTED);
    let unknown = call(&mut world, "l10n.set", json!({"code": "qq"})).expect_err("unknown");
    assert_eq!(unknown.code, opts_cli::protocol::code::NOT_FOUND);
    let bad = call(&mut world, "l10n.set", json!({"code": 5})).expect_err("param");
    assert_eq!(bad.code, opts_cli::protocol::code::INVALID_PARAMS);

    let rep = call(&mut world, "l10n.report", json!({})).expect("report of the current language");
    assert_eq!(rep["language"]["code"], "fr");
    assert!(rep["missing_keys"].as_array().expect("keys").len() > 100);
    assert!(call(&mut world, "l10n.report", json!({"code": "nope"})).is_err());

    // The pack of the active language disappears: reload falls back to English.
    std::fs::remove_dir_all(lang.join("fr")).expect("remove");
    let r = call(&mut world, "l10n.reload", json!({})).expect("reload");
    assert_eq!(r["current"], "en");
    let _ = std::fs::remove_dir_all(&root);
}
