//! The rows of the save list.

use bevy::prelude::*;
use opts_gui::l10n::{L10nArgs, L10nKey, Localization};
use opts_gui::prelude::*;

use super::{RowAction, SavesButton, SavesList, SavesMode, SavesUi};
use crate::in_game::omsi_world::WorldLoadJob;
use crate::pause::stats::format_duration;
use crate::save::container::SaveFile;
use crate::save::registry::{preflight, CheckEnv, Issue, IssueKind, SaveRegistry};
use crate::save::store::SaveEntry;
use crate::save::SaveStorage;
use crate::shell::keys;

/// The date of a save as the list shows it (local time, year-month-day and hour:minute).
fn date_text(unix: i64) -> String {
    chrono::DateTime::from_timestamp(unix, 0)
        .map(|d| {
            d.with_timezone(&chrono::Local)
                .format("%Y-%m-%d %H:%M")
                .to_string()
        })
        .unwrap_or_default()
}

fn issue_line(i: &Issue) -> (&'static str, ColorRole) {
    match i.kind {
        IssueKind::MissingMap => (keys::SAVES_WARN_MISSING_MAP, ColorRole::Error),
        IssueKind::MissingVehicle => (keys::SAVES_WARN_MISSING_VEHICLE, ColorRole::Error),
        IssueKind::Changed => (keys::SAVES_WARN_CHANGED, ColorRole::Tertiary),
    }
}

fn line(
    parent: &mut ChildSpawnerCommands,
    key: &'static str,
    args: Vec<(String, opts_gui::l10n::fluent::Arg)>,
    scale: TypeScale,
    role: ColorRole,
) {
    parent.spawn((
        Text::new(key),
        L10nKey(key),
        L10nArgs(args),
        TextColor::default(),
        TypeStyleToken(scale),
        ForegroundRole(role),
    ));
}

fn row_button(
    parent: &mut ChildSpawnerCommands,
    key: &'static str,
    id: &str,
    action: RowAction,
    tag: &str,
    destructive: bool,
) -> Entity {
    let builder = if destructive {
        ButtonBuilder::text(key)
    } else {
        ButtonBuilder::tonal(key)
    };
    let button = builder.with_key(key).spawn_child(parent);
    parent.commands().entity(button).insert((
        SavesButton::Row(id.to_owned(), action),
        UiId::new(format!("saves.{tag}.{id}")),
    ));
    button
}

/// The name and details of a readable save, or the file name and the reason it cannot be read.
fn row_details(
    row: &mut ChildSpawnerCommands,
    entry: &SaveEntry,
    issues: &[Issue],
    loc: &Localization,
) {
    let id = &entry.id;
    match &entry.state {
        Ok(loaded) => {
            let meta = &loaded.file.meta;
            row.spawn((
                Text::new(meta.name.clone()),
                TextColor::default(),
                TypeStyleToken(TypeScale::TitleMedium),
                ForegroundRole(ColorRole::OnSurface),
            ));
            let vehicles = if meta.vehicles.is_empty() {
                loc.t(keys::SAVES_NO_VEHICLE)
            } else {
                meta.vehicles.join(", ")
            };
            line(
                row,
                keys::SAVES_ROW_DETAIL,
                vec![
                    ("map".to_owned(), meta.map.clone().into()),
                    ("vehicles".to_owned(), vehicles.into()),
                    (
                        "played".to_owned(),
                        format_duration(meta.play_time_s as f32).into(),
                    ),
                    ("date".to_owned(), date_text(meta.saved_unix).into()),
                ],
                TypeScale::BodyMedium,
                ColorRole::OnSurfaceVariant,
            );
            if loaded.from_backup {
                line(
                    row,
                    keys::SAVES_FROM_BACKUP,
                    Vec::new(),
                    TypeScale::BodyMedium,
                    ColorRole::Tertiary,
                );
            }
            for issue in issues {
                let (key, role) = issue_line(issue);
                line(
                    row,
                    key,
                    vec![("name".to_owned(), issue.subject.clone().into())],
                    TypeScale::BodyMedium,
                    role,
                );
            }
        }
        Err(e) => {
            row.spawn((
                Text::new(id.clone()),
                TextColor::default(),
                TypeStyleToken(TypeScale::TitleMedium),
                ForegroundRole(ColorRole::OnSurface),
            ));
            line(
                row,
                keys::SAVES_UNREADABLE,
                vec![("reason".to_owned(), e.to_string().into())],
                TypeScale::BodyMedium,
                ColorRole::Error,
            );
        }
    }
}

