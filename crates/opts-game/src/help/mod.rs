//! Help: controls, guides, report an issue and About (`docs/src/roadmap/phase-9/screens.md` §6).

use bevy::prelude::*;
use opts_gui::l10n::L10nKey;
use opts_gui::prelude::*;

use crate::app_state::AppState;
use crate::shell::{keys, spawn_screen_root, ShellContent};

/// Root of the Help page.
#[derive(Component, Debug, Clone, Copy)]
pub struct HelpRoot;

/// Help page plugin.
pub struct HelpPlugin;

impl Plugin for HelpPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(OnEnter(AppState::Help), spawn_help)
            .add_systems(OnExit(AppState::Help), despawn_help);
    }
}

fn spawn_help(mut commands: Commands, shell: Res<ShellContent>) {
    let root = spawn_screen_root(&mut commands, &shell, "Help");
    commands.entity(root).insert((
        HelpRoot,
        Node {
            width: Val::Percent(100.0),
            height: Val::Percent(100.0),
            flex_direction: FlexDirection::Column,
            padding: UiRect::all(Val::Px(32.0)),
            row_gap: Val::Px(16.0),
            ..default()
        },
    ));
    commands.entity(root).with_children(|page| {
        for (key, scale, role) in [
            (keys::HELP_TITLE, TypeScale::Headline, ColorRole::OnSurface),
            (
                keys::HELP_ABOUT,
                TypeScale::BodyLarge,
                ColorRole::OnSurfaceVariant,
            ),
        ] {
            page.spawn((
                Text::new(key),
                L10nKey(key),
                TextColor::default(),
                TypeStyleToken(scale),
                ForegroundRole(role),
            ));
        }
    });
}

fn despawn_help(mut commands: Commands, roots: Query<Entity, With<HelpRoot>>) {
    for root in &roots {
        commands.entity(root).despawn();
    }
}
