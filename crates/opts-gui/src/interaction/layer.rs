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

use crate::theme::color::ColorRole;

mod ring;
mod spawn;
mod update;

pub use ring::{update_error_border, update_focus_ring};
pub use spawn::spawn_state_layers;
pub use update::update_state_layers;

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
