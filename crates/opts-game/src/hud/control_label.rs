//! The name of the cockpit control under the cursor (M-0141), a small label at the bottom centre
//! of the driver's and passenger views. It shows the script's trigger name (`cp_kluczyk_mov`):
//! OMSI gives hot-spots no display names, and the name is what `vehicle.click` and the
//! controls report use.

use bevy::prelude::*;
use opts_bevy::HoveredControl;
use opts_gui::prelude::*;
use opts_gui::theme::{InstrumentPalette, InstrumentVariant};

use super::parts::{panel, texts};
use crate::in_game::instruments::InstrumentInk;
use crate::in_game::types::InGameEntity;

/// The row that centres the label.
#[derive(Component, Debug, Clone, Copy)]
pub struct ControlLabel;

/// The label's text.
#[derive(Component, Debug, Clone, Copy)]
pub struct ControlLabelText;

/// Distance of the label from the bottom edge, clear of the HUD's bottom clusters.
const BOTTOM: f32 = 96.0;

fn spawn_label(commands: &mut Commands, name: &str, palette: &InstrumentPalette) {
    commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                left: Val::Px(0.0),
                right: Val::Px(0.0),
                bottom: Val::Px(BOTTOM),
                justify_content: JustifyContent::Center,
                ..default()
            },
            Pickable::IGNORE,
            GlobalZIndex(50),
            ControlLabel,
            InGameEntity,
            UiId::new("hud.control-label"),
        ))
        .with_children(|row| {
            row.spawn(panel(
                Node {
                    padding: UiRect::axes(Val::Px(14.0), Val::Px(6.0)),
                    border: UiRect::all(Val::Px(1.0)),
                    ..default()
                },
                RadiusLevel::Large,
                palette,
            ))
            .with_children(|p| {
                p.spawn((
                    texts(name, TypeScale::BodyMedium, InstrumentInk::Text, palette),
                    ControlLabelText,
                    UiId::new("hud.control-label.text"),
                ));
            });
        });
}

/// Shows the label while a control is under the cursor, updates its text, removes it otherwise.
pub fn sync_label(
    mut commands: Commands,
    hovered: Res<HoveredControl>,
    theme: Res<MaterialTheme>,
    variant: Res<InstrumentVariant>,
    labels: Query<Entity, With<ControlLabel>>,
    mut texts: Query<&mut Text, With<ControlLabelText>>,
) {
    let Some(name) = hovered.0.as_deref() else {
        for e in &labels {
            commands.entity(e).despawn();
        }
        return;
    };
    if labels.is_empty() {
        spawn_label(&mut commands, name, theme.instrument(*variant));
    }
    for mut text in &mut texts {
        if text.0 != name {
            text.0 = name.to_owned();
        }
    }
}
