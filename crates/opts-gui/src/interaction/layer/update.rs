//! Per-frame update of the state layer tint, the disabled scrim and the helper radii.

use bevy::input_focus::InputFocusVisible;
use bevy::prelude::*;

use crate::interaction::state::{
    Disabled, ErrorState, Focused, Hovered, Pressed, UiState, DISABLED_CONTENT,
};
use crate::theme::{color::ColorRole, theme::MaterialTheme};
use crate::tokens::components::BackgroundRole;

use super::{on_color, FocusProxy, ForcedState, StateLayerLink, StateLayerRole};

/// The state of one control as the layer system sees it.
fn ui_state(
    hovered: Option<&Hovered>,
    pressed: bool,
    focused: bool,
    disabled: bool,
    error: bool,
    ring_visible: bool,
) -> UiState {
    UiState {
        hovered: hovered.is_some_and(|h| h.0),
        focused: focused && ring_visible,
        pressed,
        selected: false,
        disabled,
        error,
    }
}

/// Query item of [`update_state_layers`].
type ControlItem<'a> = (
    &'a StateLayerLink,
    Option<&'a Hovered>,
    Has<Pressed>,
    Has<Focused>,
    Has<Disabled>,
    Has<ErrorState>,
    Option<&'a StateLayerRole>,
    Option<&'a BackgroundRole>,
    &'a Node,
    Option<&'a FocusProxy>,
    Option<&'a ForcedState>,
);

/// Applies the state layer tint, the disabled scrim and the border radius to the helper nodes.
pub fn update_state_layers(
    theme: Res<MaterialTheme>,
    visible: Option<Res<InputFocusVisible>>,
    controls: Query<ControlItem>,
    focus_holders: Query<(), With<Focused>>,
    mut nodes: Query<(&mut BackgroundColor, &mut Node), Without<StateLayerLink>>,
) {
    let ring_visible = visible.is_none_or(|v| v.0);
    for (
        link,
        hovered,
        pressed,
        focused,
        disabled,
        error,
        role,
        bg_role,
        control_node,
        proxy,
        forced,
    ) in &controls
    {
        let proxied = proxy.is_some_and(|p| focus_holders.contains(p.0));
        let forced = forced.copied().unwrap_or_default();
        let mut state = ui_state(
            hovered,
            pressed || forced.pressed,
            focused || proxied || forced.focused,
            disabled,
            error,
            ring_visible || proxy.is_some() || forced.focused,
        );
        state.hovered |= forced.hovered;
        let tint_role = role
            .map(|r| r.0)
            .unwrap_or_else(|| bg_role.map_or(ColorRole::OnSurface, |b| on_color(b.0)));
        let tint = theme.scheme.get(tint_role).with_alpha(state.layer_alpha());
        let scrim = if disabled {
            theme
                .scheme
                .get(ColorRole::Surface)
                .with_alpha(1.0 - DISABLED_CONTENT)
        } else {
            Color::NONE
        };
        for (entity, color) in [(link.layer, tint), (link.scrim, scrim)] {
            if let Ok((mut bg, mut node)) = nodes.get_mut(entity) {
                if bg.0 != color {
                    bg.0 = color;
                }
                if node.border_radius != control_node.border_radius {
                    node.border_radius = control_node.border_radius;
                }
            }
        }
    }
}
