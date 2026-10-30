//! The texts of the HUD widgets that need more than a format string.

use super::*;

/// The map, and the entry point the active vehicle was spawned at while it still stands there
/// (the place label; none without a vehicle).
pub(super) fn session_info_text(
    loc: &Localization,
    session: &InGameSessionInfo,
    place: &crate::session_vehicles::PlaceLabel,
) -> String {
    match place.0.as_deref() {
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
    }
}

/// The clock line: the time and weather, with the timetable's line, next stop and delay.
pub(super) fn clock_line(
    loc: &Localization,
    env: &SessionEnvironment,
    weather: &WeatherCatalogue,
    values: &HudValues,
) -> String {
    let mut args = vec![
        ("time".to_owned(), Arg::from(clock_text(env))),
        (
            "weather".to_owned(),
            Arg::from(name_of(weather, loc, env.weather())),
        ),
    ];
    match &values.timetable {
        Some(t) => {
            args.push(("line".to_owned(), Arg::from(t.line.as_str())));
            args.push(("stop".to_owned(), Arg::from(t.next_stop.as_str())));
            args.push(("due".to_owned(), Arg::from(hhmm(t.due_s))));
            args.push(("delay".to_owned(), Arg::from(delay_text(t.delay_s))));
            loc.t_args(keys::CLOCK_TIMETABLE, &args)
        }
        None => loc.t_args(keys::CLOCK, &args),
    }
}

/// The controls hint with the keys of the rebindable actions as the player has them now; a
/// different one for rail vehicles.
pub(super) fn controls_hint(
    loc: &Localization,
    action_keys: &Option<Res<opts_bevy::ActionKeys>>,
    values: &HudValues,
    layout: &HudLayout,
) -> String {
    let bound = opts_bevy::action_keys_of(action_keys);
    if values.rail.is_some() {
        return rail_hint(loc, bound, layout.compact);
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

/// The camera label: the view's name, with its number when the vehicle has several of them.
pub(super) fn camera_label_text(
    loc: &Localization,
    shown: Option<(CameraMode, usize, usize)>,
) -> String {
    shown.map_or_else(String::new, |s| {
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
    })
}
