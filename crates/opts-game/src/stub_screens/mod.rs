//! Destination screen stubs proving state transitions from the main menu.

use bevy::prelude::*;
use opts_gui::prelude::*;

use crate::app_state::AppState;

/// Marker component for UI roots of temporary destination stub screens.
#[derive(Component)]
pub struct StubScreenRoot;

/// Plugin managing lifecycle and return navigation for stubbed application screens.
pub struct StubScreensPlugin;

impl Plugin for StubScreensPlugin {
    fn build(&self, app: &mut App) {
        // Register setup/teardown hooks for stubbed destination states
        app.add_systems(OnEnter(AppState::LoadGame), spawn_load_game)
            .add_systems(OnExit(AppState::LoadGame), cleanup_stub_screen)
            .add_systems(OnEnter(AppState::Credits), spawn_credits)
            .add_systems(OnExit(AppState::Credits), cleanup_stub_screen);

        // Escape key returns to MainMenu from any stub screen
        app.add_systems(
            Update,
            escape_to_menu_system
                .run_if(
                    in_state(AppState::LoadGame)
                        .or_else(in_state(AppState::Settings))
                        .or_else(in_state(AppState::Credits)),
                )
                .run_if(opts_bevy::reporting::not_modal_active),
        );
    }
}

fn spawn_generic_stub(commands: &mut Commands, title: &'static str, subtitle: &'static str) {
    let root = commands
        .spawn((
            (
                Node {
                    width: Val::Percent(100.0),
                    height: Val::Percent(100.0),
                    align_items: AlignItems::Center,
                    justify_content: JustifyContent::Center,
                    flex_direction: FlexDirection::Column,
                    row_gap: Val::Px(24.0),
                    ..default()
                },
                BackgroundColor(Color::srgba(0.08, 0.09, 0.11, 0.95)),
            ),
            StubScreenRoot,
        ))
        .id();

    let card = CardBuilder::elevated()
        .with_padding(SpacingScale::Large)
        .spawn(commands);

    commands.entity(card).with_children(|card_parent| {
        card_parent
            .spawn(Node {
                flex_direction: FlexDirection::Column,
                align_items: AlignItems::Center,
                row_gap: Val::Px(16.0),
                min_width: Val::Px(360.0),
                ..default()
            })
            .with_children(|content| {
                content.spawn((
                    (
                        Text::new(title),
                        TextFont {
                            font_size: FontSize::Px(26.0),
                            ..default()
                        },
                        TextColor(Color::WHITE),
                    ),
                    TypeStyleToken(TypeScale::HeadlineSmall),
                    ForegroundRole(ColorRole::Primary),
                ));

                content.spawn((
                    (
                        Text::new(subtitle),
                        TextFont {
                            font_size: FontSize::Px(14.0),
                            ..default()
                        },
                        TextColor(Color::srgb(0.7, 0.7, 0.7)),
                    ),
                    TypeStyleToken(TypeScale::BodyMedium),
                    ForegroundRole(ColorRole::OnSurfaceVariant),
                ));

                DividerBuilder::horizontal().spawn_child(content);

                content.spawn((
                    (
                        Text::new("Press [Escape] or click below to return to the Main Menu."),
                        TextFont {
                            font_size: FontSize::Px(13.0),
                            ..default()
                        },
                        TextColor(Color::srgb(0.6, 0.6, 0.6)),
                    ),
                    TypeStyleToken(TypeScale::LabelMedium),
                    ForegroundRole(ColorRole::Outline),
                ));

                ButtonBuilder::filled("Back to Main Menu")
                    .on_click(|cmd, _entity| {
                        cmd.insert_resource(NextState::Pending(AppState::MainMenu));
                    })
                    .spawn_child(content);
            });
    });

    commands.entity(root).add_child(card);
}

fn spawn_load_game(mut commands: Commands) {
    spawn_generic_stub(
        &mut commands,
        "Load Saved Game",
        "Saved transit session loading is not yet available.",
    );
}

fn spawn_credits(mut commands: Commands) {
    spawn_generic_stub(
        &mut commands,
        "Credits & Attributions",
        "OPTS is an open-source (MIT or Apache-2.0) transit simulator written from public file-format \
         research; no OMSI code or assets are included.\n\
         Built with Bevy, Rust and the Material Design 3 type and colour system.\n\
         Content is read from your own OMSI 2 installation; OMSI is a trademark of its authors.",
    );
}

fn cleanup_stub_screen(mut commands: Commands, query: Query<Entity, With<StubScreenRoot>>) {
    for entity in &query {
        commands.entity(entity).despawn();
    }
}

fn escape_to_menu_system(
    keyboard: Res<ButtonInput<KeyCode>>,
    mut next_state: ResMut<NextState<AppState>>,
) {
    if keyboard.just_pressed(KeyCode::Escape) {
        next_state.set(AppState::MainMenu);
    }
}
