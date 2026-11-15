//! The *Continue* card of the home screen: the last drive, with its own *Drive* button.

use super::*;

pub(super) fn vehicle_line(record: &DriveRecord) -> (Option<L10nArgs>, Text, Option<L10nKey>) {
    if record.livery.is_empty() {
        (None, Text::new(record.vehicle.clone()), None)
    } else {
        let args = L10nArgs(vec![
            ("vehicle".to_owned(), record.vehicle.clone().into()),
            ("livery".to_owned(), record.livery.clone().into()),
        ]);
        (
            Some(args),
            Text::new(keys::HOME_VEHICLE_LIVERY),
            Some(L10nKey(keys::HOME_VEHICLE_LIVERY)),
        )
    }
}

pub(super) fn spawn_vehicle_line(
    parent: &mut ChildSpawnerCommands,
    record: &DriveRecord,
    role: ColorRole,
) {
    let (args, text, key) = vehicle_line(record);
    let mut line = parent.spawn((
        text,
        TextColor::default(),
        TypeStyleToken(TypeScale::TitleMedium),
        ForegroundRole(role),
    ));
    if let Some(args) = args {
        line.insert(args);
    }
    if let Some(key) = key {
        line.insert(key);
    }
}

pub(super) fn spawn_continue(
    page: &mut ChildSpawnerCommands,
    record: &DriveRecord,
    lists: &GameCatalogue,
    hero_folder: Option<String>,
) {
    let (map_name, _) = map_names(record, lists);
    let card = page
        .spawn((
            Node {
                height: Val::Px(260.0),
                flex_shrink: 0.0,
                overflow: Overflow::clip(),
                ..default()
            },
            BackgroundRole(ColorRole::SurfaceContainerHigh),
            RadiusToken(RadiusLevel::Large),
            Name::new("ContinueCard"),
        ))
        .id();
    if let Some(folder) = hero_folder {
        page.commands().entity(card).insert(HeroPending { folder });
    }
    page.commands().entity(card).with_children(|c| {
        c.spawn(Node {
            width: Val::Percent(100.0),
            height: Val::Percent(100.0),
            padding: UiRect::all(Val::Px(24.0)),
            flex_direction: FlexDirection::Row,
            justify_content: JustifyContent::SpaceBetween,
            align_items: AlignItems::FlexEnd,
            column_gap: Val::Px(16.0),
            ..default()
        })
        .with_children(|row| {
            row.spawn(Node {
                flex_direction: FlexDirection::Column,
                row_gap: Val::Px(4.0),
                flex_shrink: 1.0,
                ..default()
            })
            .with_children(|col| {
                col.spawn(text(
                    keys::HOME_CONTINUE,
                    TypeScale::Label,
                    ColorRole::OnSurfaceVariant,
                ));
                col.spawn(content_text(
                    map_name,
                    TypeScale::Headline,
                    ColorRole::OnSurface,
                ));
                spawn_vehicle_line(col, record, ColorRole::OnSurface);
                col.spawn(time_line(record));
            });
            let drive = ButtonBuilder::filled(keys::HOME_CONTINUE_DRIVE)
                .with_key(keys::HOME_CONTINUE_DRIVE)
                .spawn_child(row);
            row.commands().entity(drive).insert((
                HomeButton::Continue,
                DefaultFocus,
                UiId::new("home.continue"),
            ));
        });
    });
}
