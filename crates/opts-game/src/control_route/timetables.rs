//! Reads of the timetables (`bus.timetables`): the lines, tours and runs of the player's
//! vehicle's timetable files, as JSON.

use bevy::prelude::*;
use opts_bevy::control::params::{opt_bool, opt_str, opt_u64};
use opts_bevy::control::state::Outcome;
use opts_cli::protocol::RpcError;
use serde_json::{json, Value};

use super::player_route;

/// `h:mm:ss` of a time of day in seconds (wrapped at 24 h).
fn clock_text(s: f32) -> String {
    let t = (s.max(0.0).round() as u32) % 86_400;
    format!("{:02}:{:02}:{:02}", t / 3600, t / 60 % 60, t % 60)
}

pub(super) fn describe_schedule(
    s: &opts_core::formats::timetable::ActiveSchedule,
    stop_limit: usize,
) -> Value {
    json!({
        "line_file": s.line_file,
        "tour": s.tour,
        "trip": s.trip,
        "title": s.title,
        "line": s.line,
        "start": clock_text(s.start_s),
        "stop_count": s.stops.len(),
        "stops": s.stops.iter().take(stop_limit).map(|x| json!({
            "station": x.station_id,
            "name": x.name,
            "arrival": clock_text(x.arrival_s),
            "departure": clock_text(x.departure_s),
            "distance_m": x.distance_m,
        })).collect::<Vec<_>>(),
    })
}

pub(super) fn timetables(world: &mut World, params: &Value) -> Outcome {
    (|| {
        let line = opt_str(params, "line")?.map(str::to_string);
        let tour = opt_u64(params, "tour")?.map(|n| n as usize);
        let limit = opt_u64(params, "limit")?.map_or(200, |n| n as usize);
        let all = opt_bool(params, "all")?.unwrap_or(false);
        let route = player_route(world)?;
        let Some(tt) = route.handle.timetables() else {
            return Ok(json!({ "available": false, "lines": [] }));
        };
        let Some(line) = line else {
            let lines: Vec<Value> = tt
                .lines
                .iter()
                .filter(|(_, l)| all || l.user_allowed)
                .take(limit)
                .map(|(name, l)| {
                    json!({
                        "file": name,
                        "priority": l.priority,
                        "user_allowed": l.user_allowed,
                        "tours": l.tours.len(),
                        "runs": l.tours.iter().map(|t| t.trips.len()).sum::<usize>(),
                    })
                })
                .collect();
            return Ok(json!({ "available": true, "lines": lines }));
        };
        let Some((_, file)) = tt.lines.iter().find(|(n, _)| n.eq_ignore_ascii_case(&line)) else {
            return Err(RpcError::invalid_params(format!("no line file `{line}`")));
        };
        match tour {
            None => Ok(json!({
                "available": true,
                "line": line,
                "tours": file.tours.iter().enumerate().take(limit).map(|(i, t)| json!({
                    "tour": i,
                    "name": t.name,
                    "vehicle_group": t.vehicle_group,
                    "runs": t.trips.len(),
                })).collect::<Vec<_>>(),
            })),
            Some(ti) => {
                let t = file
                    .tours
                    .get(ti)
                    .ok_or_else(|| RpcError::invalid_params(format!("no tour {ti} in `{line}`")))?;
                Ok(json!({
                    "available": true,
                    "line": line,
                    "tour": ti,
                    "name": t.name,
                    "runs": t.trips.iter().enumerate().take(limit).map(|(i, r)| {
                        let trip = tt.trip(&r.trip);
                        json!({
                            "run": i,
                            "trip": r.trip,
                            "title": trip.map(|x| x.title.clone()),
                            "line": trip.map(|x| x.line.clone()),
                            "start": clock_text(r.start_min * 60.0),
                            "stops": trip.map(|x| x.stations.len()),
                        })
                    }).collect::<Vec<_>>(),
                }))
            }
        }
    })()
    .map_or_else(Outcome::err, Outcome::ok)
}
