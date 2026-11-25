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
    crate::settings::flush_background_saves();
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

mod lighting_quality_tests;
