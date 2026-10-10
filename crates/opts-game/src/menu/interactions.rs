//! Interaction systems for button hover/pressed effects, focus indicators, and keyboard activation.

use bevy::prelude::*;
use opts_gui::{
    components::button::OnClickCallback,
    interaction::{
        events::ButtonClicked,
        focus::Focusable,
        state::{Disabled, Focused},
    },
    theme::{color::ColorRole, theme::MaterialTheme},
    tokens::components::BorderRole,
};

use crate::menu::glass::GlassSurface;

/// Auxiliary component storing the original border configuration for restoration after focus loss.
#[derive(Component, Clone, Copy)]
pub struct OriginalBorderStyle {
    pub border: UiRect,
}

/// System that renders a high-visibility 2px focus ring around focused UI elements.
#[allow(clippy::type_complexity)]
pub fn menu_focus_visual_system(
    theme: Res<MaterialTheme>,
    mut commands: Commands,
    mut query: Query<
        (
            Entity,
            &mut Node,
            &mut BorderColor,
            Has<Focused>,
            Option<&OriginalBorderStyle>,
            Option<&BorderRole>,
        ),
        (With<Focusable>, Without<Disabled>),
    >,
) {
    for (entity, mut style, mut border_color, is_focused, orig_border, border_role) in &mut query {
        if is_focused {
            if orig_border.is_none() {
                commands.entity(entity).insert(OriginalBorderStyle {
                    border: style.border,
                });
            }
            style.border = UiRect::all(Val::Px(2.0));
            *border_color = BorderColor::all(theme.scheme.get(ColorRole::Secondary));
        } else if let Some(orig) = orig_border {
            style.border = orig.border;
            if let Some(role) = border_role {
                *border_color = BorderColor::all(theme.scheme.get(role.0));
            } else {
                *border_color = BorderColor::all(Color::NONE);
            }
            commands.entity(entity).remove::<OriginalBorderStyle>();
        }
    }
}

/// System that activates the currently focused button when `Enter` or `Space` is pressed.
#[allow(clippy::type_complexity)]
pub fn menu_keyboard_activation_system(
    keyboard: Res<ButtonInput<KeyCode>>,
    query: Query<(Entity, Option<&OnClickCallback>), (With<Focused>, Without<Disabled>)>,
    mut click_events: MessageWriter<ButtonClicked>,
    mut commands: Commands,
) {
    if keyboard.just_pressed(KeyCode::Enter) || keyboard.just_pressed(KeyCode::Space) {
        for (entity, callback) in &query {
            click_events.write(ButtonClicked(entity));
            if let Some(cb) = callback {
                (cb.0)(&mut commands, entity);
            }
        }
    }
}

/// System providing visual hover lightening and pressed scaling/darkening for interactive surfaces.
#[allow(clippy::type_complexity)]
pub fn menu_interaction_state_system(
    theme: Res<MaterialTheme>,
    mut query: Query<
        (
            &Interaction,
            &mut BackgroundColor,
            Option<&GlassSurface>,
            Option<&opts_gui::tokens::components::BackgroundRole>,
            Option<&mut Transform>,
        ),
        (Changed<Interaction>, Without<Disabled>),
    >,
) {
    for (interaction, mut bg_color, glass, bg_role, transform) in &mut query {
        if let Some(g) = glass {
            let base_color = theme.scheme.get(g.role);
            let base_alpha = g.alpha;

            match *interaction {
                Interaction::Hovered => {
                    let hovered_color =
                        lighten_color(base_color, 0.12).with_alpha((base_alpha + 0.12).min(0.95));
                    bg_color.0 = hovered_color;
                    if let Some(mut xform) = transform {
                        xform.scale = Vec3::splat(1.0);
                    }
                }
                Interaction::Pressed => {
                    let pressed_color = darken_color(base_color, 0.15).with_alpha(base_alpha);
                    bg_color.0 = pressed_color;
                    if let Some(mut xform) = transform {
                        xform.scale = Vec3::splat(0.98);
                    }
                }
                Interaction::None => {
                    bg_color.0 = base_color.with_alpha(base_alpha);
                    if let Some(mut xform) = transform {
                        xform.scale = Vec3::splat(1.0);
                    }
                }
            }
        } else if let Some(role) = bg_role {
            let base_color = theme.scheme.get(role.0);
            match *interaction {
                Interaction::Hovered => {
                    bg_color.0 = lighten_color(base_color, 0.10);
                }
                Interaction::Pressed => {
                    bg_color.0 = darken_color(base_color, 0.15);
                    if let Some(mut xform) = transform {
                        xform.scale = Vec3::splat(0.98);
                    }
                }
                Interaction::None => {
                    bg_color.0 = base_color;
                    if let Some(mut xform) = transform {
                        xform.scale = Vec3::splat(1.0);
                    }
                }
            }
        } else {
            // Text or transparent button: apply subtle hover tint without corrupting rest state
            match *interaction {
                Interaction::Hovered => {
                    bg_color.0 = theme.scheme.get(ColorRole::OnSurface).with_alpha(0.08);
                }
                Interaction::Pressed => {
                    bg_color.0 = theme.scheme.get(ColorRole::OnSurface).with_alpha(0.16);
                    if let Some(mut xform) = transform {
                        xform.scale = Vec3::splat(0.98);
                    }
                }
                Interaction::None => {
                    bg_color.0 = Color::NONE;
                    if let Some(mut xform) = transform {
                        xform.scale = Vec3::splat(1.0);
                    }
                }
            }
        }
    }
}

fn lighten_color(c: Color, amount: f32) -> Color {
    let srgb = c.to_srgba();
    Color::srgba(
        (srgb.red + amount).min(1.0),
        (srgb.green + amount).min(1.0),
        (srgb.blue + amount).min(1.0),
        srgb.alpha,
    )
}

fn darken_color(c: Color, amount: f32) -> Color {
    let srgb = c.to_srgba();
    Color::srgba(
        (srgb.red - amount).max(0.0),
        (srgb.green - amount).max(0.0),
        (srgb.blue - amount).max(0.0),
        srgb.alpha,
    )
}
