//! Tests of the master-detail pattern in Bevy's real UI layout.

use super::*;
use crate::actions::{Direction, ScrollStep, UiAction};
use crate::interaction::focus::FocusEvent;
use crate::l10n::{wizard_keys::md, Localization};
use crate::testing::{act, click, layout_app};
use bevy::input::keyboard::{Key, KeyboardInput};
use bevy::input::ButtonState;

fn entries(n: usize) -> Vec<ListEntry> {
    (0..n)
        .map(|i| ListEntry::new(format!("id{i}"), format!("Entry {i:05}")).with_glyph(glyphs::MAP))
        .collect()
}

fn config() -> MasterDetailConfig {
    MasterDetailConfig {
        prefix: "t",
        empty_key: md::NO_RESULTS,
        search_key: md::SEARCH,
    }
}

fn spawn(app: &mut App, model: MasterDetail) -> MasterDetailParts {
    let loc = app.world().resource::<Localization>().clone();
    let parts = {
        let mut commands = app.world_mut().commands();
        let parts = spawn_master_detail(&mut commands, &loc, config(), model);
        let frame = commands
            .spawn(Node {
                width: Val::Px(1000.0),
                height: Val::Px(560.0),
                ..default()
            })
            .id();
        commands.entity(frame).add_child(parts.root);
        parts
    };
    for _ in 0..6 {
        app.update();
    }
    parts
}

fn view<'a>(app: &'a App, parts: &MasterDetailParts) -> &'a MasterDetailView {
    app.world()
        .get::<MasterDetailView>(parts.root)
        .expect("view")
}

fn model<'a>(app: &'a App, parts: &MasterDetailParts) -> &'a MasterDetail {
    app.world().get::<MasterDetail>(parts.root).expect("model")
}

fn focus_list(app: &mut App, parts: &MasterDetailParts) {
    app.world_mut().write_message(FocusEvent::Set(parts.list));
    app.update();
    app.update();
}

mod filter;
mod scrolling;
mod selection;
