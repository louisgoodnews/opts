//! Building the Home page: the Continue card, Quick drive, recent drives.

use super::*;

mod header;
use header::{spawn_header, spawn_load_button};

mod continue_card;
mod quick_recents;
use continue_card::spawn_continue;
use quick_recents::{spawn_quick, spawn_recents};

pub(super) fn now_secs() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| i64::try_from(d.as_secs()).unwrap_or(0))
}

pub(super) fn text(key: &'static str, scale: TypeScale, role: ColorRole) -> impl Bundle {
    (
        Text::new(key),
        L10nKey(key),
        TextColor::default(),
        TypeStyleToken(scale),
        ForegroundRole(role),
    )
}

/// A line of dynamic content (a map's name): content, not interface text.
pub(super) fn content_text(value: String, scale: TypeScale, role: ColorRole) -> impl Bundle {
    (
        Text::new(value),
        TextColor::default(),
        TypeStyleToken(scale),
        ForegroundRole(role),
    )
}

pub(super) fn time_line(record: &DriveRecord) -> impl Bundle {
    let args = L10nArgs(vec![
        (
            "time".to_owned(),
            format!("{:02}:{:02}", record.hour, record.minute).into(),
        ),
        ("weather".to_owned(), record.weather_preset.clone().into()),
    ]);
    (
        args,
        text(
            keys::HOME_TIME_WEATHER,
            TypeScale::BodyMedium,
            ColorRole::OnSurfaceVariant,
        ),
    )
}

/// The map's display name and the folder of its picture.
pub(super) fn map_names(record: &DriveRecord, lists: &GameCatalogue) -> (String, Option<String>) {
    lists
        .maps
        .iter()
        .find(|m| m.friendly_name == record.map || m.id == record.map)
        .map_or_else(
            || (record.map.clone(), None),
            |m| {
                let shown = if m.friendly_name.is_empty() {
                    m.id.clone()
                } else {
                    m.friendly_name.clone()
                };
                (shown, Some(m.id.clone()))
            },
        )
}

pub(super) fn spawn_home(
    mut commands: Commands,
    shell: Res<ShellContent>,
    lists: Res<GameCatalogue>,
    class: Res<LayoutClass>,
    mut drives: ResMut<HomeDrives>,
) {
    // The history is read once at startup (`load_drive_history`) and kept in memory.
    drives.last = drives.history.last().cloned();
    drives.recents = drives.history.recents.iter().skip(1).cloned().collect();

    let root = spawn_screen_root(&mut commands, &shell, "Home");
    let margin = class.page_margin();
    commands.entity(root).insert((
        HomeRoot,
        Node {
            width: Val::Percent(100.0),
            height: Val::Percent(100.0),
            flex_direction: FlexDirection::Column,
            padding: UiRect::all(Val::Px(margin)),
            row_gap: Val::Px(24.0),
            overflow: Overflow::scroll_y(),
            ..default()
        },
    ));
    let last = drives.last.clone();
    let recents = drives.recents.clone();
    // The picture is decoded on a worker thread and put into the card when it is ready
    // (`hero::start_hero_loads` / `finish_hero_loads`): building Home never waits for a JPEG.
    let hero_folder = last.as_ref().and_then(|r| map_names(r, &lists).1);
    commands.entity(root).with_children(|page| {
        spawn_header(page, last.is_some());
        match &last {
            Some(record) => spawn_continue(page, record, &lists, hero_folder),
            None => spawn_quick(page),
        }
        spawn_recents(page, &recents, &lists, last.is_some());
        spawn_load_button(page);
    });
}
