//! The content page: folder buttons and the scan chip.

mod facts;
mod flow;

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
