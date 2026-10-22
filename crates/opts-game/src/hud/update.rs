//! Change-driven updates of the HUD: texts, visibility by switch and condition, the alert line
//! and the camera label.

use std::collections::HashMap;

use bevy::prelude::*;
use opts_bevy::CameraMode;
use opts_gui::l10n::fluent::Arg;
use opts_gui::l10n::Localization;

use super::alerts::{AlertQueue, AlertSeverity};
use super::build::{AlertPill, HudRow, HudSlot, HudText, HudWidgetNode, Part};
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

fn set_text(text: &mut Text, new: String) {
    if text.0 != new {
        text.0 = new;
    }
}

/// Writes the value and caption texts of the widgets.
#[allow(clippy::too_many_arguments)]
pub fn update_texts(
    values: Res<HudValues>,
    settings: Res<GameSettings>,
    session: Res<InGameSessionInfo>,
    env: Option<Res<SessionEnvironment>>,
    weather: Option<Res<WeatherCatalogue>>,
    loc: Res<Localization>,
    label: Res<CameraLabel>,
    layout: Res<HudLayout>,
    place: Res<crate::session_vehicles::PlaceLabel>,
    action_keys: Option<Res<opts_bevy::ActionKeys>>,
    mut texts: Query<(&HudText, &mut Text, Option<&mut InstrumentInk>)>,
) {
    let unit = SpeedUnit::parse(settings.raw("interface.units").as_deref());
    let fallback = SessionEnvironment::default();
    let env = env.as_deref().unwrap_or(&fallback);
    let no_weather = WeatherCatalogue::default();
    let weather = weather.as_deref().unwrap_or(&no_weather);
    for (hud, mut text, ink) in &mut texts {
        let new = match (hud.binding, hud.part) {
            (Binding::Speed, Part::Value) => format!("{:.0}", unit.convert(values.speed_kmh)),
            (Binding::Speed, Part::Caption) => loc.t(unit.caption_key()),
            (Binding::Gear, Part::Value) => {
                if values.gear.is_empty() {
                    "-".to_owned()
                } else {
                    values.gear.clone()
                }
            }
            (Binding::RailDirection, Part::Value) => match values.rail {
                Some((d, _)) if d < 0 => loc.t(keys::RAIL_REVERSE),
                Some(_) => loc.t(keys::RAIL_FORWARD),
                None => String::new(),
            },
            (Binding::RailLever, Part::Value) => values.rail.map_or_else(String::new, |(_, l)| {
                // Traction above zero, braking below it, in percent.
                let pct = (l * 100.0).round() as i32;
                if pct > 0 {
                    format!("+{pct}")
                } else {
                    pct.to_string()
                }
            }),
            (Binding::Rpm, Part::Value) => values
                .rpm
                .map_or_else(String::new, |r| format!("{:.0}", r.round())),
            (Binding::ParkingBrake, Part::Value) => {
                let on = values.parking_brake;
                if let Some(mut ink) = ink {
                    let want = if on {
                        InstrumentInk::Alarm
                    } else {
                        InstrumentInk::Ok
                    };
                    if *ink != want {
                        *ink = want;
                    }
                }
                loc.t(if on { keys::BRAKE_ON } else { keys::BRAKE_OFF })
            }
            // The map, and the entry point the active vehicle was spawned at while it still stands
            // there (the place label; none without a vehicle).
            (Binding::SessionInfo, Part::Value) => match place.0.as_deref() {
                Some(entry) => loc.t_args(
                    keys::SESSION,
                    &[
                        ("map".to_owned(), Arg::from(session.map_name.as_str())),
                        ("entry".to_owned(), Arg::from(entry)),
                    ],
                ),
                None => loc.t_args(
                    keys::SESSION_MAP,
                    &[("map".to_owned(), Arg::from(session.map_name.as_str()))],
                ),
            },
            (Binding::SessionClock, Part::Value) => loc.t_args(
                keys::CLOCK,
                &[
                    ("time".to_owned(), Arg::from(clock_text(env))),
                    (
                        "weather".to_owned(),
                        Arg::from(name_of(weather, &loc, env.weather())),
                    ),
                ],
            ),
            // The keys of the rebindable actions are the player's current ones.
            (Binding::ControlsHint, Part::Value) => {
                let bound = opts_bevy::action_keys_of(&action_keys);
                if values.rail.is_some() {
                    let new = rail_hint(&loc, bound, layout.compact);
                    set_text(&mut text, new);
                    continue;
                }
                loc.t_args(
                    if layout.compact {
                        keys::HINT_CONTROLS_COMPACT
                    } else {
                        keys::HINT_CONTROLS
                    },
                    &[
                        (
                            "parking".to_owned(),
                            Arg::from(bound.label("parking_brake")),
                        ),
                        ("help".to_owned(), Arg::from(bound.label("help"))),
                    ],
                )
            }
            (Binding::CameraLabel, Part::Value) => label.shown.map_or_else(String::new, |s| {
                let (mode, index, total) = s;
                let name = loc.t(camera_key(mode));
                if total > 1 {
                    loc.t_args(
                        keys::CAMERA_SUB,
                        &[
                            ("name".to_owned(), Arg::from(name)),
                            ("index".to_owned(), Arg::from(index + 1)),
                            ("total".to_owned(), Arg::from(total)),
                        ],
                    )
                } else {
                    name
                }
            }),
            // The captions with a message key are written by `L10nKey`; the alert line has its
            // own system.
            _ => continue,
        };
        set_text(&mut text, new);
    }
}

