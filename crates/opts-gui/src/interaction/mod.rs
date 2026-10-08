//! Interaction state, event triggers, and focus management module.

pub mod events;
pub mod focus;
pub mod state;

pub use events::{
    ButtonClicked, CheckboxToggled, ComboboxSelected, DateSelected, DateTimeSelected,
    NotebookTabSelected, RadioSelected, SwitchToggled, TextChanged, ThemeChanged, ToastDismissed,
    ToplevelClosed, TreeNodeSelected, TreeNodeToggled,
};
pub use focus::{FocusEvent, FocusPlugin, Focusable};
pub use state::{CheckState, Disabled, Focused, Hovered, Pressed, Selected};
