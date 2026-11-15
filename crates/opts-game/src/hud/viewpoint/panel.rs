//! Spawning the viewpoint editor panel.

use super::*;

/// Spawns the panel.
pub fn spawn_panel(commands: &mut Commands, palette: &InstrumentPalette) {
    commands
        .spawn((
            panel(
                Node {
                    position_type: PositionType::Absolute,
                    right: Val::Px(20.0),
                    top: Val::Px(68.0),
                    width: Val::Px(super::super::layout::VIEWPOINT_PANEL_WIDTH),
                    flex_direction: FlexDirection::Column,
                    row_gap: Val::Px(4.0),
                    padding: UiRect::all(Val::Px(14.0)),
                    border: UiRect::all(Val::Px(1.0)),
                    ..default()
                },
                RadiusLevel::Large,
                palette,
            ),
            ViewpointPanel,
            GlobalZIndex(60),
            InGameEntity,
            UiId::new("hud.viewpoint"),
        ))
        .with_children(|p| {
            p.spawn((
                texts(
                    keys::VIEWPOINT_TITLE,
                    TypeScale::BodyLarge,
                    InstrumentInk::Needle,
                    palette,
                ),
                L10nKey(keys::VIEWPOINT_TITLE),
            ));
            p.spawn((
                texts("", TypeScale::Label, InstrumentInk::TextDim, palette),
                ViewpointSubtitle,
                UiId::new("hud.viewpoint.subtitle"),
            ));
            for (row, name_key) in ROW_NAMES {
                value_row(p, (row, name_key), palette);
            }
            p.spawn((
                Node {
                    justify_content: JustifyContent::FlexStart,
                    ..default()
                },
                ViewpointScopeRow,
            ))
            .with_children(|r| {
                small_button(
                    r,
                    ViewpointButton::Scope,
                    keys::VIEWPOINT_SCOPE_VEHICLE,
                    "hud.viewpoint.scope".to_owned(),
                    palette,
                );
            });
            p.spawn((
                texts(
                    keys::VIEWPOINT_UNSAVED,
                    TypeScale::Label,
                    InstrumentInk::Warning,
                    palette,
                ),
                L10nKey(keys::VIEWPOINT_UNSAVED),
                ViewpointUnsaved,
                Visibility::Hidden,
            ));
            p.spawn((
                texts(
                    keys::VIEWPOINT_HINT,
                    TypeScale::Label,
                    InstrumentInk::TextDim,
                    palette,
                ),
                L10nKey(keys::VIEWPOINT_HINT),
            ));
            action_buttons(p, palette);
        });
}
