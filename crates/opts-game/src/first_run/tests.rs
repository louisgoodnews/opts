//! First-run flow: dialogs per step, the folder answer, the welcome.

use std::sync::mpsc::channel;

use super::*;
use crate::shell::ShellPlugin;
use opts_gui::prelude::*;

fn app() -> App {
    crate::isolation::isolate_user_dirs();
    let mut app = App::new();
    app.add_plugins(MinimalPlugins)
        .add_plugins(bevy::state::app::StatesPlugin)
        .add_plugins(bevy::input::InputPlugin)
        .add_plugins(AssetPlugin::default())
        .init_asset::<Image>()
        .insert_resource(GameSettings::default())
        .insert_resource(MaterialTheme::dark())
        .insert_resource(FirstRun {
            persist: false,
            ..Default::default()
        })
        .init_state::<AppState>()
        .add_plugins(OptsGuiPlugin)
        .add_plugins(ShellPlugin)
        .add_plugins(FirstRunPlugin);
    app.update();
    app
}

fn ids(app: &mut App) -> Vec<String> {
    opts_gui::ui_tree::snapshot(app.world_mut())
        .flatten()
        .iter()
        .map(|n| n.id.clone())
        .collect()
}

fn keys_shown(app: &mut App) -> Vec<&'static str> {
    app.world_mut()
        .query::<&opts_gui::l10n::L10nKey>()
        .iter(app.world())
        .map(|k| k.0)
        .collect()
}

fn run(app: &mut App, frames: usize) {
    for _ in 0..frames {
        app.update();
    }
}

fn fake_install(tag: &str) -> PathBuf {
    let root = std::env::temp_dir().join(format!("opts-fr-{tag}-{}", std::process::id()));
    std::fs::create_dir_all(root.join("maps/M")).expect("mkdir");
    std::fs::create_dir_all(root.join("Vehicles")).expect("mkdir");
    root
}

fn answer(app: &mut App, picked: Option<PathBuf>) {
    let (tx, rx) = channel();
    tx.send(picked).expect("send");
    *app.world()
        .resource::<FolderPicker>()
        .0
        .lock()
        .expect("lock") = Some(rx);
    run(app, 4);
}

#[test]
fn a_wrong_folder_is_explained_inline_and_the_dialog_stays() {
    let mut app = app();
    app.world_mut().resource_mut::<FirstRun>().step = Some(Step::Choose);
    run(&mut app, 3);
    answer(&mut app, Some(std::env::temp_dir()));
    assert!(matches!(
        app.world().resource::<FirstRun>().error,
        Some(ChooseError::NotInstallation(_))
    ));
    assert!(keys_shown(&mut app).contains(&keys::FIRSTRUN_WRONG_FOLDER));
    assert_eq!(app.world().resource::<GameSettings>().omsi_path, None);
    // a valid answer afterwards resolves everything
    let root = fake_install("fix");
    answer(&mut app, Some(root.join("maps/M")));
    assert_eq!(app.world().resource::<FirstRun>().step, None);
    assert_eq!(
        app.world().resource::<GameSettings>().omsi_path,
        Some(root.clone())
    );
    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn a_cancelled_dialog_keeps_the_question_open() {
    let mut app = app();
    app.world_mut().resource_mut::<FirstRun>().step = Some(Step::Choose);
    run(&mut app, 3);
    answer(&mut app, None);
    assert_eq!(app.world().resource::<FirstRun>().step, Some(Step::Choose));
}

#[test]
fn a_wrong_folder_outside_the_flow_becomes_a_banner() {
    let mut app = app();
    answer(&mut app, Some(std::env::temp_dir()));
    assert!(keys_shown(&mut app).contains(&keys::FIRSTRUN_ERROR_TITLE));
}

#[test]
fn the_welcome_offers_both_ways_and_is_only_shown_once() {
    let mut app = app();
    app.world_mut().resource_mut::<FirstRun>().step = Some(Step::Welcome);
    run(&mut app, 3);
    let ids_now = ids(&mut app);
    assert!(ids_now.iter().any(|i| i == "dialog.first-run.pick"));
    assert!(ids_now.iter().any(|i| i == "dialog.first-run.quick"));
    app.world_mut().write_message(DialogChoice {
        tag: DIALOG_TAG,
        choice: view::PICK,
    });
    run(&mut app, 4);
    let fr = app.world().resource::<FirstRun>();
    assert!(fr.welcome_done && fr.step.is_none());
    assert_eq!(
        *app.world().resource::<State<AppState>>().get(),
        AppState::VehicleSelection
    );
}
