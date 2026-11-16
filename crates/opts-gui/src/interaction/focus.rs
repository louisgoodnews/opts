//! Focus: one source of truth, layout-ordered traversal, regions, default focus and restore.
//!
//! Bevy's [`InputFocus`] resource is the focus truth (its `TextInput` and pointer click-to-focus
//! use it); the [`super::state::Focused`] marker mirrors it so queries stay simple, and either side may write:
//! [`sync_input_focus`] reconciles them every frame. Move focus with [`FocusEvent`].

use bevy::input_focus::{tab_navigation::TabIndex, InputFocus};
use bevy::prelude::*;

use crate::actions::ActionInput;

mod handler;
mod sync;

pub use handler::focus_event_handler_system;
pub use sync::{default_focus_system, modal_focus_restore_system, sync_input_focus};

/// Marks an entity as capable of receiving input focus. Every
/// [`Button`](crate::components::button::Button) is focusable.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq, Default)]
#[require(TabIndex)]
pub struct Focusable {
    /// Unused since M-0061: traversal follows the layout tree. Kept for source compatibility.
    pub tab_index: usize,
}

/// A control that keeps the up and down arrow keys for itself (a list box moves its own
/// selection with them); left and right still move focus.
#[derive(Component, Debug, Clone, Copy, Default)]
pub struct OwnsVerticalArrows;

/// A control that is operated by the pointer only and never takes focus (the in-game HUD:
/// `interaction-and-feel.md` section 8).
#[derive(Component, Debug, Clone, Copy, Default)]
pub struct PointerOnly;

/// Landmark kinds of the shell layout (`interaction-and-feel.md` section 2).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Landmark {
    /// Navigation rail.
    Rail,
    /// Main content.
    Content,
    /// Detail panel.
    Detail,
    /// Footer actions.
    Footer,
    /// Toast stack.
    Toasts,
}

/// Marks a container as a landmark region: Tab jumps between regions, arrows stay inside one.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub struct FocusRegion(pub Landmark);

/// The safest control of a screen: focused when it appears and nothing else has focus.
#[derive(Component, Debug, Clone, Copy, Default)]
pub struct DefaultFocus;

/// Event requesting a shift in widget focus.
#[derive(Message, Debug, Clone, Copy, PartialEq, Eq)]
pub enum FocusEvent {
    /// Move focus to next widget in layout order.
    Next,
    /// Move focus to previous widget in layout order.
    Previous,
    /// Explicitly focus a target entity.
    Set(Entity),
    /// Clear all active focus.
    Clear,
}
/// Registers focus messages and systems (also done by [`OptsGuiPlugin`](crate::OptsGuiPlugin)).
pub fn build(app: &mut App) {
    app.add_message::<FocusEvent>();
    if !app.world().contains_resource::<InputFocus>() {
        app.init_resource::<InputFocus>();
    }
    app.add_systems(
        Update,
        (
            default_focus_system,
            modal_focus_restore_system,
            focus_event_handler_system,
            sync_input_focus,
        )
            .chain()
            .after(ActionInput),
    );
}

/// Plugin registering focus navigation systems and events.
#[derive(Default)]
pub struct FocusPlugin;

impl Plugin for FocusPlugin {
    fn build(&self, app: &mut App) {
        build(app);
    }
}

#[cfg(test)]
mod tests;
