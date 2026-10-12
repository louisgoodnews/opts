//! Home: the first screen of the shell (`docs/src/roadmap/phase-9/screens.md` section 2).

use bevy::prelude::*;
use opts_gui::interaction::activation::Activated;
use opts_gui::not_yet::{NotAvailableYet, HOME_LOAD_GAME};
use opts_gui::prelude::*;

use crate::app_state::AppState;
use crate::shell::{keys, nav::Navigate, spawn_screen_root, ShellContent};

/// Root of the Home page.
#[derive(Component, Debug, Clone, Copy)]
pub struct HomeRoot;

/// Actions of the Home page.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub enum HomeAction {
    /// Open *Drive*.
    NewDrive,
}

/// Home page plugin.
pub struct HomePlugin;

impl Plugin for HomePlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(OnEnter(AppState::MainMenu), spawn_home)
            .add_systems(OnExit(AppState::MainMenu), despawn_home)
            .add_systems(Update, home_actions.run_if(in_state(AppState::MainMenu)));
    }
}

fn spawn_home(mut commands: Commands, shell: Res<ShellContent>) {
    let root = spawn_screen_root(&mut commands, &shell, "Home");
    commands.entity(root).insert((
        HomeRoot,
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
        page.spawn((
            Text::new(keys::HOME_TITLE),
            opts_gui::l10n::L10nKey(keys::HOME_TITLE),
            TextColor::default(),
            TypeStyleToken(TypeScale::Headline),
            ForegroundRole(ColorRole::OnSurface),
        ));
        let new_drive = ButtonBuilder::filled(keys::HOME_NEW_DRIVE)
            .with_key(keys::HOME_NEW_DRIVE)
            .spawn_child(page);
        let load = ButtonBuilder::tonal(keys::HOME_LOAD_GAME)
            .with_key(keys::HOME_LOAD_GAME)
            .spawn_child(page);
        page.commands().entity(new_drive).insert((
            HomeAction::NewDrive,
            DefaultFocus,
            UiId::new("home.new-drive"),
        ));
        page.commands().entity(load).insert((
            NotAvailableYet::new(&HOME_LOAD_GAME),
            UiId::new("home.load-game"),
        ));
    });
}

fn despawn_home(mut commands: Commands, roots: Query<Entity, With<HomeRoot>>) {
    for root in &roots {
        commands.entity(root).despawn();
    }
}

fn home_actions(
    mut activated: MessageReader<Activated>,
    actions: Query<&HomeAction>,
    mut navigate: MessageWriter<Navigate>,
) {
    for Activated(entity) in activated.read() {
        if let Ok(HomeAction::NewDrive) = actions.get(*entity) {
            navigate.write(Navigate::push("drive"));
        }
    }
}
