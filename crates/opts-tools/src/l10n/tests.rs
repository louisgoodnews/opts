//! Unit tests of `l10n.rs` (moved out of the file to keep it under the 500-line limit, refactoring cycle 1).

use super::*;

fn scratch(name: &str) -> PathBuf {
    let dir = std::env::var_os("HOME")
        .map(PathBuf::from)
        .unwrap_or_default()
        .join(".local/share/opts/benchmark/langpacks/tools-tests")
        .join(name);
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn run(args: &[&str]) -> Output {
    execute(&args.iter().map(|s| (*s).to_string()).collect::<Vec<_>>())
}

#[test]
fn new_check_and_stats_work_together() {
    let root = scratch("new-check");
    let r = root.to_str().unwrap();
    let o = run(&[
        "new",
        "fr",
        "--root",
        r,
        "--name",
        "French",
        "--native-name",
        "Français",
        "--author",
        "Tester",
    ]);
    assert_eq!(o.code, 0, "{}{}", o.out, o.err);
    let pack = root.join("fr");
    let c = run(&["check", pack.to_str().unwrap(), "--json"]);
    let v: serde_json::Value = serde_json::from_str(&c.out).unwrap();
    assert_eq!(c.code, 0, "{}", c.out);
    assert_eq!(v["errors"], 0);
    // One-word texts may legitimately equal English (names, loanwords), so an untouched
    // scaffold still counts them; every sentence is a copy.
    assert!(v["percent"].as_f64().unwrap() < 50.0, "{}", c.out);
    assert!(v["copied"].as_u64().unwrap() > 200, "{}", c.out);
    assert_eq!(v["outdated"], false);
    // Translate one message and break another.
    let settings = std::fs::read_to_string(pack.join("gui.ftl")).unwrap();
    let edited = settings
        .replace("widget-clear = Clear", "widget-clear = Effacer")
        .replace(
            "widget-not-available-yet = Feature not available yet!",
            "widget-not-available-yet = { $oops",
        );
    std::fs::write(pack.join("gui.ftl"), edited).unwrap();
    let c = run(&["check", pack.to_str().unwrap()]);
    assert_eq!(c.code, 1);
    assert!(c.out.contains("gui.ftl:"), "{}", c.out);
    assert!(c.out.contains("error[syntax]"), "{}", c.out);
    let s = run(&["stats", "--root", r, "--json"]);
    let v: serde_json::Value = serde_json::from_str(&s.out).unwrap();
    let langs = v["languages"].as_array().unwrap();
    assert_eq!(langs.len(), 3);
    assert_eq!(langs[2]["code"], "fr");
    assert!(langs[2]["translated"].as_u64().unwrap() >= 1);
    assert_eq!(langs[1]["code"], "de");
    let again = run(&["new", "fr", "--root", r]);
    assert_eq!(again.code, 2, "refuses to overwrite");
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn export_import_round_trip_through_the_command_line() {
    let root = scratch("round-trip");
    let json = root.join("de.json");
    let o = run(&["export", "--lang", "de", "--out", json.to_str().unwrap()]);
    assert_eq!(o.code, 0, "{}", o.err);
    // The pack object of a built-in language is complete, so import works as is.
    let packs = root.join("packs");
    let i = run(&[
        "import",
        json.to_str().unwrap(),
        "--root",
        packs.to_str().unwrap(),
    ]);
    assert_eq!(i.code, 0, "{}", i.out);
    let c = run(&[
        "check",
        packs.join("de").to_str().unwrap(),
        "--json",
        "--min-percent",
        "98",
    ]);
    assert_eq!(c.code, 0, "{}", c.out);
    let v: serde_json::Value = serde_json::from_str(&c.out).unwrap();
    assert_eq!(v["missing"], 0);
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn a_filled_json_template_is_checked_without_a_pack_folder() {
    let root = scratch("template");
    let o = run(&["export", "--lang", "fr", "--missing"]);
    assert_eq!(o.code, 0);
    let v: serde_json::Value = serde_json::from_str(&o.out).unwrap();
    assert_eq!(v["format"], "opts-l10n/1");
    let mut v = v;
    v["messages"]["widget-clear"]["text"] = "Effacer".into();
    v["messages"]["widget-items"]["forms"]["one"] = "{ $count } élément".into();
    v["messages"]["widget-items"]["forms"]["many"] = "{ $count } éléments".into();
    v["messages"]["widget-items"]["forms"]["other"] = "{ $count } éléments".into();
    v["messages"]["menu-shutdown"]["text"] = "Quitter { $unknown }".into();
    let f = root.join("fr.json");
    std::fs::write(&f, serde_json::to_string_pretty(&v).unwrap()).unwrap();
    let c = run(&["check", f.to_str().unwrap(), "--json"]);
    assert_eq!(c.code, 1);
    let r: serde_json::Value = serde_json::from_str(&c.out).unwrap();
    assert_eq!(r["errors"], 1, "{}", c.out);
    assert_eq!(r["issues"][0]["code"], "variable-mismatch");
    assert_eq!(r["issues"][0]["key"], "menu-shutdown");
    assert!(r["issues"][0]["line"].as_u64().unwrap() > 5);
    assert_eq!(r["translated"], 2);
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn usage_errors_exit_with_two() {
    assert_eq!(run(&[]).code, 2);
    assert_eq!(run(&["frobnicate"]).code, 2);
    assert_eq!(run(&["export"]).code, 2);
    assert_eq!(run(&["check", "/no/such/place"]).code, 2);
    assert_eq!(run(&["new", "BAD CODE"]).code, 2);
    assert_eq!(run(&["new", "de"]).code, 2);
}
