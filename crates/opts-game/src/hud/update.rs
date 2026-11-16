//! Change-driven updates of the HUD: texts, visibility by switch and condition, the alert line
//! and the camera label.

use std::collections::HashMap;

use bevy::prelude::*;
use opts_bevy::CameraMode;
use opts_gui::l10n::fluent::Arg;
use opts_gui::l10n::Localization;

use super::alerts::{AlertQueue, AlertSeverity};
use super::build::{AlertPill, HudText, Part};
use super::fade::{FadeKind, HudFade};
use super::keys;
use super::layout::HudLayout;
use super::values::{HudValues, SpeedUnit, RPM_PROVIDED};
use super::widget::{switch_on, widget, Binding, Row, Slot};
use crate::in_game::instruments::InstrumentInk;
use crate::in_game::types::InGameSessionInfo;
use crate::new_game_screen::weather::name_of;
use crate::settings::GameSettings;
use opts_bevy::{SessionEnvironment, WeatherCatalogue};

mod fuel;
pub use fuel::update_fuel_bars;
mod texts;
use super::params::{DisplayNodes, DisplayRows, DisplaySlots, TextEnv};
use fuel::{lamp_text, level_text};
use texts::{camera_label_text, clock_line, controls_hint, session_info_text};

mod alerts;
mod widgets;

pub use alerts::{camera_label_changes, update_alert_slot};
pub use widgets::{update_texts, update_widget_display};

/// The last camera state that the label announced, and what it says.
#[derive(Resource, Debug, Default, Clone, PartialEq)]
pub struct CameraLabel {
    /// Last seen mode and sub camera; `None` before the first sample (no label at the start).
    pub last: Option<(CameraMode, usize)>,
    /// What the label says now (kept while it fades out).
    pub shown: Option<(CameraMode, usize, usize)>,
}

/// Message key of the label of a camera mode.
#[must_use]
pub const fn camera_key(mode: CameraMode) -> &'static str {
    match mode {
        CameraMode::Ego => keys::CAMERA_EGO,
        CameraMode::Cockpit => keys::CAMERA_COCKPIT,
        CameraMode::External => keys::CAMERA_EXTERNAL,
        CameraMode::Passenger => keys::CAMERA_PASSENGER,
        CameraMode::FreeRoam => keys::CAMERA_FREE,
    }
}

/// The clock text of the session environment (`HH:MM:SS`).
#[must_use]
pub fn clock_text(env: &SessionEnvironment) -> String {
    env.clock_text()
}

/// Advances the session's play time with the virtual time (paused with the game). The time of
/// day is the [`SessionEnvironment`]'s.
pub fn tick_session_clock(time: Res<Time>, mut session: ResMut<InGameSessionInfo>) {
    session.sim_elapsed_seconds += time.delta_secs();
}

/// `h:mm` of a time of day in seconds (wrapping at 24 h).
fn hhmm(seconds: f32) -> String {
    let m = (seconds.rem_euclid(86_400.0) / 60.0) as u32;
    format!("{}:{:02}", m / 60, m % 60)
}

/// The delay as `+m:ss` (late) or `-m:ss` (early); `h:mm:ss` from an hour on.
fn delay_text(seconds: f32) -> String {
    let s = seconds.abs() as u32;
    let sign = if seconds < 0.0 { '-' } else { '+' };
    if s >= 3600 {
        format!("{sign}{}:{:02}:{:02}", s / 3600, s / 60 % 60, s % 60)
    } else {
        format!("{sign}{}:{:02}", s / 60, s % 60)
    }
}

fn set_text(text: &mut Text, new: String) {
    if text.0 != new {
        text.0 = new;
    }
}

/// The control hint of a rail vehicle: the keys of its lever and its direction selector.
fn rail_hint(loc: &Localization, bound: &opts_bevy::ActionKeys, compact: bool) -> String {
    loc.t_args(
        if compact {
            keys::HINT_CONTROLS_RAIL_COMPACT
        } else {
            keys::HINT_CONTROLS_RAIL
        },
        &[
            ("up".to_owned(), Arg::from(bound.label("rail_lever_up"))),
            ("down".to_owned(), Arg::from(bound.label("rail_lever_down"))),
            ("zero".to_owned(), Arg::from(bound.label("rail_lever_zero"))),
            ("forward".to_owned(), Arg::from(bound.label("rail_forward"))),
            ("reverse".to_owned(), Arg::from(bound.label("rail_reverse"))),
            (
                "switch".to_owned(),
                Arg::from(bound.label("rail_switch_toggle")),
            ),
            ("help".to_owned(), Arg::from(bound.label("help"))),
        ],
    )
}

#[cfg(test)]
mod tests;
