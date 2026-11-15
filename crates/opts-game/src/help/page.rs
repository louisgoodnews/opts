//! The Help page's shell: its tabs on the left and the scrolling pane, spawned on entry and
//! despawned on exit.

use bevy::prelude::*;
use opts_gui::interaction::focus::{FocusRegion, Landmark};
use opts_gui::l10n::L10nKey;
use opts_gui::prelude::*;

use crate::shell::nav::PendingDeepLink;
use crate::shell::{ShellContent, keys, spawn_screen_root};

use super::scroll;
use super::{HelpPane, HelpRoot, HelpState, HelpTab};

pub(super) fn spawn_help(
    mut commands: Commands,
    shell: Res<ShellContent>,
    mut deep: ResMut<PendingDeepLink>,
    mut state: ResMut<HelpState>,
) {
    if let Some(tab) = deep.0.take().as_deref().and_then(HelpTab::parse) {
        state.tab = tab;
    }
    let root = spawn_screen_root(&mut commands, &shell, "Help");
    commands.entity(root).insert((
        HelpRoot,
        Node {
            width: Val::Percent(100.0),
            height: Val::Percent(100.0),
            flex_direction: FlexDirection::Row,
            padding: UiRect::all(Val::Px(24.0)),
            column_gap: Val::Px(24.0),
            ..default()
        },
    ));
    commands.entity(root).with_children(|page| {
        page.spawn((
            Node {
                width: Val::Px(220.0),
                flex_shrink: 0.0,
                flex_direction: FlexDirection::Column,
                row_gap: Val::Px(4.0),
                ..default()
            },
            FocusRegion(Landmark::Detail),
        ))
        .with_children(|tabs| {
            tabs.spawn((
                Text::new(keys::HELP_TITLE),
                L10nKey(keys::HELP_TITLE),
                TextColor::default(),
                TypeStyleToken(TypeScale::Headline),
                ForegroundRole(ColorRole::OnSurface),
                Node {
                    margin: UiRect::bottom(Val::Px(12.0)),
                    ..default()
                },
            ));
            for tab in HelpTab::ALL {
                let button = ButtonBuilder::text(tab.key())
                    .with_key(tab.key())
                    .spawn_child(tabs);
                tabs.commands().entity(button).insert((
                    tab,
                    UiId::new(format!("help.tab.{}", tab.id())),
                    Node {
                        min_height: Val::Px(44.0),
                        padding: UiRect::axes(Val::Px(16.0), Val::Px(8.0)),
                        justify_content: JustifyContent::FlexStart,
                        align_items: AlignItems::Center,
                        ..default()
                    },
                ));
            }
        });
        page.spawn((
            Node {
                flex_grow: 1.0,
                min_width: Val::Px(0.0),
                flex_direction: FlexDirection::Column,
                row_gap: Val::Px(8.0),
                overflow: Overflow::scroll_y(),
                ..default()
            },
            HelpPane,
            scroll::HelpScroll,
        ));
    });
}

pub(super) fn despawn_help(mut commands: Commands, roots: Query<Entity, With<HelpRoot>>) {
    for root in &roots {
        commands.entity(root).despawn();
    }
}
