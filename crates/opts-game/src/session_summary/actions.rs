//! The summary's buttons: the report dialog and the sends to Home (*Drive again*, *Change
//! vehicle*, *Home*).

use bevy::ecs::system::SystemParam;
use bevy::prelude::*;
use opts_bevy::reporting::{request_report, ReportModalData, ReportModalState};
use opts_gui::interaction::activation::Activated;

use crate::home::StartDrive;
use crate::shell::nav::Navigate;

use super::{LastSession, SummaryAction, SummaryCard};

/// The report dialog's data and state, with the clock its request reads.
#[derive(SystemParam)]
pub struct SummaryReport<'w> {
    time: Res<'w, Time<Virtual>>,
    report: ResMut<'w, ReportModalData>,
    state: Res<'w, State<ReportModalState>>,
    next: ResMut<'w, NextState<ReportModalState>>,
}

/// Where the card's buttons send the player.
#[derive(SystemParam)]
pub struct SummaryOut<'w> {
    start: MessageWriter<'w, StartDrive>,
    navigate: MessageWriter<'w, Navigate>,
}

/// The buttons of the card.
pub fn summary_actions(
    mut commands: Commands,
    mut activated: MessageReader<Activated>,
    actions: Query<&SummaryAction>,
    cards: Query<Entity, With<SummaryCard>>,
    last: Option<Res<LastSession>>,
    dialog: SummaryReport,
    out: SummaryOut,
) {
    let SummaryReport {
        time,
        mut report,
        state: report_state,
        next: mut report_next,
    } = dialog;
    let SummaryOut {
        mut start,
        mut navigate,
    } = out;
    for Activated(entity) in activated.read() {
        let Ok(action) = actions.get(*entity) else {
            continue;
        };
        match action {
            SummaryAction::Report => {
                request_report(&time, &mut report, &report_state, &mut report_next);
                continue;
            }
            SummaryAction::Again => {
                if let Some(last) = &last {
                    start.write(StartDrive(last.record.clone()));
                }
            }
            SummaryAction::ChangeVehicle => {
                navigate.write(Navigate::push("drive"));
            }
            SummaryAction::Home => {}
        }
        for card in &cards {
            commands.entity(card).despawn();
        }
        commands.remove_resource::<LastSession>();
    }
}
