//! The report modal UI and its button/Escape handling.

use bevy::prelude::*;
use bevy::tasks::{block_on, futures_lite::future, AsyncComputeTaskPool, Task};
use opts_gui::interaction::{events::ButtonClicked, focus::FocusEvent};
use opts_gui::prelude::*;

use super::format::{format_report, now_stamp, ReportMeta};
use super::save::{save_report, SavedReport};
use super::state::{ReportContext, ReportModalData, ReportModalState};

/// Marker on the modal's backdrop (despawned recursively on close).
#[derive(Component)]
pub struct ReportModalRoot;
/// Marker on the Report button.
#[derive(Component)]
pub struct ReportButton;
/// Marker on the Cancel button.
#[derive(Component)]
pub struct CancelButton;

/// Where reports are written (resolved and created at start-up).
#[derive(Resource, Debug, Default, Clone)]
pub struct ReportsDir(pub Option<std::path::PathBuf>);

/// A save running on the task pool.
#[derive(Resource, Default)]
pub struct SaveTask(Option<Task<std::io::Result<SavedReport>>>);

/// z-index above the modal so failure toasts stay visible.
const TOAST_Z: i32 = 2000;

/// Spawns the modal: screenshot preview, label, text area, Report/Cancel.
pub fn spawn_modal(mut commands: Commands, data: Res<ReportModalData>, images: Res<Assets<Image>>) {
    let (root, content) = ModalBuilder::new().spawn(&mut commands);
    commands.entity(root).insert(ReportModalRoot);
    let preview = data.screenshot.clone();
    let aspect = preview
        .as_ref()
        .and_then(|h| images.get(h))
        .map(|i| i.width() as f32 / i.height().max(1) as f32)
        .unwrap_or(16.0 / 9.0);
    let mut text_area = Entity::PLACEHOLDER;
    commands.entity(content).with_children(|p| {
        // Preview: top 25 % of the viewport, aspect preserved, letterboxed.
        p.spawn(NodeBundle {
            style: Style {
                height: Val::Vh(25.0),
                width: Val::Percent(100.0),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                flex_shrink: 0.0,
                ..default()
            },
            ..default()
        })
        .with_children(|frame| {
            if let Some(handle) = preview {
                frame.spawn(ImageBundle {
                    image: UiImage::new(handle),
                    style: Style {
                        height: Val::Percent(100.0),
                        aspect_ratio: Some(aspect),
                        max_width: Val::Percent(100.0),
                        ..default()
                    },
                    ..default()
                });
            }
        });
        p.spawn((
            TextBundle::from_section(
                "Describe the issue:",
                TextStyle {
                    font_size: 16.0,
                    color: Color::WHITE,
                    ..default()
                },
            ),
            ForegroundRole(ColorRole::OnSurface),
        ));
        text_area = TextAreaBuilder::new()
            .with_placeholder("What happened? What did you expect?")
            .autofocus(true)
            .spawn_child(p);
        p.spawn(NodeBundle {
            style: Style {
                column_gap: Val::Px(12.0),
                justify_content: JustifyContent::FlexEnd,
                flex_shrink: 0.0,
                ..default()
            },
            ..default()
        })
        .with_children(|row| {
            let report = ButtonBuilder::filled("Report").spawn_child(row);
            let cancel = ButtonBuilder::outlined("Cancel").spawn_child(row);
            row.add_command(move |world: &mut World| {
                world.entity_mut(report).insert(ReportButton);
                world.entity_mut(cancel).insert(CancelButton);
            });
        });
    });
    // Focus the text area so the user can type immediately (clears focus elsewhere).
    commands.add(move |world: &mut World| {
        world.send_event(FocusEvent::Set(text_area));
    });
}

/// Despawns the modal (also when a capture failed and none exists).
pub fn despawn_modal(mut commands: Commands, roots: Query<Entity, With<ReportModalRoot>>) {
    for e in &roots {
        commands.entity(e).despawn_recursive();
    }
}

/// Keeps `ReportModalData::text` in sync with the text area.
pub fn sync_text(mut events: EventReader<TextAreaChanged>, mut data: ResMut<ReportModalData>) {
    for ev in events.read() {
        data.text.clone_from(&ev.text);
    }
}

/// Handles the Report and Cancel buttons and Escape.
#[allow(clippy::too_many_arguments)]
pub fn handle_modal_actions(
    mut clicks: EventReader<ButtonClicked>,
    mut cancels: EventReader<ModalEvent>,
    report: Query<(), With<ReportButton>>,
    cancel: Query<(), With<CancelButton>>,
    area: Query<&TextArea>,
    data: Res<ReportModalData>,
    ctx: Res<ReportContext>,
    dir: Res<ReportsDir>,
    mut task: ResMut<SaveTask>,
    mut next: ResMut<NextState<ReportModalState>>,
) {
    let saving = task.0.is_some();
    let mut want_cancel = cancels.read().count() > 0;
    let mut want_report = false;
    for ButtonClicked(e) in clicks.read() {
        want_report |= report.contains(*e);
        want_cancel |= cancel.contains(*e);
    }
    if saving {
        return;
    }
    if want_cancel {
        tracing::debug!("Report cancelled");
        next.set(ReportModalState::Inactive);
        return;
    }
    if !want_report {
        return;
    }
    let (Some(dir), Some(png)) = (dir.0.clone(), data.png.clone()) else {
        tracing::warn!("report cannot be saved: no reports directory or screenshot");
        return;
    };
    let text = area
        .iter()
        .next()
        .map_or_else(|| data.text.clone(), |a| a.value().to_owned());
    let (stem, timestamp) = now_stamp();
    let meta = ReportMeta::from_context(&ctx, timestamp, Some("TextArea"));
    let body = format_report(&meta, &text);
    task.0 = Some(
        AsyncComputeTaskPool::get().spawn(async move { save_report(&dir, &stem, &png, &body) }),
    );
}

/// Completes a finished save: success toast and close, or failure toast and stay open.
pub fn poll_save(
    mut commands: Commands,
    mut task: ResMut<SaveTask>,
    mut next: ResMut<NextState<ReportModalState>>,
) {
    let Some(t) = task.0.as_mut() else { return };
    let Some(result) = block_on(future::poll_once(t)) else {
        return;
    };
    task.0 = None;
    match result {
        Ok(saved) => {
            tracing::info!("Report saved to {}", saved.report.display());
            ToastBuilder::new(format!("Report saved to {}", saved.report.display()))
                .with_duration(5.0)
                .spawn(&mut commands);
            next.set(ReportModalState::Inactive);
        }
        Err(e) => {
            tracing::warn!("report could not be saved: {e}");
            let toast = ToastBuilder::new(format!("Could not save the report: {e}"))
                .with_duration(8.0)
                .spawn(&mut commands);
            commands.entity(toast).insert(ZIndex::Global(TOAST_Z));
        }
    }
}
