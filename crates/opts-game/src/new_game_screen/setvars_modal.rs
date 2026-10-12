//! Modal dialog for configuring vehicle setvars (equipment, visual options, and livery scripts).

use bevy::prelude::*;
use opts_gui::components::Button;
use opts_gui::prelude::*;

use crate::menu::glass::GlassSurface;
use crate::new_game_screen::types::{
    CloseSetvarsModalBtn, GameCatalogue, NewGameSetup, OpenSetvarsModalBtn, ResetSetvarsBtn,
    SetvarStepBtn, SetvarsModalRoot, VehicleItem,
};

pub use crate::new_game_screen::setvars_discovery::{get_vehicle_setvars, SetvarItem};

/// Spawns the Setvars Configuration Modal dialog overlay.
pub fn spawn_setvars_modal(
    commands: &mut Commands,
    theme: &MaterialTheme,
    setup: &NewGameSetup,
    vehicle: &VehicleItem,
) {
    let setvars = get_vehicle_setvars(vehicle, &setup.custom_setvars);

    commands
        .spawn((
            (Node {
                    position_type: PositionType::Absolute,
                    left: Val::Px(0.0),
                    top: Val::Px(0.0),
                    width: Val::Percent(100.0),
                    height: Val::Percent(100.0),
                    align_items: AlignItems::Center,
                    justify_content: JustifyContent::Center,
                    padding: UiRect::all(Val::Px(24.0)),
                    ..default()
                }, BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.75)), GlobalZIndex(100)),
            SetvarsModalRoot,
        ))
        .with_children(|backdrop| {
            backdrop
                .spawn((
                    (Node {
                            width: Val::Px(580.0),
                            max_height: Val::Px(520.0),
                            flex_direction: FlexDirection::Column,
                            row_gap: Val::Px(16.0),
                            padding: UiRect::all(Val::Px(24.0)),
                            border: UiRect::all(Val::Px(1.5)),
                            overflow: Overflow::clip_y(),
                            ..default()
                        }, BorderColor::all(theme.scheme.get(ColorRole::Primary)), BackgroundColor(theme.scheme.get(ColorRole::SurfaceContainerHigh))),
                    GlassSurface::new(ColorRole::SurfaceContainerHigh, 0.95),
                    RadiusToken(opts_gui::theme::shape::RadiusLevel::Large),
                ))
                .with_children(|dialog| {
                    dialog.spawn((
                        (Text::new("Bus Setvars & Equipment Configuration"), TextFont { font_size: FontSize::Px(20.0), ..default() }, TextColor(theme.scheme.get(ColorRole::Primary))),
                        TypeStyleToken(TypeScale::TitleLarge),
                    ));

                    dialog.spawn((
                        (Text::new(format!("Vehicle: {} · Livery: {}", vehicle.full_name, setup.selected_livery)), TextFont { font_size: FontSize::Px(12.0), ..default() }, TextColor(theme.scheme.get(ColorRole::OnSurfaceVariant))),
                        TypeStyleToken(TypeScale::Label),
                    ));

                    if setvars.is_empty() {
                        dialog
                            .spawn((Node {
                                    width: Val::Percent(100.0),
                                    padding: UiRect::all(Val::Px(18.0)),
                                    border: UiRect::all(Val::Px(1.0)),
                                    flex_direction: FlexDirection::Column,
                                    row_gap: Val::Px(8.0),
                                    ..default()
                                }, BorderColor::all(theme.scheme.get(ColorRole::OutlineVariant)), BackgroundColor(theme.scheme.get(ColorRole::SurfaceContainerLowest))))
                            .with_children(|box_node| {
                                box_node.spawn((
                                    (Text::new("ℹ Vehicle Configuration Not Available"), TextFont { font_size: FontSize::Px(15.0), ..default() }, TextColor(theme.scheme.get(ColorRole::Secondary))),
                                    TypeStyleToken(TypeScale::TitleMedium),
                                ));

                                box_node.spawn((
                                    (Text::new("In this menu, you can configure setvars (equipment, visual accessories, and script options) for the vehicle. This vehicle model does not declare any configurable setvars."), TextFont { font_size: FontSize::Px(13.0), ..default() }, TextColor(theme.scheme.get(ColorRole::OnSurface))),
                                    TypeStyleToken(TypeScale::BodyMedium),
                                ));
                            });
                    } else {
                        dialog
                            .spawn(Node {
                                    width: Val::Percent(100.0),
                                    flex_direction: FlexDirection::Column,
                                    row_gap: Val::Px(10.0),
                                    overflow: Overflow::clip_y(),
                                    ..default()
                                })
                            .with_children(|list| {
                                for item in &setvars {
                                    spawn_setvar_row(list, theme, item);
                                }
                            });
                    }

                    // Footer Buttons
                    dialog
                        .spawn((Node {
                                width: Val::Percent(100.0),
                                flex_direction: FlexDirection::Row,
                                justify_content: JustifyContent::SpaceBetween,
                                align_items: AlignItems::Center,
                                padding: UiRect::top(Val::Px(12.0)),
                                border: UiRect { top: Val::Px(1.0), ..default() },
                                ..default()
                            }, BorderColor::all(theme.scheme.get(ColorRole::OutlineVariant))))
                        .with_children(|footer| {
                            footer
                                .spawn((
                                    (Button, Node {
                                            padding: UiRect::axes(Val::Px(12.0), Val::Px(6.0)),
                                            border: UiRect::all(Val::Px(1.0)),
                                            ..default()
                                        }, BorderColor::all(theme.scheme.get(ColorRole::OutlineVariant)), BackgroundColor(theme.scheme.get(ColorRole::SurfaceContainerLowest))),
                                    M3Button::default(),
                                    ResetSetvarsBtn,
                                    RadiusToken(opts_gui::theme::shape::RadiusLevel::Small),
                                    Focusable { tab_index: 0 },
                                ))
                                .with_children(|btn| {
                                    btn.spawn((
                                        (Text::new("↺ Reset Defaults"), TextFont { font_size: FontSize::Px(12.0), ..default() }, TextColor(theme.scheme.get(ColorRole::OnSurface))),
                                        TypeStyleToken(TypeScale::Label),
                                    ));
                                });

                            footer
                                .spawn((
                                    (Button, Node {
                                            padding: UiRect::axes(Val::Px(16.0), Val::Px(8.0)),
                                            border: UiRect::all(Val::Px(1.0)),
                                            ..default()
                                        }, BorderColor::all(theme.scheme.get(ColorRole::Primary)), BackgroundColor(theme.scheme.get(ColorRole::Primary))),
                                    M3Button::default(),
                                    CloseSetvarsModalBtn,
                                    RadiusToken(opts_gui::theme::shape::RadiusLevel::Small),
                                    Focusable { tab_index: 0 },
                                ))
                                .with_children(|btn| {
                                    btn.spawn((
                                        (Text::new("Apply & Close"), TextFont { font_size: FontSize::Px(13.0), ..default() }, TextColor(theme.scheme.get(ColorRole::OnPrimary))),
                                        TypeStyleToken(TypeScale::BodyMedium),
                                    ));
                                });
                        });
                });
        });
}

