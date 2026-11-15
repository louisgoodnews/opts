//! The Help page's tabs and its panes: the tab clicks, the selection marks, the pane of the open
//! tab and the report button.

use bevy::prelude::*;
use opts_bevy::reporting::{ReportModalData, ReportModalState, request_report};
use opts_gui::interaction::activation::Activated;
use opts_gui::interaction::state::Selected;
use opts_gui::l10n::L10nKey;
use opts_gui::prelude::*;

use crate::shell::keys;

use super::{HelpPane, HelpState, HelpTab, ReportButton, about, controls};

pub(super) fn tab_clicks(
    mut activated: MessageReader<Activated>,
    tabs: Query<&HelpTab>,
    mut state: ResMut<HelpState>,
) {
    for Activated(entity) in activated.read() {
        if let Ok(tab) = tabs.get(*entity) {
            state.tab = *tab;
        }
    }
}

pub(super) fn sync_tabs(
    state: Res<HelpState>,
    mut commands: Commands,
    tabs: Query<(Entity, &HelpTab, Has<Selected>)>,
) {
    for (entity, tab, selected) in &tabs {
        let want = *tab == state.tab;
        if want == selected && !state.is_changed() {
            continue;
        }
        let mut e = commands.entity(entity);
        if want {
            e.try_insert((
                Selected,
                BackgroundRole(ColorRole::SecondaryContainer),
                StateLayerRole(ColorRole::OnSecondaryContainer),
            ));
        } else {
            e.try_remove::<(Selected, BackgroundRole)>();
            e.try_insert((
                BackgroundColor(Color::NONE),
                StateLayerRole(ColorRole::OnSurface),
            ));
        }
    }
}

pub(super) fn sync_pane(
    mut commands: Commands,
    state: Res<HelpState>,
    panes: Query<(Entity, Option<&Children>), With<HelpPane>>,
    added: Query<(), Added<HelpPane>>,
) {
    if !state.is_changed() && added.is_empty() {
        return;
    }
    for (pane, children) in &panes {
        for child in children.into_iter().flat_map(|c| c.iter()) {
            commands.entity(child).despawn();
        }
        let tab = state.tab;
        commands.entity(pane).with_children(|p| match tab {
            HelpTab::Controls => controls::spawn_controls(p),
            HelpTab::Guides => {
                p.spawn((
                    Text::new(keys::HELP_GUIDES_BODY),
                    L10nKey(keys::HELP_GUIDES_BODY),
                    TextColor::default(),
                    TypeStyleToken(TypeScale::BodyLarge),
                    ForegroundRole(ColorRole::OnSurfaceVariant),
                ));
                opts_gui::not_yet::spawn_label(p);
            }
            HelpTab::Report => {
                p.spawn((
                    Text::new(keys::HELP_REPORT_BODY),
                    L10nKey(keys::HELP_REPORT_BODY),
                    TextColor::default(),
                    TypeStyleToken(TypeScale::BodyLarge),
                    ForegroundRole(ColorRole::OnSurface),
                ));
                let button = ButtonBuilder::filled(keys::HELP_REPORT_ACTION)
                    .with_key(keys::HELP_REPORT_ACTION)
                    .spawn_child(p);
                p.commands().entity(button).insert((
                    ReportButton,
                    UiId::new("help.report"),
                    Node {
                        align_self: AlignSelf::FlexStart,
                        min_height: Val::Px(44.0),
                        padding: UiRect::axes(Val::Px(24.0), Val::Px(10.0)),
                        align_items: AlignItems::Center,
                        justify_content: JustifyContent::Center,
                        ..default()
                    },
                ));
            }
            HelpTab::About => about::spawn_about(p),
        });
    }
}

pub(super) fn report_click(
    mut activated: MessageReader<Activated>,
    buttons: Query<(), With<ReportButton>>,
    time: Res<Time<Virtual>>,
    mut data: ResMut<ReportModalData>,
    state: Res<State<ReportModalState>>,
    mut next: ResMut<NextState<ReportModalState>>,
) {
    for Activated(entity) in activated.read() {
        if buttons.contains(*entity) {
            request_report(&time, &mut data, &state, &mut next);
        }
    }
}
