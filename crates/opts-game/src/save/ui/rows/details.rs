//! The text of a save row: its name, details, warnings, or the reason it cannot be read.

use super::*;

/// The date of a save as the list shows it (local time, year-month-day and hour:minute).
pub(super) fn date_text(unix: i64) -> String {
    chrono::DateTime::from_timestamp(unix, 0)
        .map(|d| {
            d.with_timezone(&chrono::Local)
                .format("%Y-%m-%d %H:%M")
                .to_string()
        })
        .unwrap_or_default()
}

pub(super) fn issue_line(i: &Issue) -> (&'static str, ColorRole) {
    match i.kind {
        IssueKind::MissingMap => (keys::SAVES_WARN_MISSING_MAP, ColorRole::Error),
        IssueKind::MissingVehicle => (keys::SAVES_WARN_MISSING_VEHICLE, ColorRole::Error),
        IssueKind::Changed => (keys::SAVES_WARN_CHANGED, ColorRole::Tertiary),
    }
}

pub(super) fn line(
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

/// The name and details of a readable save, or the file name and the reason it cannot be read.
pub(super) fn row_details(
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
