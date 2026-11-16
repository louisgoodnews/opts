//! The HUD's text and widget-visibility systems: the value and caption texts, and which widgets and rows show.

use super::*;

/// Writes the value and caption texts of the widgets.
pub fn update_texts(
    facts: TextEnv,
    mut texts: Query<(&HudText, &mut Text, Option<&mut InstrumentInk>)>,
) {
    let TextEnv {
        values,
        settings,
        session,
        env,
        weather,
        loc,
        label,
        layout,
        place,
        action_keys,
    } = facts;
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
            (Binding::Fuel, Part::Value) => level_text(values.fuel),
            (Binding::FuelLow, Part::Value) => {
                lamp_text(&loc, values.fuel, ink.map(|i| i.into_inner()))
            }
            (Binding::SessionInfo, Part::Value) => session_info_text(&loc, &session, &place),
            (Binding::SessionClock, Part::Value) => clock_line(&loc, env, weather, &values),
            (Binding::ControlsHint, Part::Value) => {
                controls_hint(&loc, &action_keys, &values, &layout)
            }
            (Binding::CameraLabel, Part::Value) => camera_label_text(&loc, label.shown),
            // The captions with a message key are written by `L10nKey`; the alert line has its
            // own system.
            _ => continue,
        };
        set_text(&mut text, new);
    }
}

/// Shows, hides and removes widgets: the user's switch, the vehicle kind, the engine speed and the
/// camera mode. A row or slot without a visible widget is removed from the layout.
pub fn update_widget_display(
    values: Res<HudValues>,
    settings: Res<GameSettings>,
    layout: Res<HudLayout>,
    mut nodes: DisplayNodes,
    mut slots: DisplaySlots,
    mut rows: DisplayRows,
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
            && (!w.when.needs_rpm || (RPM_PROVIDED && values.rpm.is_some()))
            && (!matches!(w.binding, Binding::Fuel | Binding::FuelLow) || values.fuel.is_some());
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
