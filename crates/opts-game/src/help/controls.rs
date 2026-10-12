//! The controls table. Written by hand from the key hints of the HUD until the in-game controls
//! source can generate it (`help.controls-generated`, M-0064); the key names are not translated.

use bevy::prelude::*;
use opts_gui::l10n::L10nKey;
use opts_gui::prelude::*;

use crate::shell::keys;

/// One row: the keys and the message key of what they do.
pub type Row = (&'static str, &'static str);
/// A group: its message key and rows.
pub type Group = (&'static str, &'static [Row]);

/// The controls, grouped.
pub const GROUPS: &[Group] = &[
    (
        keys::HELP_GROUP_DRIVING,
        &[
            ("W / S", keys::HELP_KEY_THROTTLE),
            ("← / →", keys::HELP_KEY_STEER),
            (".", keys::HELP_KEY_PARKING),
            ("D / N / R", keys::HELP_KEY_GEARS),
            ("Z / X / C", keys::HELP_KEY_TRAM),
        ],
    ),
    (
        keys::HELP_GROUP_CAMERA,
        &[
            ("1 – 5", keys::HELP_KEY_CAMERA),
            ("0", keys::HELP_KEY_CAMERA_PREV),
            ("Shift + Space", keys::HELP_KEY_CAMERA_RESET),
        ],
    ),
    (
        keys::HELP_GROUP_MENUS,
        &[
            ("Esc", keys::HELP_KEY_PAUSE_MENU),
            ("P", keys::HELP_KEY_SIM_PAUSE),
            ("Alt", keys::HELP_KEY_VEHICLE_MENU),
            ("F1", keys::HELP_KEY_HELP),
            ("F2", keys::HELP_KEY_THEME),
            ("F10", keys::HELP_KEY_DEV),
        ],
    ),
];

/// Spawns the table into `parent`; `with_marker` adds the red *not available yet* line for the
/// generated list.
pub fn spawn_controls(parent: &mut ChildSpawnerCommands, with_marker: bool) {
    for (group, rows) in GROUPS {
        parent.spawn((
            Text::new(*group),
            L10nKey(group),
            TextColor::default(),
            TypeStyleToken(TypeScale::TitleMedium),
            ForegroundRole(ColorRole::OnSurface),
        ));
        for (keys_text, action) in *rows {
            parent
                .spawn(Node {
                    column_gap: Val::Px(16.0),
                    align_items: AlignItems::Center,
                    ..default()
                })
                .with_children(|row| {
                    row.spawn((
                        Node {
                            min_width: Val::Px(120.0),
                            padding: UiRect::axes(Val::Px(8.0), Val::Px(2.0)),
                            ..default()
                        },
                        BackgroundRole(ColorRole::SurfaceContainerHighest),
                        RadiusToken(RadiusLevel::Small),
                    ))
                    .with_children(|chip| {
                        chip.spawn((
                            Text::new(*keys_text),
                            TextColor::default(),
                            TypeStyleToken(TypeScale::Label),
                            ForegroundRole(ColorRole::OnSurface),
                        ));
                    });
                    row.spawn((
                        Text::new(*action),
                        L10nKey(action),
                        TextColor::default(),
                        TypeStyleToken(TypeScale::BodyMedium),
                        ForegroundRole(ColorRole::OnSurfaceVariant),
                    ));
                });
        }
    }
    if with_marker {
        opts_gui::not_yet::spawn_label(parent);
    }
}
