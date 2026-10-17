//! Tests of the settings page: rows, changes, persistence, search, the revert prompt (with a
//! fake display) and the CI rules of the UI.

use std::sync::{Arc, Mutex};
use std::time::Duration;

use bevy::prelude::*;
use bevy::time::TimeUpdateStrategy;
use opts_gui::interaction::state::Disabled;
use opts_gui::prelude::*;
use opts_gui::testing::{click, layout_app, rules};

use super::page::{spawn_settings_page, SettingsPagePlugin, SettingsPageState};
use crate::content_facts::{
    ContentFactsStore, FactsProvider, FactsRequest, HeavyFacts, LightFacts, RootState,
};
use crate::settings::GameSettings;
use crate::settings_registry::display::{DisplayApply, DisplayBackend};
use crate::settings_registry::{
    Apply, Available, ChangeSource, SettingKind, SettingsRegistry, SettingsRegistryPlugin,
};

type Log = Arc<Mutex<Vec<(String, String)>>>;

struct FakeDisplay(Log);

impl DisplayApply for FakeDisplay {
    fn apply(&mut self, _: &mut World, key: &str, value: &str) {
        if let Ok(mut l) = self.0.lock() {
            l.push((key.to_string(), value.to_string()));
        }
    }
}

fn temp_db(name: &str) -> std::path::PathBuf {
    let dir =
        std::env::temp_dir().join(format!("opts-settings-test-{}-{name}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    let _ = std::fs::create_dir_all(&dir);
    dir.join("settings.db")
}

/// Facts that are known at once and say nothing (no test reads the user's real catalogue).
struct NoFacts;

impl FactsProvider for NoFacts {
    fn light(&self, _: &FactsRequest) -> LightFacts {
        LightFacts::default()
    }
    fn heavy(&self, _: &FactsRequest) -> HeavyFacts {
        HeavyFacts::default()
    }
}

/// A layout app with the page of `category` and a fake display; the database is a temp file.
fn page_app(name: &str, category: &str) -> (App, Log, std::path::PathBuf) {
    page_app_with(name, category, Arc::new(NoFacts))
}

fn page_app_with(
    name: &str,
    category: &str,
    facts: Arc<dyn FactsProvider>,
) -> (App, Log, std::path::PathBuf) {
    let db = temp_db(name);
    let mut app = layout_app();
    app.insert_resource(ContentFactsStore::new(facts));
    let mut settings = GameSettings::default();
    settings.db_path_override = Some(db.clone());
    app.insert_resource(settings)
        .insert_resource(TimeUpdateStrategy::ManualDuration(Duration::from_secs(1)))
        .add_plugins((SettingsRegistryPlugin, SettingsPagePlugin));
    let log: Log = Arc::default();
    app.insert_resource(DisplayBackend(Box::new(FakeDisplay(log.clone()))));
    let root = app.world_mut().spawn(Node::default()).id();
    {
        let mut commands = app.world_mut().commands();
        spawn_settings_page(&mut commands, root, Some(category));
    }
    for _ in 0..4 {
        app.update();
    }
    (app, log, db)
}

fn by_id(app: &mut App, id: &str) -> Option<Entity> {
    app.world_mut()
        .query::<(Entity, &UiId)>()
        .iter(app.world())
        .find(|(_, u)| u.0 == id)
        .map(|(e, _)| e)
}

fn stored(db: &std::path::Path, key: &str) -> Option<String> {
    // Saves go through the background writer: wait for it before reading the file.
    crate::settings::GameSettings::flush_background_saves();
    let conn = rusqlite::Connection::open(db).ok()?;
    conn.query_row(
        "SELECT value FROM app_settings WHERE key = ?1",
        [key],
        |r| r.get(0),
    )
    .ok()
}

fn text(app: &App, key: &str) -> String {
    let r = app.world().resource::<SettingsRegistry>();
    r.text(app.world().resource::<GameSettings>(), key)
        .unwrap_or_default()
}

#[test]
fn choosing_an_option_changes_and_saves_the_setting() {
    let (mut app, _, db) = page_app("choose", "graphics");
    let b = by_id(&mut app, "settings/graphics/texture_filtering/option-8").expect("option");
    click(&mut app, b);
    app.update();
    app.update();
    assert_eq!(text(&app, "texture_filtering"), "8");
    assert_eq!(
        app.world().resource::<GameSettings>().texture_filtering,
        opts_bevy::TextureFiltering::X8
    );
    assert_eq!(stored(&db, "texture_filtering").as_deref(), Some("8"));
}

#[test]
fn reset_restores_the_default_and_is_disabled_at_the_default() {
    let (mut app, _, db) = page_app("reset", "graphics");
    let reset = by_id(&mut app, "settings/graphics/look/reset").expect("reset");
    assert!(
        app.world().get::<Disabled>(reset).is_some(),
        "at default: nothing to reset"
    );
    let b = by_id(&mut app, "settings/graphics/look/option-natural").expect("option");
    click(&mut app, b);
    app.update();
    app.update();
    assert_eq!(text(&app, "look"), "natural");
    let reset = by_id(&mut app, "settings/graphics/look/reset").expect("reset");
    assert!(app.world().get::<Disabled>(reset).is_none());
    click(&mut app, reset);
    app.update();
    app.update();
    assert_eq!(text(&app, "look"), "vivid");
    assert_eq!(stored(&db, "look").as_deref(), Some("vivid"));
}

#[test]
fn search_lists_matches_across_categories() {
    let (mut app, _, _) = page_app("search", "graphics");
    app.world_mut().resource_mut::<SettingsPageState>().query = "language".into();
    app.world_mut().resource_mut::<SettingsPageState>().dirty = true;
    app.update();
    app.update();
    assert!(by_id(&mut app, "settings/interface/interface.language/option-de").is_some());
    assert!(by_id(&mut app, "settings/graphics/look/option-vivid").is_none());
    app.world_mut().resource_mut::<SettingsPageState>().query = "zzzzqq".into();
    app.world_mut().resource_mut::<SettingsPageState>().dirty = true;
    app.update();
    app.update();
    assert!(by_id(&mut app, "settings/interface/interface.language/option-de").is_none());
}

#[test]
fn planned_rows_are_visible_disabled_and_marked() {
    let (mut app, _, _) = page_app("notyet", "audio");
    let b = by_id(&mut app, "settings/audio/audio.master/down").expect("stepper stays visible");
    assert!(app.world().get::<Disabled>(b).is_some());
    let labels = app
        .world_mut()
        .query_filtered::<(), With<opts_gui::not_yet::NotYetLabel>>()
        .iter(app.world())
        .count();
    assert_eq!(labels, 4, "one red label per planned audio row");
}

fn advance(app: &mut App, secs: u32) {
    for _ in 0..secs {
        app.update();
    }
}

#[test]
fn a_risky_change_reverts_when_nobody_confirms() {
    let (mut app, log, db) = page_app("revert", "graphics");
    let b = by_id(
        &mut app,
        "settings/graphics/graphics.window_mode/option-borderless",
    )
    .expect("option");
    click(&mut app, b);
    app.update();
    assert_eq!(
        text(&app, "graphics.window_mode"),
        "borderless",
        "applied at once"
    );
    assert!(
        by_id(&mut app, "settings/revert-prompt/keep").is_some(),
        "prompt is open"
    );
    assert_eq!(
        stored(&db, "graphics.window_mode"),
        None,
        "not saved before it is kept"
    );
    advance(&mut app, 16);
    assert_eq!(
        text(&app, "graphics.window_mode"),
        "windowed",
        "15 s without an answer: back"
    );
    assert!(
        by_id(&mut app, "settings/revert-prompt/keep").is_none(),
        "prompt closed"
    );
    let log = log.lock().expect("log").clone();
    assert_eq!(
        log,
        vec![
            ("graphics.window_mode".to_string(), "borderless".to_string()),
            ("graphics.window_mode".to_string(), "windowed".to_string()),
        ]
    );
    assert_eq!(stored(&db, "graphics.window_mode"), None);
}

#[test]
fn keeping_a_risky_change_saves_it() {
    let (mut app, _, db) = page_app("keep", "graphics");
    let b = by_id(
        &mut app,
        "settings/graphics/graphics.resolution/option-1280x720",
    )
    .expect("option");
    click(&mut app, b);
    app.update();
    let keep = by_id(&mut app, "settings/revert-prompt/keep").expect("prompt");
    click(&mut app, keep);
    app.update();
    app.update();
    assert_eq!(text(&app, "graphics.resolution"), "1280x720");
    assert_eq!(
        stored(&db, "graphics.resolution").as_deref(),
        Some("1280x720")
    );
    advance(&mut app, 20);
    assert_eq!(
        text(&app, "graphics.resolution"),
        "1280x720",
        "no late revert"
    );
}

#[test]
fn going_back_returns_the_old_value() {
    let (mut app, _, _) = page_app("goback", "graphics");
    let b = by_id(
        &mut app,
        "settings/graphics/graphics.resolution/option-1600x900",
    )
    .expect("option");
    click(&mut app, b);
    app.update();
    let back = by_id(&mut app, "settings/revert-prompt/revert").expect("prompt");
    click(&mut app, back);
    app.update();
    app.update();
    assert_eq!(text(&app, "graphics.resolution"), "1920x1080");
}

/// Every usable setting can be changed on the page's own route and shows its new value.
#[test]
fn every_visible_setting_has_a_working_control() {
    let all: Vec<_> = SettingsRegistry::with_builtin()
        .all()
        .into_iter()
        .cloned()
        .collect();
    for d in all
        .iter()
        .filter(|d| !d.hidden && matches!(d.available, Available::Always))
    {
        let (mut app, _, _) = page_app(
            &format!("walk-{}", d.key.replace('.', "_")),
            d.category.id(),
        );
        let ids = app
            .world_mut()
            .query::<&UiId>()
            .iter(app.world())
            .filter(|u| {
                u.0.starts_with(&format!("settings/{}/{}/", d.category.id(), d.key))
            })
            .count();
        assert!(ids > 0, "{} has no control on the page", d.key);
        let alt = match d.kind {
            SettingKind::Bool => (if d.default == "true" { "false" } else { "true" }).to_string(),
            SettingKind::Enum(o) => o
                .iter()
                .find(|o| o.value != d.default)
                .map(|o| o.value.to_string())
                .unwrap_or_default(),
            SettingKind::Int { min, max, .. } => (if d.default.parse::<i64>().unwrap_or(0) == min {
                max
            } else {
                min
            })
            .to_string(),
            SettingKind::Time => "05:30".into(),
            SettingKind::Text if d.key == "interface.language" => "fr".into(),
            SettingKind::Path | SettingKind::Text => "x".into(),
            SettingKind::Float { .. } | SettingKind::Binding | SettingKind::Action => continue,
        };
        let r = crate::settings_registry::apply_change(
            app.world_mut(),
            d.key,
            Some(&alt),
            ChangeSource::Protocol,
        );
        assert!(r.is_ok(), "{}: {r:?}", d.key);
        app.update();
        app.update();
        assert_eq!(text(&app, d.key), alt, "{} did not take {alt}", d.key);
        if !matches!(
            d.apply,
            Apply::Live | Apply::OnRestart | Apply::WithRevert(_)
        ) {
            unreachable!();
        }
    }
}

#[test]
fn every_category_obeys_the_ui_rules() {
    for cat in crate::settings_registry::Category::ALL {
        let (mut app, _, _) = page_app(&format!("rules-{}", cat.id()), cat.id());
        let id = cat.id();
        assert!(
            rules::small_targets(app.world_mut()).is_empty(),
            "{id}: {:?}",
            rules::small_targets(app.world_mut())
        );
        assert!(rules::small_text(app.world_mut()).is_empty(), "{id}");
        assert!(
            rules::disabled_without_reason(app.world_mut()).is_empty(),
            "{id}: {:?}",
            rules::disabled_without_reason(app.world_mut())
        );
        let t = rules::keyboard_traversal(&mut app);
        assert!(t.missed.is_empty(), "{id} unreached: {:?}", t.missed);
        assert!(
            t.inert.is_empty(),
            "{id} inert: {:?} activated: {:?} reached: {:?}",
            t.inert,
            t.activated,
            t.reached
        );
    }
}

fn content_app(name: &str) -> (App, std::path::PathBuf) {
    let (mut app, _, db) = page_app(name, "content");
    app.insert_resource(crate::first_run::FolderOpener(Arc::new(|| None)))
        .init_resource::<crate::first_run::FolderPicker>()
        .add_message::<crate::first_run::OmsiFolderChosen>()
        .add_systems(
            Update,
            (
                super::content::run_actions,
                super::content::sync_chosen_folder,
            ),
        );
    app.update();
    (app, db)
}

#[test]
fn the_omsi_folder_button_starts_the_shells_folder_flow() {
    let (mut app, _) = content_app("omsi-flow");
    assert!(!app
        .world()
        .resource::<crate::first_run::FolderPicker>()
        .is_open());
    let b = by_id(&mut app, "settings/content/omsi_path/choose").expect("choose button");
    click(&mut app, b);
    for _ in 0..3 {
        app.update();
    }
    assert!(
        app.world()
            .resource::<crate::first_run::FolderPicker>()
            .is_open(),
        "first_run::choose_omsi_folder ran"
    );
}

#[test]
fn a_folder_the_shell_flow_adopted_reaches_the_registry() {
    let root = std::env::temp_dir().join(format!("opts-settings-omsi-{}", std::process::id()));
    let (mut app, db) = content_app("omsi-ok");
    app.world_mut()
        .write_message(crate::first_run::OmsiFolderChosen(Ok(root.clone())));
    for _ in 0..3 {
        app.update();
    }
    assert_eq!(stored(&db, "omsi_path").as_deref(), root.to_str());
}

#[test]
fn a_refused_folder_changes_nothing() {
    let (mut app, db) = content_app("omsi-bad");
    app.world_mut()
        .write_message(crate::first_run::OmsiFolderChosen(Err(
            crate::first_run::ChooseError::Cancelled,
        )));
    for _ in 0..3 {
        app.update();
    }
    assert!(stored(&db, "omsi_path").is_none_or(|v| v.is_empty()));
}

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
    let en = opts_gui::l10n::fluent::parse(include_str!("../settings_registry/messages_en.ftl"));
    let de = opts_gui::l10n::fluent::parse(include_str!("../settings_registry/messages_de.ftl"));
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

/// Keyboard: focusing any control of a category and pressing Confirm uses it (the page's own
/// traversal; the library helper would also press the category buttons, which changes the page).
#[test]
fn every_control_of_every_category_can_be_used_with_the_keyboard() {
    use opts_gui::actions::UiAction;
    use opts_gui::interaction::activation::Activated;
    use opts_gui::interaction::focus::FocusEvent;
    for cat in crate::settings_registry::Category::ALL {
        let (mut app, _, _) = page_app(&format!("kbd-{}", cat.id()), cat.id());
        let prefix = format!("settings/{}/", cat.id());
        let ids: Vec<(Entity, String)> = app
            .world_mut()
            .query::<(Entity, &UiId, &opts_gui::interaction::focus::Focusable)>()
            .iter(app.world())
            .filter(|(e, u, _)| {
                u.0.starts_with(&prefix) && app.world().get::<Disabled>(*e).is_none()
            })
            .map(|(e, u, _)| (e, u.0.to_string()))
            .collect();
        if matches!(
            cat,
            crate::settings_registry::Category::Audio
                | crate::settings_registry::Category::Controls
        ) {
            assert!(
                ids.is_empty(),
                "{}: planned, every control is disabled",
                cat.id()
            );
            continue;
        }
        assert!(!ids.is_empty(), "{}: no controls", cat.id());
        for (_, id) in ids {
            // A language change rebuilds the page: look the control up again by its id.
            let e = by_id(&mut app, &id).expect("control exists");
            app.world_mut().write_message(FocusEvent::Set(e));
            app.update();
            app.update();
            let before = opts_gui::testing::focused(&mut app);
            let mut seen = false;
            app.world_mut().write_message(UiAction::Confirm);
            for _ in 0..2 {
                app.update();
                let msgs = app.world().resource::<Messages<Activated>>();
                let mut cur = msgs.get_cursor();
                seen |= cur.read(msgs).any(|a| a.0 == e);
            }
            let f = opts_gui::testing::focused(&mut app)
                .and_then(|f| app.world().get::<UiId>(f).map(|u| u.0.to_string()));
            assert!(seen, "{id}: Confirm did not activate it (focus before confirm {before:?}, now on {f:?}, target {e:?}, locale {})", app.world().resource::<opts_gui::l10n::Localization>().locale());
            // Rebuilds replace entities; settle before the next control.
            for _ in 0..10 {
                app.update();
            }
        }
    }
}

/// The acceptance criterion of M-0063: one descriptor, and the setting is in the UI, in the saved
/// file, in the control protocol and in the generated reference page, with no other edit.
#[test]
fn one_descriptor_reaches_ui_persistence_protocol_and_reference() {
    use crate::settings_registry::{Category, SettingDescriptor, SettingsAppExt};
    const FIXTURE: SettingDescriptor = SettingDescriptor::new(
        "fixture.wheel_sensitivity",
        SettingKind::Int {
            min: 1,
            max: 10,
            step: 1,
        },
        "5",
        Category::Controls,
        5,
        "fixture-label",
        "fixture-desc",
    );
    let (mut app, _, db) = page_app("acceptance", "controls");
    app.register_setting(FIXTURE);
    {
        let mut loc = app
            .world_mut()
            .resource_mut::<opts_gui::l10n::Localization>();
        loc.extend(
            "en",
            "fixture-label = Wheel sensitivity\nfixture-desc = How strongly the wheel steers.\n",
        );
        loc.extend(
            "de",
            "fixture-label = Lenkradempfindlichkeit\nfixture-desc = Wie stark das Lenkrad lenkt.\n",
        );
    }
    app.world_mut().resource_mut::<SettingsPageState>().dirty = true;
    for _ in 0..3 {
        app.update();
    }
    // UI: the row exists and its control works.
    let up =
        by_id(&mut app, "settings/controls/fixture.wheel_sensitivity/up").expect("row in the UI");
    click(&mut app, up);
    app.update();
    app.update();
    assert_eq!(text(&app, "fixture.wheel_sensitivity"), "6");
    // Persistence.
    assert_eq!(
        stored(&db, "fixture.wheel_sensitivity").as_deref(),
        Some("6")
    );
    // Protocol.
    let listed = outcome_text(crate::settings_registry::protocol::list(
        app.world_mut(),
        &serde_json::json!({ "category": "controls" }),
    ));
    assert!(listed.contains("fixture.wheel_sensitivity"), "{listed}");
    // Generated reference page.
    let md = crate::settings_registry::reference::markdown_with(
        app.world().resource::<SettingsRegistry>(),
        app.world().resource::<opts_gui::l10n::Localization>(),
    );
    assert!(
        md.contains("`fixture.wheel_sensitivity`") && md.contains("Wheel sensitivity"),
        "{}",
        md.lines()
            .filter(|l| l.contains("fixture"))
            .collect::<Vec<_>>()
            .join("\n")
    );
    // Search.
    let loc = app
        .world()
        .resource::<opts_gui::l10n::Localization>()
        .clone();
    let found = crate::settings_registry::search::search(
        app.world().resource::<SettingsRegistry>(),
        &loc,
        "wheel",
    );
    assert!(found.iter().any(|d| d.key == "fixture.wheel_sensitivity"));
}

fn outcome_text(o: opts_bevy::control::state::Outcome) -> String {
    match o {
        opts_bevy::control::state::Outcome::Done(Ok(v)) => v.to_string(),
        _ => String::new(),
    }
}

/// Facts that take seconds, like the full catalogue on a slow disk.
struct SlowFacts(Duration);

impl FactsProvider for SlowFacts {
    fn light(&self, _: &FactsRequest) -> LightFacts {
        std::thread::sleep(self.0);
        LightFacts {
            last_scan: Some(1_700_000_000),
            entities: 75_313,
            root: RootState::Present,
            folder_changed: false,
        }
    }
    fn heavy(&self, _: &FactsRequest) -> HeavyFacts {
        std::thread::sleep(self.0);
        HeavyFacts {
            kinds: opts_catalogue::KindCounts {
                files: 679_543,
                entities: 75_313,
                maps: 43,
                vehicles: 947,
                scenery_objects: 20_000,
                splines: 1_500,
                other: 52_823,
            },
        }
    }
}

/// M-0088: whatever the facts cost, the page builds within a frame budget, shows "Calculating..."
/// and fills in when the answer arrives. The provider takes 3 seconds in all; every frame must
/// stay far below that (generous limit: CI machines are busy; the real budget is 33 ms).
#[test]
fn the_content_page_builds_within_a_frame_and_fills_in_when_the_facts_arrive() {
    const FRAME_LIMIT: Duration = Duration::from_millis(250);
    let started = std::time::Instant::now();
    let (mut app, _, _) = page_app_with(
        "content-freeze",
        "content",
        Arc::new(SlowFacts(Duration::from_millis(1500))),
    );
    let keys = |app: &mut App| -> Vec<(super::info::ContentLine, &'static str, String)> {
        let mut q = app.world_mut().query::<(
            &super::info::ContentLine,
            &opts_gui::l10n::L10nKey,
            &opts_gui::l10n::L10nArgs,
        )>();
        q.iter(app.world())
            .map(|(l, k, a)| (*l, k.0, format!("{:?}", a.0)))
            .collect()
    };
    assert!(
        started.elapsed() < Duration::from_millis(1500),
        "building the page waited for the facts"
    );
    let first = keys(&mut app);
    assert_eq!(first.len(), 3, "the three lines exist at once: {first:?}");
    assert!(
        first.iter().all(|(_, k, _)| k.ends_with("-pending")),
        "placeholders first: {first:?}"
    );
    let mut worst = Duration::ZERO;
    let deadline = std::time::Instant::now() + Duration::from_secs(20);
    loop {
        let t = std::time::Instant::now();
        app.update();
        worst = worst.max(t.elapsed());
        let now = keys(&mut app);
        if now.iter().all(|(_, k, _)| !k.ends_with("-pending")) {
            let all: String = now.iter().map(|(_, k, a)| format!("{k} {a}")).collect();
            for n in ["679543", "75313", "947", "20000"] {
                assert!(all.contains(n), "{n} missing in {all}");
            }
            break;
        }
        assert!(
            std::time::Instant::now() < deadline,
            "the facts never arrived: {now:?}"
        );
        std::thread::sleep(Duration::from_millis(20));
    }
    assert!(
        worst < FRAME_LIMIT,
        "a frame took {worst:?} while the facts were gathered (limit {FRAME_LIMIT:?})"
    );
}

/// The scan chip never claims a verdict before the facts are there.
#[test]
fn the_scan_chip_says_it_is_checking_until_the_facts_arrive() {
    let (mut app, _, _) = page_app_with(
        "content-chip",
        "content",
        Arc::new(SlowFacts(Duration::from_millis(600))),
    );
    let label = |app: &mut App| {
        let mut q = app
            .world_mut()
            .query_filtered::<&opts_gui::l10n::L10nKey, With<super::scan_status::ScanChipLabel>>();
        q.iter(app.world()).next().map(|k| k.0)
    };
    assert_eq!(label(&mut app), Some("scan-chip-pending"));
    let deadline = std::time::Instant::now() + Duration::from_secs(20);
    while label(&mut app) == Some("scan-chip-pending") {
        assert!(std::time::Instant::now() < deadline, "no verdict");
        app.update();
        std::thread::sleep(Duration::from_millis(20));
    }
    assert_eq!(label(&mut app), Some("scan-chip"));
}

mod lighting_quality_section {
    use super::*;
    use opts_bevy::light_quality::{ladder_preset, GovernorState, LightMode, LightQuality};

    fn quality(app: &App) -> LightQuality {
        *app.world().resource::<LightQuality>()
    }

    #[test]
    fn automatic_shows_the_governors_values_disabled_and_custom_unlocks_them() {
        let (mut app, _, _) = page_app("lightquality", "graphics");
        assert!(by_id(
            &mut app,
            "settings/graphics/graphics.light_quality_mode/option-custom"
        )
        .is_some());
        let cap_up = "settings/graphics/graphics.light_cone_cap/up";
        let up = by_id(&mut app, cap_up).expect("cone cap stepper");
        assert!(
            app.world().get::<Disabled>(up).is_some(),
            "automatic: the custom controls are disabled"
        );
        let bloom = by_id(&mut app, "settings/graphics/graphics.bloom/switch").expect("bloom");
        assert!(app.world().get::<Disabled>(bloom).is_some());
        assert!(
            by_id(&mut app, "settings/graphics/graphics.bloom/reset").is_none(),
            "nothing to reset while the governor decides"
        );

        // The governor moves to level 3: the page shows its values (cone cap 0) without
        // touching the stored custom values.
        app.world_mut().resource_mut::<GovernorState>().level = 3;
        for _ in 0..4 {
            app.update();
        }
        assert_eq!(quality(&app), ladder_preset(3));
        let shown = app
            .world_mut()
            .query::<(&super::super::rows::SettingValueText, &Text)>()
            .iter(app.world())
            .find(|(k, _)| k.0 == "graphics.light_cone_cap")
            .map(|(_, t)| t.0.clone());
        assert_eq!(shown.as_deref(), Some("0"));
        assert_eq!(
            text(&app, "graphics.light_cone_cap"),
            "64",
            "stored value untouched"
        );

        // Custom: the level reached becomes the starting values and the controls work.
        let custom = by_id(
            &mut app,
            "settings/graphics/graphics.light_quality_mode/option-custom",
        )
        .expect("custom option");
        click(&mut app, custom);
        for _ in 0..4 {
            app.update();
        }
        assert_eq!(text(&app, "graphics.light_quality_mode"), "custom");
        assert_eq!(text(&app, "graphics.light_cone_cap"), "0");
        assert_eq!(text(&app, "graphics.light_headlight_cones"), "false");
        let up = by_id(&mut app, cap_up).expect("cone cap stepper");
        assert!(app.world().get::<Disabled>(up).is_none(), "custom: enabled");
        click(&mut app, up);
        app.update();
        app.update();
        assert_eq!(text(&app, "graphics.light_cone_cap"), "8");
        assert_eq!(quality(&app).cone_cap, 8);
        assert_eq!(quality(&app).mode, LightMode::Custom);
        // The governor's target frame rate is the other way round: greyed out in custom.
        let fps_up = by_id(
            &mut app,
            "settings/graphics/graphics.light_governor_target_fps/up",
        )
        .expect("target fps stepper");
        assert!(
            app.world().get::<Disabled>(fps_up).is_some(),
            "custom: the governor target is disabled"
        );
    }

    #[test]
    fn the_governor_target_is_enabled_in_automatic_mode() {
        let (mut app, _, _) = page_app("lightgovfps", "graphics");
        let up = by_id(
            &mut app,
            "settings/graphics/graphics.light_governor_target_fps/up",
        )
        .expect("target fps stepper");
        assert!(app.world().get::<Disabled>(up).is_none());
        click(&mut app, up);
        app.update();
        app.update();
        assert_eq!(text(&app, "graphics.light_governor_target_fps"), "70");
        assert_eq!(
            app.world()
                .resource::<opts_bevy::light_quality::GovernorTargetFps>()
                .0,
            70
        );
    }

    #[test]
    fn the_control_protocol_sets_the_mode() {
        let (mut app, _, _) = page_app("lightproto", "graphics");
        let out = crate::settings_registry::protocol::set(
            app.world_mut(),
            &serde_json::json!({ "key": "graphics.light_quality_mode", "value": "custom" }),
        );
        assert!(
            outcome_text(out).contains("custom"),
            "settings.set must accept the mode"
        );
        app.update();
        app.update();
        assert_eq!(quality(&app).mode, LightMode::Custom);
        assert_eq!(text(&app, "graphics.light_quality_mode"), "custom");
    }
}
