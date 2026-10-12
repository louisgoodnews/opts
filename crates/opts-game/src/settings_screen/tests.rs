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

/// A layout app with the page of `category` and a fake display; the database is a temp file.
fn page_app(name: &str, category: &str) -> (App, Log, std::path::PathBuf) {
    let db = temp_db(name);
    let mut app = layout_app();
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
    app.update();
    app.update();
    assert!(by_id(&mut app, "settings/interface/interface.language/option-de").is_some());
    assert!(by_id(&mut app, "settings/graphics/look/option-vivid").is_none());
    app.world_mut().resource_mut::<SettingsPageState>().query = "zzzzqq".into();
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
fn the_page_obeys_the_ui_rules() {
    let (mut app, _, _) = page_app("rules", "interface");
    assert!(
        rules::small_targets(app.world_mut()).is_empty(),
        "{:?}",
        rules::small_targets(app.world_mut())
    );
    assert!(rules::small_text(app.world_mut()).is_empty());
    assert!(
        rules::disabled_without_reason(app.world_mut()).is_empty(),
        "{:?}",
        rules::disabled_without_reason(app.world_mut())
    );
    let t = rules::keyboard_traversal(&mut app);
    assert!(t.missed.is_empty(), "unreached: {:?}", t.missed);
    assert!(t.inert.is_empty(), "inert: {:?}", t.inert);
}
