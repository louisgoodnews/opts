//! Completes a save running on the task pool: a success toast and close, or a failure toast.

use super::*;

/// Completes a finished save: success toast and close, or failure toast and stay open.
pub fn poll_save(
    mut commands: Commands,
    mut task: ResMut<SaveTask>,
    mut data: ResMut<ReportModalData>,
    loc: Option<Res<Localization>>,
    mut next: ResMut<NextState<ReportModalState>>,
) {
    let loc = loc.as_deref();
    let Some(t) = task.0.as_mut() else { return };
    let Some(result) = block_on(future::poll_once(t)) else {
        return;
    };
    task.0 = None;
    match result {
        Ok(saved) => {
            tracing::info!("Report saved to {}", saved.report.display());
            ToastBuilder::new(tr(
                loc,
                keys::SAVED,
                &[("path", saved.report.display().to_string())],
            ))
            .with_duration(5.0)
            .spawn(&mut commands);
            next.set(ReportModalState::Inactive);
        }
        Err(e) => {
            tracing::warn!("report could not be saved: {e}");
            let text = tr(loc, keys::FAILED, &[("error", e.to_string())]);
            fail(&mut commands, &mut data, text);
        }
    }
}
