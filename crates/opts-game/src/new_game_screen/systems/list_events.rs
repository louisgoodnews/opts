//! Master-detail list selection, favourites, and start-point events for the wizard.

use super::*;

mod select;
use select::on_selected;

/// The start point selector changed.
pub fn handle_start_point(
    mut events: MessageReader<ComboboxSelected>,
    boxes: Query<(), With<StartPointBox>>,
    mut setup: ResMut<NewGameSetup>,
) {
    for ev in events.read() {
        if boxes.contains(ev.entity) && setup.selected_entry_point != ev.selected_value {
            setup.selected_entry_point.clone_from(&ev.selected_value);
        }
    }
}

/// The resources the list handlers read and change.
#[derive(bevy::ecs::system::SystemParam)]
pub struct ListState<'w> {
    setup: ResMut<'w, NewGameSetup>,
    catalogue: Res<'w, GameCatalogue>,
    weather: Res<'w, opts_bevy::WeatherCatalogue>,
    history: ResMut<'w, PlannerHistory>,
    liveries: ResMut<'w, LiveryState>,
    library: Res<'w, super::super::configs::ConfigLibrary>,
    state: ResMut<'w, PlannerState>,
    loc: Res<'w, Localization>,
    fixed_today: Option<Res<'w, super::super::planner::Today>>,
    trips: Res<'w, super::super::trips::TripState>,
}


/// A star on a list entry.
fn on_favourite(s: &mut ListState, step: PlannerStep, ev: &MasterDetailFavourite) {
    if !matches!(step, PlannerStep::Where | PlannerStep::What) {
        return;
    }
    let vehicle = step == PlannerStep::What;
    if let Err(e) = s.history.set_favourite(vehicle, &ev.id, ev.favourite) {
        tracing::warn!("cannot save favourites: {e}");
    }
}

/// *Enter* on a list entry goes on to the next step when the choices so far are valid.
fn on_chosen(s: &mut ListState) {
    if step_invalid(s.state.step, &s.setup, &s.catalogue).is_none() {
        if let Some(n) = s.state.next_step() {
            s.state.step = n;
        }
    }
}

/// Selection, stars and *Enter* of the lists.
pub fn handle_list_events(
    mut selected: MessageReader<MasterDetailSelected>,
    mut favourites: MessageReader<MasterDetailFavourite>,
    mut chosen: MessageReader<MasterDetailChosen>,
    lists: Query<(&StepList, &MasterDetailView)>,
    mut s: ListState,
    mut eras: Local<std::collections::HashMap<String, Option<MapEra>>>,
    mut commands: Commands,
) {
    for ev in selected.read() {
        let Ok((StepList(step), view)) = lists.get(ev.root) else {
            continue;
        };
        on_selected(
            &mut commands,
            &mut s,
            &mut eras,
            (*step, &ev.id, view.parts().detail),
        );
    }
    for ev in favourites.read() {
        if let Ok((StepList(step), _)) = lists.get(ev.root) {
            on_favourite(&mut s, *step, ev);
        }
    }
    for ev in chosen.read() {
        if lists.contains(ev.root) {
            on_chosen(&mut s);
        }
    }
}


