//! The one state mechanism: hover, focus, pressed, disabled and error treatment for every control.
//!
//! Every entity carrying the [`Button`] marker (the
//! "interactive control" marker of all widgets) gets two helper children and one component:
//!
//! - a *state layer*: an absolute, pick-through node first among the children, tinted with the
//!   control's content colour at 8 % (hover) or 10 % (focus, pressed);
//! - a *scrim*: an absolute node above the content, tinted with the surface colour, visible only
//!   while the control is [`Disabled`] (leaves 38 % of the content; the container fades with it,
//!   a documented simplification of M3's 12 % container);
//! - an [`Outline`] on the control itself as focus ring (2 px, `Secondary` role, drawn outside the
//!   layout) while it has keyboard or gamepad focus.
//!
//! Part A's token sync writes `BackgroundColor`/`BorderColor` of the control; this module never
//! touches those, so the two cannot fight.

use bevy::input_focus::InputFocusVisible;
use bevy::prelude::*;

use crate::interaction::state::{
    Disabled, ErrorState, Focused, Hovered, Pressed, UiState, DISABLED_CONTENT,
};
use crate::theme::{color::ColorRole, theme::MaterialTheme};
use crate::tokens::components::{BackgroundRole, BorderRole};

/// On a control: the entity of its state layer child.
#[derive(Component, Debug, Clone, Copy)]
pub struct StateLayerLink {
    /// The tint node below the content.
    pub layer: Entity,
    /// The disabled scrim above the content.
    pub scrim: Entity,
}

/// Marks the state layer node (below the content).
#[derive(Component, Debug, Clone, Copy)]
pub struct StateLayerNode;

/// Marks the disabled scrim node (above the content).
#[derive(Component, Debug, Clone, Copy)]
pub struct ScrimNode;

/// Overrides the colour role of the state layer (default: the content colour matching the
/// control's [`BackgroundRole`], else `OnSurface`).
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub struct StateLayerRole(pub ColorRole);

/// Remembers the border role a control had before an [`ErrorState`] replaced it (internal).
#[derive(Component, Debug, Clone, Copy)]
pub struct SavedBorderRole(pub Option<ColorRole>);

/// Pins visual states for the component gallery and screenshots: the pinned states are OR-ed with
/// the real ones. Never set by game code.
#[derive(Component, Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct ForcedState {
    /// Draw the hover layer.
    pub hovered: bool,
    /// Draw the focus layer and ring.
    pub focused: bool,
    /// Draw the pressed layer.
    pub pressed: bool,
}

/// Gives a control that is *not* a [`Button`] the state layer, scrim and focus ring (frames of
/// composite controls such as the text field). [`Button`] requires it.
#[derive(Component, Debug, Clone, Copy, Default)]
pub struct StateLayered;

/// The frame of a composite control shows the focus of the entity it points to (the text field's
/// input): the ring and the focus layer are drawn on the frame.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub struct FocusProxy(pub Entity);

/// Width of the focus ring.
pub const FOCUS_RING_WIDTH: f32 = 2.0;
/// Gap between control and focus ring.
pub const FOCUS_RING_OFFSET: f32 = 2.0;

/// Content colour that goes with a container colour (the colour of its state layer).
pub fn on_color(container: ColorRole) -> ColorRole {
    use ColorRole::*;
    match container {
        Primary => OnPrimary,
        Secondary => OnSecondary,
        Tertiary => OnTertiary,
        Error => OnError,
        PrimaryContainer => OnPrimaryContainer,
        SecondaryContainer => OnSecondaryContainer,
        TertiaryContainer => OnTertiaryContainer,
        ErrorContainer => OnErrorContainer,
        InverseSurface => InverseOnSurface,
        _ => OnSurface,
    }
}

fn overlay_node() -> Node {
    Node {
        position_type: PositionType::Absolute,
        left: Val::Px(0.0),
        right: Val::Px(0.0),
        top: Val::Px(0.0),
        bottom: Val::Px(0.0),
        ..default()
    }
}

