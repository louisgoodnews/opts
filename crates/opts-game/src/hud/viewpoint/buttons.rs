//! The viewpoint editor's buttons: the step buttons and the reset, cancel and save row.

use super::*;

pub(super) fn small_button(
    parent: &mut ChildSpawnerCommands,
    action: ViewpointButton,
    key: &'static str,
    id: String,
    palette: &InstrumentPalette,
) {
    parent
        .spawn((
            (
                Button,
                panel_node(
                    UiRect::axes(Val::Px(12.0), Val::Px(6.0)),
                    FlexDirection::Row,
                    0.0,
                ),
                BorderColor::all(palette.panel_border),
                BackgroundColor(palette.panel),
            ),
            M3Button::default(),
            action,
            PointerOnly,
            InstrumentPanel,
            RadiusToken(RadiusLevel::Medium),
            UiId::new(id),
        ))
        .with_children(|b| {
            let mut text = b.spawn((
                texts(key, TypeScale::BodyMedium, InstrumentInk::Text, palette),
                L10nKey(key),
            ));
            if action == ViewpointButton::Scope {
                text.insert(ViewpointScopeText);
            }
        });
}

/// *Reset*, *Cancel* and *Save*.
pub(super) fn action_buttons(p: &mut ChildSpawnerCommands, palette: &InstrumentPalette) {
    p.spawn(Node {
        column_gap: Val::Px(8.0),
        justify_content: JustifyContent::FlexEnd,
        ..default()
    })
    .with_children(|b| {
        small_button(
            b,
            ViewpointButton::Reset,
            keys::VIEWPOINT_RESET,
            "hud.viewpoint.reset".to_owned(),
            palette,
        );
        small_button(
            b,
            ViewpointButton::Cancel,
            keys::VIEWPOINT_CANCEL,
            "hud.viewpoint.cancel".to_owned(),
            palette,
        );
        small_button(
            b,
            ViewpointButton::Save,
            keys::VIEWPOINT_SAVE,
            "hud.viewpoint.save".to_owned(),
            palette,
        );
    });
}
