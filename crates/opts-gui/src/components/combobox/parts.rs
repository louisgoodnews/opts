//! Spawning of a combobox header button and of its menu items.

use bevy::prelude::*;

use super::{ComboboxHeader, ComboboxLabelText, ComboboxMenuItem};
use crate::components::Button;
use crate::interaction::state::Disabled;
use crate::theme::{color::ColorRole, shape::RadiusLevel, typography::TypeScale};
use crate::tokens::components::{
    BackgroundRole, BorderRole, ForegroundRole, RadiusToken, TypeStyleToken,
};

pub(super) fn spawn_header(
    parent: &mut ChildSpawnerCommands,
    root: Entity,
    display_text: &str,
    disabled: bool,
) {
    let mut header = parent.spawn((
        (
            Button,
            Node {
                display: Display::Flex,
                flex_direction: FlexDirection::Row,
                justify_content: JustifyContent::SpaceBetween,
                align_items: AlignItems::Center,
                padding: UiRect::axes(Val::Px(16.0), Val::Px(12.0)),
                min_height: Val::Px(48.0),
                border: UiRect::all(Val::Px(1.0)),
                ..default()
            },
        ),
        ComboboxHeader(root),
        BackgroundRole(ColorRole::Surface),
        BorderRole(ColorRole::Outline),
        RadiusToken(RadiusLevel::ExtraSmall),
    ));
    if disabled {
        header.insert(Disabled);
    }
    header.with_children(|header_parent| {
        header_parent.spawn((
            (
                Text::new(display_text),
                TextFont {
                    font_size: FontSize::Px(14.0),
                    ..default()
                },
                TextColor::default(),
            ),
            ComboboxLabelText(root),
            ForegroundRole(ColorRole::OnSurface),
            TypeStyleToken(TypeScale::BodyMedium),
        ));

        header_parent.spawn((
            (
                Text::new("▼"),
                TextFont {
                    font_size: FontSize::Px(12.0),
                    ..default()
                },
                TextColor::default(),
            ),
            ForegroundRole(ColorRole::OnSurfaceVariant),
            TypeStyleToken(TypeScale::Label),
            crate::ui_tree::A11yHidden,
        ));
    });
}

pub(super) fn spawn_menu_items(list: &mut ChildSpawnerCommands, root: Entity, options: &[String]) {
    for (i, opt) in options.iter().enumerate() {
        list.spawn((
            (
                Button,
                Node {
                    padding: UiRect::axes(Val::Px(12.0), Val::Px(10.0)),
                    width: Val::Percent(100.0),
                    flex_shrink: 0.0,
                    display: Display::Flex,
                    align_items: AlignItems::Center,
                    ..default()
                },
                BackgroundColor(Color::NONE),
            ),
            ComboboxMenuItem {
                combobox_entity: root,
                index: i,
            },
            BackgroundRole(ColorRole::SurfaceContainerHigh),
            RadiusToken(RadiusLevel::ExtraSmall),
        ))
        .with_children(|item_parent| {
            item_parent.spawn((
                (
                    Text::new(opt.clone()),
                    TextFont {
                        font_size: FontSize::Px(14.0),
                        ..default()
                    },
                    TextColor::default(),
                ),
                ForegroundRole(ColorRole::OnSurface),
                TypeStyleToken(TypeScale::BodyMedium),
            ));
        });
    }
}
