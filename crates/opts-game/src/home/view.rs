//! Building the Home page: the Continue card, Quick drive, recent drives.

use super::*;

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

fn vehicle_line(record: &DriveRecord) -> (Option<L10nArgs>, Text, Option<L10nKey>) {
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

fn spawn_vehicle_line(parent: &mut ChildSpawnerCommands, record: &DriveRecord, role: ColorRole) {
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

#[allow(clippy::too_many_arguments)]
pub(super) fn spawn_home(
    mut commands: Commands,
    shell: Res<ShellContent>,
    mut images: ResMut<Assets<Image>>,
    lists: Res<GameCatalogue>,
    class: Res<LayoutClass>,
    mut drives: ResMut<HomeDrives>,
) {
    if drives.persist {
        drives.history = DriveHistory::load();
    }
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
    let hero = last.as_ref().and_then(|r| {
        let folder = map_names(r, &lists).1?;
        let dir = crate::omsi_root::known_omsi_root()?
            .join("maps")
            .join(folder);
        let (handle, stats) = load_map_picture(&dir, &mut images)?;
        let size = images.get(&handle).map(Image::size_f32)?;
        Some((handle, stats, size))
    });
    commands.entity(root).with_children(|page| {
        page.spawn(Node {
            justify_content: JustifyContent::SpaceBetween,
            align_items: AlignItems::Center,
            flex_wrap: FlexWrap::Wrap,
            column_gap: Val::Px(16.0),
            ..default()
        })
        .with_children(|header| {
            let title = if last.is_some() {
                keys::HOME_WELCOME_BACK
            } else {
                keys::HOME_WELCOME_FIRST
            };
            header.spawn(text(title, TypeScale::Headline, ColorRole::OnSurface));
            header
                .spawn((
                    Node {
                        padding: UiRect::axes(Val::Px(12.0), Val::Px(6.0)),
                        ..default()
                    },
                    BackgroundRole(ColorRole::SurfaceContainerHigh),
                    RadiusToken(RadiusLevel::Full),
                ))
                .with_children(|chip| {
                    chip.spawn((
                        text(
                            keys::HOME_STATUS_NONE,
                            TypeScale::Label,
                            ColorRole::OnSurfaceVariant,
                        ),
                        StatusChip,
                    ));
                });
        });
        match &last {
            Some(record) => spawn_continue(page, record, &lists, hero),
            None => spawn_quick(page),
        }
        spawn_recents(page, &recents, &lists, last.is_some());
        let load = ButtonBuilder::tonal(keys::HOME_LOAD_GAME)
            .with_key(keys::HOME_LOAD_GAME)
            .spawn_child(page);
        page.commands().entity(load).insert((
            Node {
                align_self: AlignSelf::FlexStart,
                min_height: Val::Px(48.0),
                padding: UiRect::axes(Val::Px(24.0), Val::Px(10.0)),
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                ..default()
            },
            NotAvailableYet::new(&HOME_LOAD_GAME),
            UiId::new("home.load-game"),
        ));
    });
}

fn spawn_continue(
    page: &mut ChildSpawnerCommands,
    record: &DriveRecord,
    lists: &GameCatalogue,
    hero: Option<(Handle<Image>, opts_gui::theme::ImageLuminance, Vec2)>,
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
    let fill = Node {
        position_type: PositionType::Absolute,
        left: Val::Px(0.0),
        top: Val::Px(0.0),
        width: Val::Percent(100.0),
        height: Val::Percent(100.0),
        ..default()
    };
    page.commands().entity(card).with_children(|c| {
        if let Some((handle, stats, size)) = hero {
            c.spawn((
                ImageNode::new(handle),
                fill.clone(),
                RadiusToken(RadiusLevel::Large),
                HeroImage { size },
                opts_gui::ui_tree::A11yHidden,
            ));
            c.spawn((
                fill.clone(),
                RadiusToken(RadiusLevel::Large),
                BackgroundColor(Color::NONE),
                MenuBackgroundScrim(stats),
            ));
        }
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

fn spawn_quick(page: &mut ChildSpawnerCommands) {
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

fn spawn_recents(
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
