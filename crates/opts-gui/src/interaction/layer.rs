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

use crate::components::button::Button;
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
    controls: Query<Entity, (Added<Button>, Without<StateLayerLink>)>,
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
);

/// Applies the state layer tint, the disabled scrim and the border radius to the helper nodes.
pub fn update_state_layers(
    theme: Res<MaterialTheme>,
    visible: Option<Res<InputFocusVisible>>,
    controls: Query<ControlItem>,
    mut nodes: Query<(&mut BackgroundColor, &mut Node), Without<StateLayerLink>>,
) {
    let ring_visible = visible.is_none_or(|v| v.0);
    for (link, hovered, pressed, focused, disabled, error, role, bg_role, control_node) in &controls
    {
        let state = ui_state(hovered, pressed, focused, disabled, error, ring_visible);
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
type FocusRingItem<'a> = (Entity, Has<Focused>, Option<&'a Outline>);

/// Draws and removes the focus ring.
pub fn update_focus_ring(
    mut commands: Commands,
    theme: Res<MaterialTheme>,
    visible: Option<Res<InputFocusVisible>>,
    controls: Query<FocusRingItem, With<StateLayerLink>>,
) {
    let ring_visible = visible.is_none_or(|v| v.0);
    let color = theme.scheme.get(ColorRole::Secondary);
    for (entity, focused, outline) in &controls {
        let want = focused && ring_visible;
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
    if !app.world().contains_resource::<InputFocusVisible>() {
        app.insert_resource(InputFocusVisible(true));
    }
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
mod tests {
    use super::*;
    use crate::components::button::ButtonBuilder;
    use crate::testing::headless_app;

    fn spawn(app: &mut App) -> (Entity, StateLayerLink) {
        let entity = {
            let mut commands = app.world_mut().commands();
            ButtonBuilder::filled("Go").spawn(&mut commands)
        };
        app.update();
        app.update();
        let link = *app.world().get::<StateLayerLink>(entity).expect("link");
        (entity, link)
    }

    fn alpha(app: &App, e: Entity) -> f32 {
        app.world()
            .get::<BackgroundColor>(e)
            .map_or(-1.0, |b| b.0.alpha())
    }

    #[test]
    fn rest_hover_focus_pressed_have_the_documented_layers() {
        let mut app = headless_app();
        let (button, link) = spawn(&mut app);
        assert_eq!(alpha(&app, link.layer), 0.0);

        app.world_mut().entity_mut(button).insert(Hovered(true));
        app.update();
        assert!((alpha(&app, link.layer) - 0.08).abs() < 1e-4, "hover 8 %");

        app.world_mut()
            .resource_mut::<ButtonInput<MouseButton>>()
            .press(MouseButton::Left);
        app.world_mut().entity_mut(button).insert(Pressed);
        app.update();
        app.world_mut()
            .resource_mut::<ButtonInput<MouseButton>>()
            .release(MouseButton::Left);
        assert!(
            (alpha(&app, link.layer) - 0.10).abs() < 1e-4,
            "pressed 10 %"
        );

        app.world_mut().entity_mut(button).remove::<Pressed>();
        app.world_mut().entity_mut(button).insert(Hovered(false));
        app.world_mut().entity_mut(button).insert(Focused);
        app.update();
        assert!((alpha(&app, link.layer) - 0.10).abs() < 1e-4, "focus 10 %");
        assert!(app.world().get::<Outline>(button).is_some(), "focus ring");

        app.world_mut().entity_mut(button).remove::<Focused>();
        app.update();
        assert!(app.world().get::<Outline>(button).is_none());
        assert_eq!(alpha(&app, link.layer), 0.0);
    }

    #[test]
    fn disabled_scrim_leaves_38_percent_and_blocks_hover() {
        let mut app = headless_app();
        let (button, link) = spawn(&mut app);
        assert_eq!(alpha(&app, link.scrim), 0.0);
        app.world_mut()
            .entity_mut(button)
            .insert((Disabled, Hovered(true)));
        app.update();
        assert!((alpha(&app, link.scrim) - 0.62).abs() < 1e-4);
        assert_eq!(alpha(&app, link.layer), 0.0, "no hover layer when disabled");
    }

    #[test]
    fn focus_ring_is_hidden_after_pointer_focus() {
        let mut app = headless_app();
        let (button, _) = spawn(&mut app);
        app.world_mut().insert_resource(InputFocusVisible(false));
        app.world_mut().entity_mut(button).insert(Focused);
        app.update();
        assert!(app.world().get::<Outline>(button).is_none());
    }

    #[test]
    fn error_state_swaps_the_border_and_restores_it() {
        let mut app = headless_app();
        let (button, _) = spawn(&mut app);
        app.world_mut()
            .entity_mut(button)
            .insert(BorderRole(ColorRole::Outline));
        app.update();
        app.world_mut()
            .entity_mut(button)
            .insert(ErrorState::new("Needs a name"));
        app.update();
        assert_eq!(
            app.world().get::<BorderRole>(button).map(|b| b.0),
            Some(ColorRole::Error)
        );
        app.world_mut().entity_mut(button).remove::<ErrorState>();
        app.update();
        assert_eq!(
            app.world().get::<BorderRole>(button).map(|b| b.0),
            Some(ColorRole::Outline)
        );
    }
}
