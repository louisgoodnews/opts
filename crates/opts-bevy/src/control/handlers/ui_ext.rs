//! `ui.scrollables`, `ui.scroll`, `ui.text`, `ui.select`, `ui.toggle`, `ui.hover`, `ui.key`,
//! `ui.drag`: everything an agent needs beyond `ui.click` to operate any screen by id.
//!
//! Like `ui.click` they send the messages and input events a person would cause (an
//! [`Activated`], a [`FocusEvent`], keyboard events, a cursor move). Scrolling sets the same
//! scroll offsets the wheel sets. While a modal dialog is open only elements inside it can be
//! scrolled, selected or toggled (`1009`), mirroring the focus scope the dialog creates.

use bevy::ecs::entity::Entity;
use bevy::ecs::world::World;
use bevy::input::keyboard::{Key, KeyCode, KeyboardInput, NativeKeyCode};
use bevy::input::mouse::{MouseButton, MouseButtonInput, MouseMotion};
use bevy::input::ButtonState;
use bevy::math::Vec2;
use bevy::prelude::{ChildOf, Display, Visibility, With};
use bevy::ui::{ComputedNode, Node, OverflowAxis, ScrollPosition};
use bevy::window::{CursorMoved, Window};
use opts_cli::protocol::{code, RpcError};
use opts_gui::components::combobox::{ComboboxHeader, ComboboxMenuItem, M3Combobox};
use opts_gui::components::modal::ModalRoot;
use opts_gui::components::text_field::M3TextField;
use opts_gui::interaction::{activation::Activated, focus::FocusEvent, layer::FocusProxy};
use opts_gui::patterns::master_detail::{
    MasterDetail, MasterDetailChosen, MasterDetailView, MasterList,
};
use opts_gui::ui_id::UiId;
use opts_gui::ui_tree::{find_entity, rect_of, snapshot, UiNode, UiRole};
use serde_json::{json, Value};
use std::time::{Duration, Instant};

use crate::control::handlers::input::primary_window_or_placeholder;
use crate::control::keys::parse_key;
use crate::control::params::{opt_bool, opt_f64, opt_str, opt_u64, str_param};
use crate::control::state::{ControlRegistry, ControlState, Outcome, Release};

/// Registers the methods.
pub fn register(r: &mut ControlRegistry) {
    r.register("ui.scrollables", scrollables);
    r.register("ui.scroll", scroll);
    r.register("ui.text", text);
    r.register("ui.select", select);
    r.register("ui.toggle", toggle);
    r.register("ui.hover", hover);
    r.register("ui.key", key);
    r.register("ui.drag", drag);
}

// ---- scrollables ------------------------------------------------------------------------

mod scroll;
pub use scroll::{
    collect_scrollables, scrollable_item_count, scrollables_json, ScrollKind, Scrollable,
};
use scroll::{scroll, scrollables};

// ---- text, select, toggle ------------------------------------------------------------------

mod text_input;
use text_input::{select, text, toggle};

// ---- helpers, keys and pointer ----------------------------------------------------------

mod helpers;
mod keys;
mod pointer;

pub use helpers::{is_displayed, is_inside, open_modals};
pub use keys::type_text;
pub use pointer::move_cursor;
use helpers::{check_modal_scope, check_usable, nearest_id, node_of, not_found};
use keys::key;
use pointer::{drag, hover};