/// The buttons of a row: *Load*, *Overwrite* (when saving into a session), *Rename* and *Delete*.
fn row_buttons(
    buttons: &mut ChildSpawnerCommands,
    entry: &SaveEntry,
    (mode, in_session): (SavesMode, bool),
) {
    let id = &entry.id;
    if entry.state.is_ok() {
        row_button(
            buttons,
            keys::SAVES_LOAD,
            id,
            RowAction::Load,
            "load",
            false,
        );
        if mode == SavesMode::Save && in_session {
            row_button(
                buttons,
                keys::SAVES_OVERWRITE,
                id,
                RowAction::Overwrite,
                "overwrite",
                false,
            );
        }
        row_button(
            buttons,
            keys::SAVES_RENAME,
            id,
            RowAction::Rename,
            "rename",
            true,
        );
    }
    row_button(
        buttons,
        keys::SAVES_DELETE,
        id,
        RowAction::Delete,
        "delete",
        true,
    );
}

fn spawn_row(
    list: &mut ChildSpawnerCommands,
    entry: &SaveEntry,
    mode: SavesMode,
    issues: &[Issue],
    loc: &Localization,
    in_session: bool,
) {
    let id = &entry.id;
    list.spawn((
        Node {
            flex_direction: FlexDirection::Column,
            row_gap: Val::Px(4.0),
            padding: UiRect::all(Val::Px(12.0)),
            flex_shrink: 0.0,
            ..default()
        },
        BackgroundRole(ColorRole::SurfaceContainerHigh),
        RadiusToken(RadiusLevel::Medium),
        UiId::new(format!("saves.row.{id}")),
    ))
    .with_children(|row| {
        row_details(row, entry, issues, loc);
        row.spawn(Node {
            flex_direction: FlexDirection::Row,
            column_gap: Val::Px(8.0),
            flex_wrap: FlexWrap::Wrap,
            ..default()
        })
        .with_children(|buttons| {
            row_buttons(buttons, entry, (mode, in_session));
        });
    });
}

/// Rebuilds the rows when the list changed.
#[allow(clippy::too_many_arguments)]
pub fn rebuild_list(
    mut commands: Commands,
    mut ui: ResMut<SavesUi>,
    storage: Res<SaveStorage>,
    registry: Res<SaveRegistry>,
    loc: Res<Localization>,
    job: Option<Res<WorldLoadJob>>,
    state: Res<State<crate::app_state::AppState>>,
    lists: Query<Entity, With<SavesList>>,
) {
    if !ui.dirty {
        return;
    }
    let Ok(list) = lists.single() else { return };
    ui.dirty = false;
    let entries = storage.0.as_ref().map(|s| s.list()).unwrap_or_default();
    let root = job
        .map(|j| j.omsi_root().to_path_buf())
        .or_else(crate::omsi_root::known_omsi_root);
    let in_session = *state.get() == crate::app_state::AppState::InGame;
    let mode = ui.mode;
    commands.entity(list).despawn_children();
    commands.entity(list).with_children(|rows| {
        if entries.is_empty() {
            line(
                rows,
                keys::SAVES_EMPTY,
                Vec::new(),
                TypeScale::BodyMedium,
                ColorRole::OnSurfaceVariant,
            );
        }
        for entry in &entries {
            let issues = entry
                .state
                .as_ref()
                .map(|l| issues_of(&registry, &l.file, root.as_deref()))
                .unwrap_or_default();
            spawn_row(rows, entry, mode, &issues, &loc, in_session);
        }
    });
}

fn issues_of(
    registry: &SaveRegistry,
    file: &SaveFile,
    root: Option<&std::path::Path>,
) -> Vec<Issue> {
    preflight(
        registry.all(),
        file,
        &CheckEnv {
            root,
            fingerprint: &file.fingerprint,
        },
    )
}