fn spawn_setvar_row(parent: &mut ChildSpawnerCommands, theme: &MaterialTheme, item: &SetvarItem) {
    parent
        .spawn((
            Node {
                width: Val::Percent(100.0),
                padding: UiRect::axes(Val::Px(12.0), Val::Px(8.0)),
                border: UiRect::all(Val::Px(1.0)),
                flex_direction: FlexDirection::Row,
                align_items: AlignItems::Center,
                justify_content: JustifyContent::SpaceBetween,
                ..default()
            },
            BorderColor::all(theme.scheme.get(ColorRole::OutlineVariant)),
            BackgroundColor(theme.scheme.get(ColorRole::SurfaceContainerLow)),
        ))
        .with_children(|row| {
            row.spawn(Node {
                flex_direction: FlexDirection::Column,
                row_gap: Val::Px(2.0),
                max_width: Val::Px(360.0),
                ..default()
            })
            .with_children(|col| {
                col.spawn((
                    (
                        Text::new(format!("{} [{}]", item.label, item.name)),
                        TextFont {
                            font_size: FontSize::Px(13.0),
                            ..default()
                        },
                        TextColor(theme.scheme.get(ColorRole::OnSurface)),
                    ),
                    TypeStyleToken(TypeScale::BodyMedium),
                ));

                col.spawn((
                    (
                        Text::new(&item.description),
                        TextFont {
                            font_size: FontSize::Px(11.0),
                            ..default()
                        },
                        TextColor(theme.scheme.get(ColorRole::Outline)),
                    ),
                    TypeStyleToken(TypeScale::Label),
                ));
            });

            row.spawn(Node {
                flex_direction: FlexDirection::Row,
                align_items: AlignItems::Center,
                column_gap: Val::Px(6.0),
                ..default()
            })
            .with_children(|ctrl| {
                spawn_stepper_btn(ctrl, theme, "-", item.name.clone(), -1);

                ctrl.spawn((
                    (
                        Text::new(format!("{}", item.value)),
                        TextFont {
                            font_size: FontSize::Px(13.0),
                            ..default()
                        },
                        TextColor(theme.scheme.get(ColorRole::Primary)),
                    ),
                    TypeStyleToken(TypeScale::BodyMedium),
                ));

                spawn_stepper_btn(ctrl, theme, "+", item.name.clone(), 1);
            });
        });
}

