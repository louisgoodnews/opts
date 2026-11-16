//! Integration tests for the language pack control methods: list, set, reload and report.
use super::*;

fn call(
    world: &mut World,
    method: &str,
    params: Value,
) -> Result<Value, opts_cli::protocol::RpcError> {
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
    // The packs folder of this test (`XDG_DATA_HOME` would only work on Linux); this is the only
    // test that sets it.
    let lang = root.join("opts/lang");
    std::env::set_var("OPTS_LANG_DIR", &lang);
    write(&lang, "fr/pack.json", &meta("fr", "French", "Français", ""));
    write(&lang, "fr/s.ftl", "widget-clear = Effacer\n");
    write(
        &lang,
        "ar/pack.json",
        &meta("ar", "Arabic", "العربية", ",\"direction\":\"rtl\""),
    );
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
        world
            .resource::<GameSettings>()
            .raw("interface.language")
            .as_deref(),
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
