//! The choices of a drive that the command line and the control protocol can make without the
//! wizard (M-0141): start point, weather, date, time of day and equipment options. They land in
//! [`NewGameSetup`], where the wizard's own choices are.

use opts_bevy::WeatherCatalogue;
use serde_json::Value;

use crate::new_game_screen::types::NewGameSetup;

/// `YYYY-MM-DD` as year, month, day.
pub fn parse_date(text: &str) -> Result<(i32, u32, u32), String> {
    let bad = || format!("`{text}` is not a date (YYYY-MM-DD)");
    let mut parts = text.trim().splitn(3, '-');
    let y: i32 = parts.next().and_then(|p| p.parse().ok()).ok_or_else(bad)?;
    let m: u32 = parts.next().and_then(|p| p.parse().ok()).ok_or_else(bad)?;
    let d: u32 = parts.next().and_then(|p| p.parse().ok()).ok_or_else(bad)?;
    if !(1..=12).contains(&m) || !(1..=31).contains(&d) {
        return Err(bad());
    }
    Ok((y, m, d))
}

/// `HH:MM` as hour and minute.
pub fn parse_time(text: &str) -> Result<(u32, u32), String> {
    let bad = || format!("`{text}` is not a time of day (HH:MM)");
    let (h, m) = text.trim().split_once(':').ok_or_else(bad)?;
    let (h, m): (u32, u32) = (h.parse().map_err(|_| bad())?, m.parse().map_err(|_| bad())?);
    if h > 23 || m > 59 {
        return Err(bad());
    }
    Ok((h, m))
}

/// `NAME=VALUE` as an equipment option.
pub fn parse_setvar(text: &str) -> Result<(String, i32), String> {
    let (name, value) = text
        .split_once('=')
        .ok_or_else(|| format!("`{text}` is not NAME=VALUE"))?;
    let value: i32 = value
        .trim()
        .parse()
        .map_err(|_| format!("`{text}`: the value must be a whole number"))?;
    if name.trim().is_empty() {
        return Err(format!("`{text}`: the name is empty"));
    }
    Ok((name.trim().to_owned(), value))
}

/// `LINEFILE:TOUR:RUN` as a timetable run (the line file stem may contain spaces).
pub fn parse_trip(text: &str) -> Result<(String, usize, usize), String> {
    let bad = || format!("`{text}` is not a run (LINEFILE:TOUR:RUN)");
    let mut parts = text.trim().rsplitn(3, ':');
    let run: usize = parts.next().and_then(|p| p.parse().ok()).ok_or_else(bad)?;
    let tour: usize = parts.next().and_then(|p| p.parse().ok()).ok_or_else(bad)?;
    let line = parts
        .next()
        .filter(|l| !l.trim().is_empty())
        .ok_or_else(bad)?;
    Ok((line.trim().to_owned(), tour, run))
}

/// The options as they arrive: all optional.
#[derive(Debug, Clone, Default)]
pub struct StartOptions {
    /// Start point.
    pub entry: Option<String>,
    /// Weather preset (id or name).
    pub weather: Option<String>,
    /// `YYYY-MM-DD`.
    pub date: Option<String>,
    /// `HH:MM`.
    pub time: Option<String>,
    /// Equipment options.
    pub setvars: Vec<(String, i32)>,
    /// A timetable run to drive, `LINEFILE:TOUR:RUN` (M-0145); without it no trip is planned.
    pub trip: Option<String>,
}

impl StartOptions {
    /// The options of a JSON object (`entry`, `weather`, `date`, `time`, `setvars`: `{name: n}`).
    pub fn from_json(params: &Value) -> Result<Self, String> {
        let text = |key: &str| match params.get(key) {
            None | Some(Value::Null) => Ok(None),
            Some(Value::String(s)) => Ok(Some(s.clone())),
            Some(_) => Err(format!("`{key}` must be a string")),
        };
        let mut setvars = Vec::new();
        match params.get("setvars") {
            None | Some(Value::Null) => {}
            Some(Value::Object(map)) => {
                for (name, v) in map {
                    let n = v
                        .as_i64()
                        .and_then(|n| i32::try_from(n).ok())
                        .ok_or_else(|| format!("`setvars.{name}` must be a whole number"))?;
                    setvars.push((name.clone(), n));
                }
            }
            Some(_) => return Err("`setvars` must be an object {name: number}".into()),
        }
        Ok(Self {
            entry: text("entry")?,
            weather: text("weather")?,
            date: text("date")?,
            time: text("time")?,
            setvars,
            trip: text("trip")?,
        })
    }

    /// The options of the command line.
    pub fn from_args(args: &opts_cli::args::CliArgs) -> Result<Self, String> {
        let setvars = args
            .setvars
            .iter()
            .map(|s| parse_setvar(s))
            .collect::<Result<Vec<_>, _>>()?;
        Ok(Self {
            entry: args.entry.clone(),
            weather: args.weather.clone(),
            date: args.date.clone(),
            time: args.time.clone(),
            setvars,
            trip: None,
        })
    }

    /// Puts the options into `setup`; the weather name is resolved against the installed presets
    /// when there are any (an unknown one is an error then).
    pub fn apply(
        &self,
        setup: &mut NewGameSetup,
        weather: Option<&WeatherCatalogue>,
    ) -> Result<(), String> {
        if let Some(w) = &self.weather {
            // Before the installation's presets are read the name stands as given.
            setup.weather_preset = match weather.filter(|c| c.is_installed()) {
                Some(cat) => crate::control_environment::weather_preset(cat, w)
                    .ok_or_else(|| format!("no weather preset `{w}` (`sim.get` lists them)"))?,
                None => w.clone(),
            };
        }
        if let Some(d) = &self.date {
            (setup.year, setup.month, setup.day) = parse_date(d)?;
        }
        if let Some(t) = &self.time {
            (setup.hour, setup.minute) = parse_time(t)?;
        }
        if let Some(e) = &self.entry {
            setup.selected_entry_point.clone_from(e);
        }
        for (name, value) in &self.setvars {
            setup.custom_setvars.insert(name.clone(), *value);
        }
        // The stop and the texts are filled in when the map's timetables are read (loading).
        setup.trip = match &self.trip {
            Some(t) => {
                let (line_file, tour, run) = parse_trip(t)?;
                Some(crate::new_game_screen::trips::TripChoice {
                    line_file,
                    tour,
                    run,
                    station_id: 0,
                    stop_name: String::new(),
                    line: String::new(),
                    destination: String::new(),
                    depart_s: 0,
                })
            }
            None => None,
        };
        Ok(())
    }
}

#[cfg(test)]
mod tests;
