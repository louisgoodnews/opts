//! Spawning of the dropdown menu, its search field and its no-match notice.

use bevy::prelude::*;
use bevy::ui::OverrideClip;

use super::parts::spawn_menu_items;
use super::search::{ComboboxList, ComboboxNoMatch, ComboboxSearch};
use super::systems::menu_wheel_scroll;
use super::{ComboboxMenu, MENU_Z};
use crate::components::text_field::TextFieldBuilder;
use crate::theme::{color::ColorRole, elevation::ElevationLevel, shape::RadiusLevel, typography::TypeScale};
use crate::tokens::components::{BackgroundRole, BorderRole, ElevationToken, ForegroundRole, RadiusToken, TypeStyleToken};

fn spawn_search_field(menu_parent: &mut ChildSpawnerCommands, root: Entity) {
    let field = TextFieldBuilder::outlined("")
        .compact()
        .full_width()
        .spawn_child(menu_parent);
    let mut field = menu_parent.commands_mut().entity(field);
    field.insert((
        ComboboxSearch(root),
        crate::l10n::L10nKey(crate::l10n::keys::COMBOBOX_SEARCH_LABEL),
    ));
}

fn spawn_no_match(menu_parent: &mut ChildSpawnerCommands, root: Entity) {
    menu_parent.spawn((
        (
            Node {
                display: Display::None,
                padding: UiRect::axes(Val::Px(12.0), Val::Px(10.0)),
                ..default()
            },
            Text::new(""),
            TextFont {
                font_size: FontSize::Px(14.0),
                ..default()
            },
            TextColor::default(),
        ),
        crate::l10n::L10nKey(crate::l10n::keys::COMBOBOX_NO_MATCH),
        ComboboxNoMatch(root),
        ForegroundRole(ColorRole::OnSurfaceVariant),
        TypeStyleToken(TypeScale::BodyMedium),
    ));
}

pub(super) fn spawn_dropdown_menu(
    parent: &mut ChildSpawnerCommands,
    root: Entity,
    options: &[String],
    search: bool,
) {
    parent
        .spawn((
            (
                Node {
                    display: Display::None,
                    flex_direction: FlexDirection::Column,
                    position_type: PositionType::Absolute,
                    top: Val::Px(52.0),
                    left: Val::Px(0.0),
                    min_width: Val::Percent(100.0),
                    max_height: Val::Px(if search { 288.0 } else { 240.0 }),
                    border: UiRect::all(Val::Px(1.0)),
                    padding: UiRect::all(Val::Px(4.0)),
                    row_gap: Val::Px(4.0),
                    ..default()
                },
                GlobalZIndex(MENU_Z),
                OverrideClip,
            ),
            ComboboxMenu(root),
            BackgroundRole(ColorRole::SurfaceContainerHigh),
            BorderRole(ColorRole::OutlineVariant),
            RadiusToken(RadiusLevel::ExtraSmall),
            ElevationToken(ElevationLevel::Level2),
        ))
        .observe(menu_wheel_scroll)
        .with_children(|menu_parent| {
            if search {
                spawn_search_field(menu_parent, root);
            }
            menu_parent
                .spawn((
                    Node {
                        display: Display::Flex,
                        flex_direction: FlexDirection::Column,
                        flex_shrink: 1.0,
                        min_height: Val::Px(0.0),
                        overflow: Overflow::scroll_y(),
                        ..default()
                    },
                    ScrollPosition::default(),
                    ComboboxList(root),
                ))
                .with_children(|list| {
                    spawn_menu_items(list, root, options);
                });
            if search {
                spawn_no_match(menu_parent, root);
            }
        });
}
