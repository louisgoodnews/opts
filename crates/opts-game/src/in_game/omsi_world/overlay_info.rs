//! The extra lines of the developer panel (F10): frame rate, the player's place and the road
//! under it, the nearest bus stop and the traffic. The panel's older lines (map, tile and asset
//! counts, vehicle dynamics) stay on the detailed page (Shift + F10).

use bevy::prelude::*;
use opts_traffic::{LaneKind, LaneRules};

use super::traffic::{ScheduledTraffic, TrafficState};

/// How much the developer panel shows (Shift + F10 switches).
#[derive(Resource, Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct DeveloperDetail {
    /// The detailed page: load statistics and vehicle dynamics too.
    pub full: bool,
}

/// What the lines are made from, gathered by the update system.
pub struct PanelFacts<'a> {
    /// Frames per second and the longest frame of the last seconds (ms).
    pub fps: Option<(f32, f32)>,
    /// Session clock text (`12:34:56`), when there is one.
    pub clock: Option<String>,
    /// The player's vehicle: OMSI position (east, north, height m), heading (deg), speed (km/h).
    pub vehicle: Option<([f64; 2], f32, f32, f32)>,
    /// Its place in the session frame (east, north from the start tile's corner), the frame of
    /// the lane graph and of the bus stops.
    pub session: Option<[f32; 2]>,
    /// The lane graph and traffic, when the traffic runs.
    pub traffic: Option<&'a TrafficState>,
    /// The scheduled traffic.
    pub scheduled: Option<&'a ScheduledTraffic>,
    /// Bus stops: id code and position in the session frame (east, north).
    pub stops: Vec<(u64, [f32; 2])>,
}

fn rules_text(r: &LaneRules) -> String {
    let mut parts = Vec::new();
    if r.bus {
        parts.push("bus".to_owned());
    }
    if r.trucks {
        parts.push("trucks".to_owned());
    }
    if r.no_cars {
        parts.push("no cars".to_owned());
    }
    if r.no_overtaking {
        parts.push("no overtaking".to_owned());
    }
    if let Some(p) = r.priority {
        parts.push(format!("priority {p}"));
    }
    if parts.is_empty() {
        "-".to_owned()
    } else {
        parts.join(", ")
    }
}

/// The lane under the vehicle: position on it, height, grade, speed limit and rules.
fn road_line(t: &TrafficState, session: [f32; 2], (height, heading): (f32, f32)) -> Option<String> {
    let graph = t.lane_graph();
    let here = bevy::math::DVec2::new(f64::from(session[0]), f64::from(session[1]));
    let Some((id, s, d)) = graph.nearest_place(
        here,
        f64::from(heading.to_radians()),
        std::f64::consts::PI,
        30.0,
        |l| l.kind == LaneKind::Road,
    ) else {
        return Some("road: no lane within 30 m".to_owned());
    };
    let lane = graph.lane(id)?;
    let grade = (lane.z_at((s + 2.0).min(lane.length())) - lane.z_at((s - 2.0).max(0.0)))
        / (((s + 2.0).min(lane.length())) - (s - 2.0).max(0.0)).max(0.5)
        * 100.0;
    Some(format!(
        "road: lane {} at {s:.0}/{:.0} m ({d:.1} m away) | height {:.1} (vehicle {:+.2}) | grade {grade:+.1} % | limit {} | rules {}",
        id.0,
        lane.length(),
        lane.z_at(s),
        f64::from(height) - lane.z_at(s),
        lane.speed_limit
            .map_or_else(|| "-".to_owned(), |v| format!("{:.0} km/h", v * 3.6)),
        rules_text(&lane.rules),
    ))
}

/// Frames per second and clock, when there is either.
fn head_line(f: &PanelFacts) -> Option<String> {
    let mut head = String::new();
    if let Some((fps, worst)) = f.fps {
        head.push_str(&format!("{fps:.0} fps (worst frame {worst:.0} ms)"));
    }
    if let Some(c) = &f.clock {
        if !head.is_empty() {
            head.push_str(" | ");
        }
        head.push_str(&format!("clock {c}"));
    }
    (!head.is_empty()).then_some(head)
}

/// The lines of the compact page.
#[must_use]
pub fn lines(f: &PanelFacts) -> String {
    let mut out = Vec::new();
    out.extend(head_line(f));
    let Some((abs, height, heading, speed)) = f.vehicle else {
        out.push("vehicle: none".to_owned());
        return out.join("\n");
    };
    out.push(format!(
        "vehicle: east {:.1} north {:.1} height {:.1} | heading {heading:.0}° | {speed:.0} km/h",
        abs[0], abs[1], height
    ));
    let session = f.session.unwrap_or([abs[0] as f32, abs[1] as f32]);
    if let Some(t) = f.traffic {
        out.extend(road_line(t, session, (height, heading)));
    }
    if let Some((id, d)) = f
        .stops
        .iter()
        .map(|(id, p)| {
            (
                *id,
                ((p[0] - session[0]).powi(2) + (p[1] - session[1]).powi(2)).sqrt(),
            )
        })
        .min_by(|a, b| a.1.total_cmp(&b.1))
    {
        out.push(format!("nearest bus stop: {id} at {d:.0} m"));
    }
    if let Some(t) = f.traffic {
        let s = t.stats();
        let mut line = format!(
            "traffic: {} vehicles | hard brakes {} | lane change aborts {}",
            t.agent_count(),
            s.hard_brakes,
            s.lane_change_aborts
        );
        if let Some(sc) = f.scheduled {
            line.push_str(&format!(
                " | scheduled: {} runs on the road, {} vehicles",
                sc.active_runs(),
                sc.driving()
            ));
        }
        out.push(line);
    }
    out.join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn facts() -> PanelFacts<'static> {
        PanelFacts {
            fps: Some((59.6, 21.0)),
            clock: Some("12:34:56".to_owned()),
            vehicle: Some(([892125.9, 5849620.8], 33.3, 306.6, 12.0)),
            session: Some([100.0, 200.0]),
            traffic: None,
            scheduled: None,
            stops: vec![(1234, [130.0, 240.0]), (99, [900.0, 900.0])],
        }
    }

    #[test]
    fn the_short_page_has_the_frame_rate_the_clock_the_map_position_and_the_nearest_stop() {
        let text = lines(&facts());
        assert!(text.contains("60 fps (worst frame 21 ms)"), "{text}");
        assert!(text.contains("clock 12:34:56"), "{text}");
        assert!(
            text.contains("east 892125.9 north 5849620.8 height 33.3"),
            "{text}"
        );
        assert!(
            text.contains("heading 307°") && text.contains("12 km/h"),
            "{text}"
        );
        assert!(text.contains("nearest bus stop: 1234 at 50 m"), "{text}");
    }

    #[test]
    fn without_a_vehicle_the_page_says_so() {
        let mut f = facts();
        f.vehicle = None;
        assert!(lines(&f).ends_with("vehicle: none"));
    }
}