/// Gives every new control its state layer and scrim.
pub fn spawn_state_layers(
    mut commands: Commands,
    controls: Query<Entity, (Added<StateLayered>, Without<StateLayerLink>)>,
) {
    for control in &controls {
        let layer = commands
            .spawn((
                overlay_node(),
                BackgroundColor(Color::NONE),
                Pickable::IGNORE,
                StateLayerNode,
                Name::new("StateLayer"),
            ))
            .id();
        let scrim = commands
            .spawn((
                overlay_node(),
                BackgroundColor(Color::NONE),
                Pickable::IGNORE,
                ZIndex(1),
                ScrimNode,
                Name::new("DisabledScrim"),
            ))
            .id();
        let link = StateLayerLink { layer, scrim };
        commands
            .entity(control)
            .queue_silenced(move |mut entity: EntityWorldMut| {
                entity.insert(link);
                entity.insert_children(0, &[layer]);
                entity.add_child(scrim);
            });
    }
}

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

/// Query item of [`update_focus_ring`].
type FocusRingItem<'a> = (
    Entity,
    Has<Focused>,
    Option<&'a Outline>,
    Option<&'a FocusProxy>,
    Option<&'a ForcedState>,
);

/// Draws and removes the focus ring.
pub fn update_focus_ring(
    mut commands: Commands,
    theme: Res<MaterialTheme>,
    visible: Option<Res<InputFocusVisible>>,
    controls: Query<FocusRingItem, With<StateLayerLink>>,
    focus_holders: Query<(), With<Focused>>,
) {
    let ring_visible = visible.is_none_or(|v| v.0);
    let color = theme.scheme.get(ColorRole::Secondary);
    for (entity, focused, outline, proxy, forced) in &controls {
        let proxied = proxy.is_some_and(|p| focus_holders.contains(p.0));
        let want = (focused && ring_visible) || proxied || forced.is_some_and(|f| f.focused);
        match (want, outline) {
            (true, Some(o)) if o.color == color => {}
            (true, _) => {
                commands.entity(entity).try_insert(Outline {
                    width: Val::Px(FOCUS_RING_WIDTH),
                    offset: Val::Px(FOCUS_RING_OFFSET),
                    color,
                });
            }
            (false, Some(_)) => {
                commands.entity(entity).try_remove::<Outline>();
            }
            (false, None) => {}
        }
    }
}

/// Swaps the border role to `Error` while an [`ErrorState`] is present and restores it after.
pub fn update_error_border(
    mut commands: Commands,
    added: Query<Entity, Added<ErrorState>>,
    mut removed: RemovedComponents<ErrorState>,
    saved: Query<&SavedBorderRole>,
    mut borders: Query<&mut BorderRole>,
) {
    for entity in &added {
        let previous = borders.get(entity).ok().map(|b| b.0);
        commands
            .entity(entity)
            .try_insert(SavedBorderRole(previous));
        match borders.get_mut(entity) {
            Ok(mut b) => b.0 = ColorRole::Error,
            Err(_) => {
                commands
                    .entity(entity)
                    .try_insert(BorderRole(ColorRole::Error));
            }
        }
    }
    for entity in removed.read() {
        let Ok(SavedBorderRole(previous)) = saved.get(entity) else {
            continue;
        };
        match (previous, borders.get_mut(entity)) {
            (Some(role), Ok(mut b)) => b.0 = *role,
            (None, Ok(_)) => {
                commands.entity(entity).try_remove::<BorderRole>();
            }
            _ => {}
        }
        commands.entity(entity).try_remove::<SavedBorderRole>();
    }
}

/// Registers the state layer systems.
pub fn build(app: &mut App) {
    // A fresh screen shows where focus is; a pointer press hides the ring (Bevy's click-to-focus).
    app.insert_resource(InputFocusVisible(true));
    app.add_systems(Startup, |visible: Option<ResMut<InputFocusVisible>>| {
        if let Some(mut v) = visible {
            v.0 = true;
        }
    });
    app.add_systems(
        Update,
        (
            spawn_state_layers,
            update_state_layers.after(spawn_state_layers),
            update_focus_ring,
            update_error_border,
        ),
    );
}

#[cfg(test)]
mod tests;
