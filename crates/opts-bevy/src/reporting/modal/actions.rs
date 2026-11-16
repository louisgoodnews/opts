//! The modal's buttons and Escape: start the save of the report, or cancel it.

use super::*;

/// System parameters for modal report actions.
#[derive(bevy::ecs::system::SystemParam)]
pub struct ReportModalActionParams<'w, 's> {
    pub clicks: MessageReader<'w, 's, ButtonClicked>,
    pub cancels: MessageReader<'w, 's, ModalEvent>,
    pub report: Query<'w, 's, (), With<ReportButton>>,
    pub cancel: Query<'w, 's, (), With<CancelButton>>,
    pub area: Query<'w, 's, &'static TextArea>,
    pub data: ResMut<'w, ReportModalData>,
    pub loc: Option<Res<'w, Localization>>,
    pub ctx: Res<'w, ReportContext>,
    pub dir: Res<'w, ReportsDir>,
    pub task: ResMut<'w, SaveTask>,
    pub next: ResMut<'w, NextState<ReportModalState>>,
}

/// Handles the Report and Cancel buttons and Escape.
pub fn handle_modal_actions(mut p: ReportModalActionParams, mut commands: Commands) {
    let saving = p.task.0.is_some();
    let mut want_cancel = p.cancels.read().count() > 0;
    let mut want_report = false;
    for ButtonClicked(e) in p.clicks.read() {
        want_report |= p.report.contains(*e);
        want_cancel |= p.cancel.contains(*e);
    }
    if saving {
        return;
    }
    if want_cancel {
        tracing::debug!("Report cancelled");
        p.next.set(ReportModalState::Inactive);
        return;
    }
    if !want_report {
        return;
    }
    let (Some(dir), Some(png)) = (p.dir.0.clone(), p.data.png.clone()) else {
        tracing::warn!("report cannot be saved: no reports directory or screenshot");
        fail(
            &mut commands,
            &mut p.data,
            tr(p.loc.as_deref(), keys::NO_TARGET, &[]),
        );
        return;
    };
    let text = p
        .area
        .iter()
        .next()
        .map_or_else(|| p.data.text.clone(), |a| a.value().to_owned());
    let (stem, timestamp) = now_stamp();
    let meta = ReportMeta::from_context(&p.ctx, timestamp, Some("TextArea"));
    let body = format_report(&meta, &text);
    p.task.0 = Some(
        AsyncComputeTaskPool::get().spawn(async move { save_report(&dir, &stem, &png, &body) }),
    );
}
