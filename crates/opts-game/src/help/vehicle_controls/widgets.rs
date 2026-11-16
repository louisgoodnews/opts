//! The small widgets of the vehicle controls list: a heading, a note and a key chip, built from the
//! `opts_gui` tokens.

use bevy::prelude::*;
use opts_gui::l10n::L10nKey;
use opts_gui::prelude::*;

pub(super) fn heading(parent: &mut ChildSpawnerCommands, key: &'static str) {
    parent.spawn((
        Text::new(key),
        L10nKey(key),
        TextColor::default(),
        TypeStyleToken(TypeScale::TitleMedium),
        ForegroundRole(ColorRole::OnSurface),
    ));
}

pub(super) fn note(parent: &mut ChildSpawnerCommands, key: &'static str) {
    parent.spawn((
        Text::new(key),
        L10nKey(key),
        TextColor::default(),
        TypeStyleToken(TypeScale::BodyMedium),
        ForegroundRole(ColorRole::OnSurfaceVariant),
    ));
}

pub(super) fn chip(parent: &mut ChildSpawnerCommands, text: &str, min_width: f32) {
    parent
        .spawn((
            Node {
                min_width: Val::Px(min_width),
                padding: UiRect::axes(Val::Px(8.0), Val::Px(2.0)),
                ..default()
            },
            BackgroundRole(ColorRole::SurfaceContainerHighest),
            RadiusToken(RadiusLevel::Small),
        ))
        .with_children(|c| {
            c.spawn((
                Text::new(text),
                TextColor::default(),
                TypeStyleToken(TypeScale::Label),
                ForegroundRole(ColorRole::OnSurface),
            ));
        });
}
