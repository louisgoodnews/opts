//! Interaction state, activation, focus, tooltips and the state layer mechanism.

use bevy::prelude::*;

pub mod activation;
pub mod events;
pub mod focus;
pub mod handlers;
pub mod layer;
pub mod navigation;
pub mod scroll_into_view;
pub mod state;
pub mod tooltip;

pub use activation::Activated;
pub use events::{
    ButtonClicked, CheckboxToggled, ComboboxSelected, DateSelected, DateTimeSelected,
    NotebookTabSelected, RadioSelected, SwitchToggled, TextChanged, ThemeChanged, ToastDismissed,
    ToplevelClosed, TreeNodeSelected, TreeNodeToggled,
};
pub use focus::{
    DefaultFocus, FocusEvent, FocusPlugin, FocusRegion, Focusable, Landmark, OwnsVerticalArrows,
    PointerOnly,
};
pub use layer::StateLayerRole;
pub use state::{
    CheckState, Disabled, DisabledReason, ErrorState, Focused, Hovered, Pressed, Selected, UiState,
};
pub use tooltip::Tooltip;

/// Registers the interaction mechanism every widget shares: activation on release, state layers,
/// focus ring and tooltips. Added by [`OptsGuiPlugin`](crate::OptsGuiPlugin); screens need no
/// registration of their own.
#[derive(Default)]
pub struct InteractionPlugin;

impl Plugin for InteractionPlugin {
    fn build(&self, app: &mut App) {
        activation::build(app);
        layer::build(app);
        tooltip::build(app);
        focus::build(app);
        scroll_into_view::build(app);
        handlers::build(app);
        crate::ui_id::build(app);
        crate::not_yet::build(app);
    }
}
