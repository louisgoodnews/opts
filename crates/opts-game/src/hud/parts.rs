//! Markers and building blocks of the HUD: the components the update systems look for and the
//! helpers that make panels, texts and readouts from instrument tokens.

use bevy::prelude::*;
use opts_gui::prelude::*;
use opts_gui::theme::InstrumentPalette;

use super::keys;
use super::profile::HudProfile;
use super::widget::{Binding, HudWidget, Row, Slot, VehicleKind};
use crate::in_game::instruments::{InstrumentFade, InstrumentInk, InstrumentPanel};

mod builders;
mod components;
mod instrument;
mod readout;
pub(super) use builders::{panel, panel_node, texts};
pub use components::*;
pub(super) use instrument::{build_instrument, text_widget};
pub(super) use readout::readout;
