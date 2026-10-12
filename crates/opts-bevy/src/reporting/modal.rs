//! The report modal UI and its button/Escape handling.

use bevy::prelude::*;
use bevy::tasks::{block_on, futures_lite::future, AsyncComputeTaskPool, Task};
use opts_gui::interaction::{events::ButtonClicked, focus::FocusEvent};
use opts_gui::l10n::{L10nKey, Localization};
use opts_gui::prelude::*;

use super::format::{format_report, now_stamp, ReportMeta};
use super::keys;
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

/// The inline error line of the dialog.
#[derive(Component)]
pub struct ReportErrorText;

/// The message `key` in the user's language, with `args`; the English fallback without tables.
pub(super) fn tr(loc: Option<&Localization>, key: (&str, &str), args: &[(&str, String)]) -> String {
    let formatted: Vec<(String, opts_gui::l10n::fluent::Arg)> = args
        .iter()
        .map(|(k, v)| ((*k).to_owned(), v.clone().into()))
        .collect();
    loc.and_then(|l| l.format(key.0, &formatted))
        .unwrap_or_else(|| {
            args.iter().fold(key.1.to_owned(), |text, (k, v)| {
                text.replace(&format!("{{ ${k} }}"), v)
            })
        })
}

/// z-index above the modal so failure toasts stay visible.
const TOAST_Z: i32 = 2000;

/// Spawns the modal: screenshot preview, label, text area, Report/Cancel.
pub fn spawn_modal(
    mut commands: Commands,
    data: Res<ReportModalData>,
    images: Res<Assets<Image>>,
    loc: Option<Res<Localization>>,
) {
    let loc = loc.as_deref();
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
        p.spawn((
            Text::new(tr(loc, keys::TITLE, &[])),
            L10nKey(keys::TITLE.0),
            TextColor::default(),
            TypeStyleToken(TypeScale::TitleLarge),
            ForegroundRole(ColorRole::OnSurface),
        ));
        // Preview: top 25 % of the viewport, aspect preserved, letterboxed.
        p.spawn(Node {
            height: Val::Vh(25.0),
            width: Val::Percent(100.0),
            justify_content: JustifyContent::Center,
            align_items: AlignItems::Center,
            flex_shrink: 0.0,
            ..default()
        })
        .with_children(|frame| {
            if let Some(handle) = preview {
                frame.spawn((
                    ImageNode::new(handle),
                    // Explicit sizes (not an aspect ratio): the picture keeps its proportions, is
                    // 25 % of the viewport high and shrinks to the dialog's width.
                    Node {
                        height: Val::Vh(25.0),
                        width: Val::Vh(25.0 * aspect),
                        max_width: Val::Percent(100.0),
                        ..default()
                    },
                ));
            }
        });
        p.spawn((
            Text::new(tr(loc, keys::PREVIEW, &[])),
            L10nKey(keys::PREVIEW.0),
            TextColor::default(),
            TypeStyleToken(TypeScale::Label),
            ForegroundRole(ColorRole::OnSurfaceVariant),
        ));
        p.spawn((
            Text::new(tr(loc, keys::DESCRIBE, &[])),
            L10nKey(keys::DESCRIBE.0),
            TextColor::default(),
            TypeStyleToken(TypeScale::BodyLarge),
            ForegroundRole(ColorRole::OnSurface),
        ));
        text_area = TextAreaBuilder::new()
            .with_placeholder(tr(loc, keys::PLACEHOLDER, &[]))
            .autofocus(true)
            .spawn_child(p);
        p.spawn((
            Text::new(String::new()),
            TextColor::default(),
            TypeStyleToken(TypeScale::BodyMedium),
            ForegroundRole(ColorRole::Error),
            ReportErrorText,
        ));
        p.spawn(Node {
            column_gap: Val::Px(12.0),
            justify_content: JustifyContent::FlexEnd,
            flex_shrink: 0.0,
            ..default()
        })
        .with_children(|row| {
            let report = ButtonBuilder::filled(tr(loc, keys::SAVE, &[]))
                .with_key(keys::SAVE.0)
                .spawn_child(row);
            let cancel = ButtonBuilder::outlined(tr(loc, keys::CANCEL, &[]))
                .with_key(keys::CANCEL.0)
                .spawn_child(row);
            row.commands().queue(move |world: &mut World| {
                world.entity_mut(report).insert(ReportButton);
                world.entity_mut(cancel).insert(CancelButton);
            });
        });
    });
    // Focus the text area so the user can type immediately (clears focus elsewhere).
    commands.queue(move |world: &mut World| {
        world.write_message(FocusEvent::Set(text_area));
    });
}

/// Despawns the modal (also when a capture failed and none exists).
pub fn despawn_modal(mut commands: Commands, roots: Query<Entity, With<ReportModalRoot>>) {
    for e in &roots {
        commands.entity(e).despawn();
    }
}

/// Keeps `ReportModalData::text` in sync with the text area.
pub fn sync_text(mut events: MessageReader<TextAreaChanged>, mut data: ResMut<ReportModalData>) {
    for ev in events.read() {
        data.text.clone_from(&ev.text);
    }
}

/// Handles the Report and Cancel buttons and Escape.
#[allow(clippy::too_many_arguments)]
pub fn handle_modal_actions(
    mut clicks: MessageReader<ButtonClicked>,
    mut cancels: MessageReader<ModalEvent>,
    report: Query<(), With<ReportButton>>,
    cancel: Query<(), With<CancelButton>>,
    area: Query<&TextArea>,
    mut data: ResMut<ReportModalData>,
    loc: Option<Res<Localization>>,
    mut commands: Commands,
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
        fail(
            &mut commands,
            &mut data,
            tr(loc.as_deref(), keys::NO_TARGET, &[]),
        );
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

/// A failure is never only a toast: it stays in the dialog as well.
fn fail(commands: &mut Commands, data: &mut ReportModalData, text: String) {
    data.error = Some(text.clone());
    let toast = ToastBuilder::new(text).with_duration(8.0).spawn(commands);
    commands.entity(toast).insert(GlobalZIndex(TOAST_Z));
}

/// Shows the current error in the dialog.
pub fn sync_error(data: Res<ReportModalData>, mut lines: Query<&mut Text, With<ReportErrorText>>) {
    if !data.is_changed() {
        return;
    }
    for mut line in &mut lines {
        let text = data.error.clone().unwrap_or_default();
        if line.0 != text {
            line.0 = text;
        }
    }
}

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
