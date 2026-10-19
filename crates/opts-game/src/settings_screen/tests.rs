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

mod changes;

mod ui_rules;

mod content;
use content::outcome_text;

mod sources;

mod controls;

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
