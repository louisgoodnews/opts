//! Showing and removing banners: the spawned tree and its colours.

use super::*;

fn colours(severity: BannerSeverity) -> (ColorRole, ColorRole, &'static str) {
    match severity {
        BannerSeverity::Info => (
            ColorRole::SecondaryContainer,
            ColorRole::OnSecondaryContainer,
            icons::INFO,
        ),
        BannerSeverity::Warning => (
            ColorRole::TertiaryContainer,
            ColorRole::OnTertiaryContainer,
            icons::WARNING,
        ),
        BannerSeverity::Error => (
            ColorRole::ErrorContainer,
            ColorRole::OnErrorContainer,
            icons::ERROR,
        ),
    }
}

/// Shows a banner, replacing one with the same id. Needs the shell frame.
pub fn show_banner(commands: &mut Commands, spec: BannerSpec) {
    commands.queue(move |world: &mut World| {
        dismiss_in_world(world, spec.id);
        let area = world
            .query_filtered::<Entity, With<ShellBanners>>()
            .iter(world)
            .next();
        let Some(area) = area else {
            tracing::warn!(id = spec.id, "no banner area: the shell frame is missing");
            return;
        };
        let mut commands = world.commands();
        spawn_banner(&mut commands, area, &spec);
        world.flush();
    });
}

/// Removes the banner with `id`, if shown.
pub fn dismiss_banner(commands: &mut Commands, id: &'static str) {
    commands.queue(move |world: &mut World| dismiss_in_world(world, id));
}

fn dismiss_in_world(world: &mut World, id: &'static str) {
    let found: Vec<Entity> = world
        .query::<(Entity, &Banner)>()
        .iter(world)
        .filter(|(_, b)| b.id == id)
        .map(|(e, _)| e)
        .collect();
    for entity in found {
        if let Ok(e) = world.get_entity_mut(entity) {
            e.despawn();
        }
    }
}

fn spawn_banner(commands: &mut Commands, area: Entity, spec: &BannerSpec) {
    let (bg, fg, glyph) = colours(spec.severity);
    let banner = commands
        .spawn((
            Node {
                margin: UiRect::axes(Val::Px(16.0), Val::Px(8.0)),
                padding: UiRect::axes(Val::Px(16.0), Val::Px(12.0)),
                column_gap: Val::Px(16.0),
                align_items: AlignItems::Center,
                ..default()
            },
            BackgroundRole(bg),
            RadiusToken(RadiusLevel::Medium),
            Banner { id: spec.id },
            UiId::new(format!("banner.{}", spec.id)),
        ))
        .id();
    if let Some(owner) = spec.owner {
        commands.entity(banner).insert(BannerOwner(owner));
    }
    commands.entity(area).add_child(banner);
    commands.entity(banner).with_children(|b| {
        b.spawn((
            Text::new(glyph),
            TextFont {
                font: FontSource::Handle(opts_gui::fonts::ICON_FONT),
                font_size: FontSize::Px(24.0),
                ..default()
            },
            TextColor::default(),
            ForegroundRole(fg),
            opts_gui::ui_tree::A11yHidden,
        ));
        b.spawn(Node {
            flex_direction: FlexDirection::Column,
            flex_grow: 1.0,
            flex_shrink: 1.0,
            min_width: Val::Px(0.0),
            ..default()
        })
        .with_children(|col| {
            col.spawn((
                Text::new(spec.title_key),
                L10nKey(spec.title_key),
                spec.args.clone(),
                TextColor::default(),
                TypeStyleToken(TypeScale::TitleMedium),
                ForegroundRole(fg),
            ));
            col.spawn((
                Text::new(spec.body_key),
                L10nKey(spec.body_key),
                spec.args.clone(),
                TextColor::default(),
                TypeStyleToken(TypeScale::BodyMedium),
                ForegroundRole(fg),
            ));
        });
        if let Some((action, label_key)) = spec.action {
            let button = ButtonBuilder::filled(label_key)
                .with_key(label_key)
                .spawn_child(b);
            b.commands().entity(button).insert((
                BannerActionButton {
                    banner: spec.id,
                    action,
                },
                UiId::new(format!("banner.{}.{}", spec.id, action)),
            ));
        }
    });
}
