//! Making a door's panel and the words of its caption.

use super::*;

pub(super) fn spawn(
    commands: &mut Commands,
    palette: &InstrumentPalette,
    image: Handle<Image>,
    width: f32,
    door: usize,
    y: f32,
) {
    commands
        .spawn((
            panel(
                Node {
                    position_type: PositionType::Absolute,
                    right: Val::Px(20.0),
                    top: Val::Px(y),
                    flex_direction: FlexDirection::Column,
                    row_gap: Val::Px(4.0),
                    padding: UiRect::all(Val::Px(6.0)),
                    border: UiRect::all(Val::Px(1.0)),
                    ..default()
                },
                RadiusLevel::Large,
                palette,
            ),
            DoorOverlay { door, y },
            GlobalZIndex(55),
            InGameEntity,
            UiId::new(format!("hud.door_camera.{}", door + 1)),
        ))
        .with_children(|p| {
            p.spawn((
                Node {
                    width: Val::Px(width),
                    aspect_ratio: Some(16.0 / 9.0),
                    ..default()
                },
                ImageNode::new(image),
                DoorOverlayImage(door),
                UiId::new(format!("hud.door_camera.{}.image", door + 1)),
            ));
            p.spawn((
                texts("", TypeScale::Label, InstrumentInk::TextDim, palette),
                DoorOverlayCaption(door),
                UiId::new(format!("hud.door_camera.{}.caption", door + 1)),
            ));
        });
}

/// The words of a door's name: its number, kind, place along the vehicle and side.
pub(super) fn caption(loc: &Localization, doors: &DoorPoints, door: usize) -> String {
    let Some(d) = doors.0.get(door) else {
        return String::new();
    };
    let zone = match doors.zone(door) {
        Zone::Front => keys::DOOR_FRONT,
        Zone::Middle => keys::DOOR_MIDDLE,
        Zone::Rear => keys::DOOR_REAR,
    };
    loc.t_args(
        keys::DOOR_TITLE,
        &[
            ("number".to_owned(), Arg::from(door as i32 + 1)),
            (
                "kind".to_owned(),
                Arg::from(loc.t(if d.entry {
                    keys::DOOR_ENTRY
                } else {
                    keys::DOOR_EXIT
                })),
            ),
            ("zone".to_owned(), Arg::from(loc.t(zone))),
            (
                "side".to_owned(),
                Arg::from(loc.t(if d.side < 0 {
                    keys::DOOR_LEFT
                } else {
                    keys::DOOR_RIGHT
                })),
            ),
        ],
    )
}
