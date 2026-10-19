//! The content page: folder buttons and the scan chip.

use super::*;

fn content_app(name: &str) -> (App, std::path::PathBuf) {
    let (mut app, _, db) = page_app(name, "content");
    app.insert_resource(crate::first_run::FolderOpener(Arc::new(|| None)))
        .init_resource::<crate::first_run::FolderPicker>()
        .add_message::<crate::first_run::OmsiFolderChosen>()
        .add_systems(
            Update,
            (
                super::super::content::run_actions,
                super::super::content::sync_chosen_folder,
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

pub(super) fn outcome_text(o: opts_bevy::control::state::Outcome) -> String {
    match o {
        opts_bevy::control::state::Outcome::Done(Ok(v)) => v.to_string(),
        _ => String::new(),
    }
}

/// Facts that take seconds, like the full catalogue on a slow disk.
pub(super) struct SlowFacts(Duration);

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
    let keys = |app: &mut App| -> Vec<(super::super::info::ContentLine, &'static str, String)> {
        let mut q = app.world_mut().query::<(
            &super::super::info::ContentLine,
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
            .query_filtered::<&opts_gui::l10n::L10nKey, With<super::super::scan_status::ScanChipLabel>>();
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
