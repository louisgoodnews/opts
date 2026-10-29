//! The header row and the *Load game* button of the home screen.

use super::*;

/// The welcome headline and the content status chip.
pub(super) fn spawn_header(page: &mut ChildSpawnerCommands, returning: bool) {
    page.spawn(Node {
        justify_content: JustifyContent::SpaceBetween,
        align_items: AlignItems::Center,
        flex_wrap: FlexWrap::Wrap,
        column_gap: Val::Px(16.0),
        ..default()
    })
    .with_children(|header| {
        let title = if returning {
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
                    crate::content_facts::WantsContentFacts,
                ));
            });
    });
}

/// The *Load game* button below the cards.
pub(super) fn spawn_load_button(page: &mut ChildSpawnerCommands) {
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
        HomeButton::LoadGame,
        UiId::new("home.load-game"),
    ));
}
