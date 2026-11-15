//! The *Quick drive* card of a first visit, and the recent drives of the home screen.

use super::*;

pub(super) fn spawn_quick(page: &mut ChildSpawnerCommands) {
    page.spawn((
        Node {
            padding: UiRect::all(Val::Px(24.0)),
            flex_direction: FlexDirection::Column,
            row_gap: Val::Px(12.0),
            align_items: AlignItems::FlexStart,
            ..default()
        },
        BackgroundRole(ColorRole::SurfaceContainerHigh),
        RadiusToken(RadiusLevel::Large),
    ))
    .with_children(|card| {
        card.spawn(text(
            keys::HOME_QUICK_TITLE,
            TypeScale::TitleLarge,
            ColorRole::OnSurface,
        ));
        card.spawn(text(
            keys::HOME_QUICK_BODY,
            TypeScale::BodyLarge,
            ColorRole::OnSurfaceVariant,
        ));
        let quick = ButtonBuilder::filled(keys::HOME_QUICK_ACTION)
            .with_key(keys::HOME_QUICK_ACTION)
            .spawn_child(card);
        card.commands().entity(quick).insert((
            HomeButton::Quick,
            DefaultFocus,
            UiId::new("home.quick-drive"),
        ));
    });
}

pub(super) fn spawn_recents(
    page: &mut ChildSpawnerCommands,
    recents: &[DriveRecord],
    lists: &GameCatalogue,
    has_last: bool,
) {
    page.spawn(Node {
        justify_content: JustifyContent::SpaceBetween,
        align_items: AlignItems::Center,
        ..default()
    })
    .with_children(|head| {
        if !recents.is_empty() {
            head.spawn(text(
                keys::HOME_RECENT,
                TypeScale::TitleLarge,
                ColorRole::OnSurface,
            ));
        }
        let new = if has_last {
            ButtonBuilder::tonal(keys::HOME_NEW_DRIVE)
        } else {
            ButtonBuilder::outlined(keys::HOME_NEW_DRIVE)
        }
        .with_key(keys::HOME_NEW_DRIVE)
        .spawn_child(head);
        head.commands()
            .entity(new)
            .insert((HomeButton::NewDrive, UiId::new("home.new-drive")));
    });
    if recents.is_empty() {
        return;
    }
    page.spawn(Node {
        flex_wrap: FlexWrap::Wrap,
        column_gap: Val::Px(16.0),
        row_gap: Val::Px(16.0),
        ..default()
    })
    .with_children(|grid| {
        for (i, record) in recents.iter().enumerate() {
            let card = grid
                .spawn((
                    Button,
                    Node {
                        min_width: Val::Px(240.0),
                        flex_grow: 1.0,
                        flex_basis: Val::Px(240.0),
                        padding: UiRect::all(Val::Px(16.0)),
                        flex_direction: FlexDirection::Column,
                        row_gap: Val::Px(4.0),
                        ..default()
                    },
                    BackgroundRole(ColorRole::SurfaceContainerLow),
                    StateLayerRole(ColorRole::OnSurface),
                    RadiusToken(RadiusLevel::Medium),
                    HomeButton::Recent(i),
                    UiId::new(format!("home.recent.{i}")),
                ))
                .id();
            grid.commands().entity(card).with_children(|c| {
                c.spawn(content_text(
                    map_names(record, lists).0,
                    TypeScale::TitleMedium,
                    ColorRole::OnSurface,
                ));
                c.spawn((
                    Text::new(record.vehicle.clone()),
                    TextColor::default(),
                    TypeStyleToken(TypeScale::BodyMedium),
                    ForegroundRole(ColorRole::OnSurfaceVariant),
                ));
                c.spawn(time_line(record));
            });
        }
    });
}
