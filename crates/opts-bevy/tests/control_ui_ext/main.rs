//! `ui.scrollables`, `ui.scroll`, `ui.toggle`, `ui.select`, `ui.text`, `ui.key`, `ui.click count`
//! against a headless app with real UI layout: every view is scrolled and every control operated
//! by id, through the same messages a person causes.

use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

use bevy::prelude::*;
use opts_bevy::control::{ControlAddr, ControlConfig, ControlPlugin};
use opts_cli::client::{Client, ClientConfig, ClientError};
use opts_gui::components::button::ButtonBuilder;
use opts_gui::components::combobox::ComboboxBuilder;
use opts_gui::components::combobox::{ComboboxHeader, M3Combobox};
use opts_gui::components::switch::SwitchBuilder;
use opts_gui::components::text_field::TextFieldBuilder;
use opts_gui::testing::layout_app;
use opts_gui::ui_id::UiId;
use serde_json::Value;

mod dialog_toggle;
mod fixture;
mod scroll;
mod select;
mod text_keys;
