//! The top bar of the HUD: the status bar (it fades) and the pause button.

use super::*;

/// The top row: the status bar (fades) and the pause button.
pub(super) fn spawn_top_bar(
    commands: &mut Commands,
    profile: &HudProfile,
    palette: &InstrumentPalette,
) -> Entity {
    let in_slot = |slot| profile.in_slot(slot);
    commands
        .spawn(Node {
            width: Val::Percent(100.0),
            flex_direction: FlexDirection::Row,
            justify_content: JustifyContent::SpaceBetween,
            align_items: AlignItems::FlexStart,
            ..default()
        })
        .with_children(|bar| {
            let status = in_slot(Slot::StatusBar);
            if !status.is_empty() {
                bar.spawn((
                    panel(
                        panel_node(
                            UiRect::axes(Val::Px(16.0), Val::Px(10.0)),
                            FlexDirection::Column,
                            4.0,
                        ),
                        RadiusLevel::Medium,
                        palette,
                    ),
                    HudSlot(Slot::StatusBar),
                    FadeKind::Status,
                    HudFade::shown(STATUS_HOLD, STATUS_FADE),
                    UiId::new("hud.slot.status-bar"),
                ))
                .with_children(|pill| {
                    for (_, w) in &status {
                        text_widget(pill, w, TypeScale::BodyMedium, InstrumentInk::Text, palette);
                    }
                });
            } else {
                bar.spawn(Node::default());
            }
            // The pause button is chrome, not a widget: pointer only, never focused.
            bar.spawn((
                (
                    Button,
                    panel_node(
                        UiRect::axes(Val::Px(16.0), Val::Px(8.0)),
                        FlexDirection::Row,
                        0.0,
                    ),
                    BorderColor::all(palette.panel_border),
                    BackgroundColor(palette.panel),
                ),
                M3Button::default(),
                PauseMenuBtn,
                PointerOnly,
                InstrumentPanel,
                RadiusToken(RadiusLevel::Medium),
                UiId::new("hud.pause"),
            ))
            .with_children(|btn| {
                btn.spawn((
                    texts(
                        keys::PAUSE_BUTTON,
                        TypeScale::Label,
                        InstrumentInk::Text,
                        palette,
                    ),
                    opts_gui::l10n::L10nKey(keys::PAUSE_BUTTON),
                ));
            });
        })
        .id()
}