fn spawn_stepper_btn(
    parent: &mut ChildSpawnerCommands,
    theme: &MaterialTheme,
    label: &str,
    var_name: String,
    delta: i32,
) {
    parent
        .spawn((
            (
                Button,
                Node {
                    width: Val::Px(28.0),
                    height: Val::Px(28.0),
                    align_items: AlignItems::Center,
                    justify_content: JustifyContent::Center,
                    border: UiRect::all(Val::Px(1.0)),
                    ..default()
                },
                BorderColor::all(theme.scheme.get(ColorRole::OutlineVariant)),
                BackgroundColor(theme.scheme.get(ColorRole::SurfaceContainerHigh)),
            ),
            M3Button::default(),
            SetvarStepBtn { var_name, delta },
            Focusable { tab_index: 0 },
        ))
        .with_children(|btn| {
            btn.spawn((
                (
                    Text::new(label),
                    TextFont {
                        font_size: FontSize::Px(14.0),
                        ..default()
                    },
                    TextColor(theme.scheme.get(ColorRole::Primary)),
                ),
                TypeStyleToken(TypeScale::BodyMedium),
            ));
        });
}

/// Handles opening, closing, stepping, and resetting the setvars configuration modal.
#[allow(clippy::too_many_arguments)]
pub fn handle_setvars_modal_interactions(
    mut commands: Commands,
    mut events: MessageReader<ButtonClicked>,
    open_query: Query<&OpenSetvarsModalBtn>,
    close_query: Query<&CloseSetvarsModalBtn>,
    step_query: Query<&SetvarStepBtn>,
    reset_query: Query<&ResetSetvarsBtn>,
    modal_query: Query<Entity, With<SetvarsModalRoot>>,
    theme: Res<MaterialTheme>,
    catalogue: Res<GameCatalogue>,
    mut setup: ResMut<NewGameSetup>,
) {
    let mut should_reopen = false;
    for event in events.read() {
        if open_query.get(event.0).is_ok() {
            setup.setvars_modal_open = true;
            should_reopen = true;
        } else if close_query.get(event.0).is_ok() {
            setup.setvars_modal_open = false;
            for ent in &modal_query {
                commands.entity(ent).despawn();
            }
        } else if let Ok(step) = step_query.get(event.0) {
            let current = *setup.custom_setvars.get(&step.var_name).unwrap_or(&0);
            let next_val = (current + step.delta).clamp(0, 1);
            setup.custom_setvars.insert(step.var_name.clone(), next_val);
            should_reopen = true;
        } else if reset_query.get(event.0).is_ok() {
            setup.custom_setvars.clear();
            should_reopen = true;
        }
    }

    if should_reopen && setup.setvars_modal_open {
        for ent in &modal_query {
            commands.entity(ent).despawn();
        }
        let current_veh = catalogue
            .vehicles
            .iter()
            .find(|v| {
                v.model == setup.selected_vehicle
                    || v.full_name == setup.selected_vehicle
                    || v.id == setup.selected_vehicle
            })
            .unwrap_or_else(|| &catalogue.vehicles[0]);

        spawn_setvars_modal(&mut commands, &theme, &setup, current_veh);
    }
}
