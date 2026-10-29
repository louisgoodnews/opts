//! The cue of the mouse driving mode (M-0143): thin lines at the middle of the window (the
//! thresholds between throttle and brake, and between left and right) and a line of text saying
//! how to leave it or to take the mouse back.

use bevy::prelude::*;
use opts_bevy::MouseDriving;
use opts_gui::l10n::L10nKey;
use opts_gui::prelude::*;
use opts_gui::theme::{InstrumentPalette, InstrumentVariant};

use super::keys;
use super::parts::{panel, texts};
use crate::in_game::instruments::InstrumentInk;
use crate::in_game::types::InGameEntity;

/// The root of the cue; `freed` is the state it was built for.
#[derive(Component, Debug, Clone, Copy)]
pub struct MouseDrivingCue {
    freed: bool,
}

/// The two thin lines through the middle of the screen (the mouse drives).
fn crosshair(root: &mut ChildSpawnerCommands, line: Color) {
    root.spawn((
        Node {
            position_type: PositionType::Absolute,
            left: Val::Px(0.0),
            right: Val::Px(0.0),
            top: Val::Percent(50.0),
            height: Val::Px(1.0),
            ..default()
        },
        BackgroundColor(line),
        Pickable::IGNORE,
    ));
    root.spawn((
        Node {
            position_type: PositionType::Absolute,
            top: Val::Px(0.0),
            bottom: Val::Px(0.0),
            left: Val::Percent(50.0),
            width: Val::Px(1.0),
            ..default()
        },
        BackgroundColor(line),
        Pickable::IGNORE,
    ));
}

/// The reminder panel near the bottom edge.
fn hint(root: &mut ChildSpawnerCommands, key: &'static str, palette: &InstrumentPalette) {
    root.spawn((
        Node {
            position_type: PositionType::Absolute,
            left: Val::Px(0.0),
            right: Val::Px(0.0),
            bottom: Val::Px(150.0),
            justify_content: JustifyContent::Center,
            ..default()
        },
        Pickable::IGNORE,
    ))
    .with_children(|row| {
        row.spawn((
            panel(
                Node {
                    padding: UiRect::axes(Val::Px(14.0), Val::Px(6.0)),
                    border: UiRect::all(Val::Px(1.0)),
                    ..default()
                },
                RadiusLevel::Large,
                palette,
            ),
            Pickable::IGNORE,
        ))
        .with_children(|p| {
            p.spawn((
                texts(key, TypeScale::BodyMedium, InstrumentInk::Text, palette),
                L10nKey(key),
            ));
        });
    });
}

fn spawn_cue(commands: &mut Commands, freed: bool, palette: &InstrumentPalette) {
    let line = palette.text_dim.with_alpha(0.4);
    let key = if freed {
        keys::MOUSE_DRIVING_FREE
    } else {
        keys::MOUSE_DRIVING_ON
    };
    commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                left: Val::Px(0.0),
                right: Val::Px(0.0),
                top: Val::Px(0.0),
                bottom: Val::Px(0.0),
                ..default()
            },
            Pickable::IGNORE,
            GlobalZIndex(45),
            MouseDrivingCue { freed },
            InGameEntity,
            UiId::new("hud.mouse-driving"),
        ))
        .with_children(|root| {
            if !freed {
                crosshair(root, line);
            }
            hint(root, key, palette);
        });
}

/// Shows the cue while the mode is on: with the lines while the mouse drives, as a reminder only
/// while it is freed.
pub fn sync_cue(
    mut commands: Commands,
    state: Res<MouseDriving>,
    theme: Res<MaterialTheme>,
    variant: Res<InstrumentVariant>,
    cues: Query<(Entity, &MouseDrivingCue)>,
) {
    let want = (state.enabled && state.on).then_some(state.freed);
    let have = cues.iter().next().map(|(e, c)| (e, c.freed));
    match (want, have) {
        (Some(w), Some((_, h))) if w == h => {}
        (Some(w), have) => {
            if let Some((e, _)) = have {
                commands.entity(e).despawn();
            }
            spawn_cue(&mut commands, w, theme.instrument(*variant));
        }
        (None, Some((e, _))) => commands.entity(e).despawn(),
        (None, None) => {}
    }
}