/// Shows, hides and removes widgets: the user's switch, the vehicle kind, the engine speed and the
/// camera mode. A row or slot without a visible widget is removed from the layout.
#[allow(clippy::type_complexity)]
pub fn update_widget_display(
    values: Res<HudValues>,
    settings: Res<GameSettings>,
    layout: Res<HudLayout>,
    mut nodes: Query<
        (&HudWidgetNode, &mut Node, Option<&mut Visibility>),
        (Without<HudSlot>, Without<HudRow>),
    >,
    mut slots: Query<(&HudSlot, &mut Node), (Without<HudWidgetNode>, Without<HudRow>)>,
    mut rows: Query<(&HudRow, &mut Node), (Without<HudWidgetNode>, Without<HudSlot>)>,
) {
    let mut any_slot: HashMap<Slot, bool> = HashMap::new();
    let mut any_row: HashMap<Row, bool> = HashMap::new();
    for (hud, mut node, vis) in &mut nodes {
        let Some(w) = widget(hud.id) else { continue };
        let on = w
            .switch
            .as_ref()
            .is_none_or(|sw| switch_on(sw, settings.raw(sw.key).as_deref()))
            // Without a vehicle there are no instruments.
            && (values.has_vehicle || w.binding.row() != Some(Row::Instruments))
            && (w.when.kinds.is_empty() || w.when.kinds.contains(&values.kind))
            && (!w.when.needs_rpm || (RPM_PROVIDED && values.rpm.is_some()));
        let display = if on { Display::Flex } else { Display::None };
        if node.display != display {
            node.display = display;
        }
        if let Some(mut vis) = vis {
            let hidden = w.when.hidden_in.contains(&values.camera);
            let want = if hidden {
                Visibility::Hidden
            } else {
                Visibility::Inherited
            };
            // The mini map's visibility belongs to its own system (it hides under the viewpoint
            // editor).
            if *vis != want && w.binding != Binding::MiniMap {
                *vis = want;
            }
        }
        match w.binding.row() {
            Some(row) => *any_row.entry(row).or_default() |= on,
            None => *any_slot.entry(hud.slot).or_default() |= on,
        }
    }
    let cluster_on = any_row.values().any(|on| *on);
    for (row, mut node) in &mut rows {
        let room = row.0 != Row::Hints || layout.hints;
        let display = if room && any_row.get(&row.0).copied().unwrap_or(true) {
            Display::Flex
        } else {
            Display::None
        };
        if node.display != display {
            node.display = display;
        }
    }
    for (slot, mut node) in &mut slots {
        let on = match slot.0 {
            Slot::ClusterBottomLeft => cluster_on,
            other => any_slot.get(&other).copied().unwrap_or(true),
        };
        let display = if on { Display::Flex } else { Display::None };
        if node.display != display {
            node.display = display;
        }
    }
}

/// Shows the alert of the queue in the alert slot; the pill is hidden when there is none.
pub fn update_alert_slot(
    queue: Res<AlertQueue>,
    loc: Res<Localization>,
    mut pills: Query<&mut Visibility, With<AlertPill>>,
    mut texts: Query<(&HudText, &mut Text, &mut InstrumentInk)>,
) {
    let shown = queue.visible();
    for mut vis in &mut pills {
        let want = if shown.is_some() {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        };
        if *vis != want {
            *vis = want;
        }
    }
    let Some(alert) = shown else { return };
    let args: Vec<(String, Arg)> = alert
        .args
        .iter()
        .map(|(k, v)| ((*k).to_owned(), Arg::from(v.as_str())))
        .collect();
    let line = loc.t_args(
        keys::ALERT_LINE,
        &[
            (
                "word".to_owned(),
                Arg::from(loc.t(alert.severity.word_key())),
            ),
            ("text".to_owned(), Arg::from(loc.t_args(alert.key, &args))),
        ],
    );
    let ink = match alert.severity {
        AlertSeverity::Critical => InstrumentInk::Alarm,
        AlertSeverity::Warning => InstrumentInk::Warning,
        AlertSeverity::Info => InstrumentInk::Text,
    };
    for (hud, mut text, mut i) in &mut texts {
        if hud.binding == Binding::Alerts {
            set_text(&mut text, line.clone());
            if *i != ink {
                *i = ink;
            }
        }
    }
}

/// Announces a camera or mode change for 1.5 s. The first sample (the start of the drive) is no
/// change.
pub fn camera_label_changes(
    values: Res<HudValues>,
    mut label: ResMut<CameraLabel>,
    mut pills: Query<(&FadeKind, &mut HudFade)>,
) {
    let now = (values.camera, values.sub_camera);
    match label.last {
        None => label.last = Some(now),
        Some(last) if last != now => {
            label.last = Some(now);
            label.shown = Some((now.0, now.1, values.sub_total));
            for (kind, mut fade) in &mut pills {
                if *kind == FadeKind::CameraLabel {
                    fade.show();
                }
            }
        }
        Some(_) => {}
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
