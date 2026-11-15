//! The camera label: below the top bar, centred, hidden until the camera changes.

use super::*;

/// The camera label: below the top bar, centred, hidden until the camera changes.
pub(super) fn spawn_camera_label(
    commands: &mut Commands,
    profile: &HudProfile,
    palette: &InstrumentPalette,
) -> Option<Entity> {
    let in_slot = |slot| profile.in_slot(slot);
    let camera = in_slot(Slot::CameraLabel);
    (!camera.is_empty()).then(|| {
        commands
            .spawn((
                Node {
                    position_type: PositionType::Absolute,
                    top: Val::Px(CAMERA_LABEL_MIN_TOP),
                    left: Val::Px(0.0),
                    width: Val::Percent(100.0),
                    justify_content: JustifyContent::Center,
                    ..default()
                },
                CameraLabelRow,
            ))
            .with_children(|row| {
                row.spawn((
                    panel(
                        panel_node(
                            UiRect::axes(Val::Px(16.0), Val::Px(8.0)),
                            FlexDirection::Row,
                            0.0,
                        ),
                        RadiusLevel::Full,
                        palette,
                    ),
                    HudSlot(Slot::CameraLabel),
                    FadeKind::CameraLabel,
                    HudFade::hidden(CAMERA_LABEL_SECONDS - CAMERA_LABEL_FADE, CAMERA_LABEL_FADE),
                    Visibility::Hidden,
                    UiId::new("hud.slot.camera-label"),
                ))
                .with_children(|pill| {
                    for (_, w) in &camera {
                        text_widget(pill, w, TypeScale::BodyMedium, InstrumentInk::Text, palette);
                    }
                });
            })
            .id()
    })
}
