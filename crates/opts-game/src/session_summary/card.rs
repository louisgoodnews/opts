//! The summary card: built in the banner area when Home opens after a drive, removed when
//! Home is left.

use bevy::prelude::*;
use opts_gui::l10n::{L10nKey, Localization};
use opts_gui::prelude::*;

use crate::shell::frame::ShellBanners;
use crate::shell::keys;

use super::rows::{action_row, problems_row, stat_row};
use super::{LastSession, SummaryCard};

/// Builds the card in the banner area when Home opens after a drive that moved.
pub fn spawn_summary(
    mut commands: Commands,
    last: Option<Res<LastSession>>,
    area: Query<Entity, With<ShellBanners>>,
    existing: Query<(), With<SummaryCard>>,
    loc: Res<Localization>,
) {
    let (Some(last), Ok(area)) = (last, area.single()) else {
        return;
    };
    if !existing.is_empty() {
        return;
    }
    let stats = &last.stats;
    let card = commands
        .spawn((
            Node {
                margin: UiRect::axes(Val::Px(16.0), Val::Px(8.0)),
                padding: UiRect::all(Val::Px(16.0)),
                flex_direction: FlexDirection::Column,
                row_gap: Val::Px(8.0),
                ..default()
            },
            BackgroundRole(ColorRole::SurfaceContainerHigh),
            RadiusToken(RadiusLevel::Large),
            SummaryCard,
            Name::new("SessionSummary"),
        ))
        .id();
    commands.entity(area).add_child(card);
    let locale = loc.locale().to_owned();
    commands.entity(card).with_children(|c| {
        c.spawn((
            Text::new(keys::SUMMARY_TITLE),
            L10nKey(keys::SUMMARY_TITLE),
            TextColor::default(),
            TypeStyleToken(TypeScale::TitleLarge),
            ForegroundRole(ColorRole::OnSurface),
        ));
        stat_row(c, stats, &locale);
        problems_row(c, stats);
        action_row(c);
    });
}

/// Removes the card when Home is left.
pub fn clear_summary(mut commands: Commands, cards: Query<Entity, With<SummaryCard>>) {
    for card in &cards {
        commands.entity(card).despawn();
    }
}
