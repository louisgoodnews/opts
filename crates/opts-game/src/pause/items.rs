//! The entries of the pause overlay.

use super::*;

/// The overlay's entries.
pub fn pause_items(
    mut commands: Commands,
    clicks: params::PauseClicks,
    mut pause: ResMut<Pause>,
    clock: params::PauseClock,
    mut menu: ResMut<VehicleMenu>,
    dialog: params::ReportAccess,
) {
    let params::PauseClicks {
        mut activated,
        items,
        disabled,
    } = clicks;
    let params::PauseClock { mut time, rebuild } = clock;
    let params::ReportAccess {
        mut report,
        state: report_state,
        next: mut report_next,
    } = dialog;
    for Activated(entity) in activated.read() {
        let Ok(which) = items.get(*entity) else {
            continue;
        };
        if disabled.contains(*entity) {
            continue;
        }
        match which {
            PauseItem::Resume => {
                environment::resume_drive(&mut commands, &mut pause, &mut time, rebuild.as_deref());
            }
            PauseItem::Environment => {
                // One dialog at a time: the overlay steps aside and returns when the page closes.
                if let Some(root) = pause.root.take() {
                    commands.entity(root).try_despawn();
                }
                environment::open_environment_page(&mut commands);
            }
            PauseItem::Vehicle => {
                close_pause(&mut commands, &mut pause, &mut time, true);
                menu.open_main();
            }
            PauseItem::Save | PauseItem::Load => {
                // One dialog at a time: the overlay steps aside and returns when the list closes.
                if let Some(root) = pause.root.take() {
                    commands.entity(root).try_despawn();
                }
                let mode = if *which == PauseItem::Save {
                    crate::save::ui::SavesMode::Save
                } else {
                    crate::save::ui::SavesMode::Load
                };
                crate::save::ui::open_saves_overlay(&mut commands, mode);
            }
            PauseItem::Report => {
                // The report takes its screenshot of the game, not of the overlay; the overlay
                // comes back when the dialog closes and the simulation stays paused.
                close_pause(&mut commands, &mut pause, &mut time, false);
                pause.after_report = true;
                pause.seen_report = false;
                request_report(&time, &mut report, &report_state, &mut report_next);
            }
            PauseItem::Leave => {
                // Always one question (interaction-and-feel section 6): the user chose to end the
                // session. The overlay steps aside (one dialog at a time) and returns on Cancel.
                if let Some(root) = pause.root.take() {
                    commands.entity(root).try_despawn();
                }
                spawn_confirm_dialog(&mut commands, leave_spec());
            }
            PauseItem::Controls => {
                // One dialog at a time: the overlay steps aside and returns when Help closes.
                if let Some(root) = pause.root.take() {
                    commands.entity(root).try_despawn();
                }
                crate::help::open_overlay(&mut commands);
            }
            PauseItem::Settings => {
                // One dialog at a time: the overlay steps aside and returns when Settings close.
                if let Some(root) = pause.root.take() {
                    commands.entity(root).try_despawn();
                }
                crate::settings_screen::open_settings_overlay(&mut commands);
            }
        }
    }
}
