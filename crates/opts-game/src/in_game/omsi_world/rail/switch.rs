//! The points ahead of the player's rail vehicle (M-0113): a hint in the alert slot of the HUD and
//! the control protocol `rail.switch`.
//!
//! The points of a track have no state of their own here: the way a vehicle takes at the next
//! points is the `branch` of its [`Train`] (the `rail_branch_*` and `rail_switch_toggle` actions),
//! decided at the moment it passes them, so a switch can never change under a vehicle.

use super::*;
use crate::hud::{AlertSeverity, RaiseAlert};

/// Points are announced from this far ahead (m).
const HINT_REACH_M: f32 = 120.0;

/// The next points ahead of the player's vehicle and the way it will take.
#[derive(Debug, Clone, PartialEq)]
pub struct PointsAhead {
    /// Metres to the points.
    pub distance_m: f32,
    /// The ways open there, straight first.
    pub ways: Vec<Branch>,
    /// The way the vehicle takes: its choice when the points offer it, else straight.
    pub chosen: Branch,
}

impl RailPlayer {
    /// The player's train on its track, when it is placed.
    #[must_use]
    pub fn train(&self) -> Option<&Train> {
        self.train.as_ref()
    }

    /// The player's train, to set the way at the next points.
    pub fn train_mut(&mut self) -> Option<&mut Train> {
        self.train.as_mut()
    }
}

/// The way named by `id`.
#[must_use]
pub fn way_by_id(id: &str) -> Option<Branch> {
    [Branch::Straight, Branch::Left, Branch::Right]
        .into_iter()
        .find(|b| way_id(*b) == id)
}

/// The id of a way (`straight`, `left`, `right`).
#[must_use]
pub fn way_id(b: Branch) -> &'static str {
    match b {
        Branch::Straight => "straight",
        Branch::Left => "left",
        Branch::Right => "right",
    }
}

/// The points within `within` metres of `train`.
#[must_use]
pub fn points_ahead(train: &Train, net: &PedestrianNet, within: f32) -> Option<PointsAhead> {
    let ahead = train.next_switch(net, within)?;
    let chosen = if ahead.branches.contains(&train.branch) {
        train.branch
    } else {
        Branch::Straight
    };
    Some(PointsAhead {
        distance_m: ahead.distance_m,
        ways: ahead.branches,
        chosen,
    })
}

/// Raises the hint when new points come within reach and when the way over them changes.
pub fn switch_hint_system(
    net: Option<Res<RailNetwork>>,
    players: Query<&RailPlayer, With<opts_bevy::PlayerVehicle>>,
    alerts: Option<MessageWriter<RaiseAlert>>,
    mut last: Local<Option<(f32, Branch)>>,
) {
    let (Some(net), Some(mut alerts)) = (net, alerts) else {
        return;
    };
    let found = players
        .iter()
        .find_map(|p| p.train.as_ref())
        .and_then(|t| points_ahead(t, &net.0, HINT_REACH_M));
    let Some(p) = found else {
        *last = None;
        return;
    };
    let new_points = last.is_none_or(|(d, _)| p.distance_m > d + 5.0);
    let changed = last.is_some_and(|(_, b)| b != p.chosen);
    if new_points || changed {
        alerts.write(RaiseAlert {
            id: "rail.switch",
            severity: AlertSeverity::Info,
            key: match p.chosen {
                Branch::Straight => crate::hud::keys::ALERT_SWITCH_STRAIGHT,
                Branch::Left => crate::hud::keys::ALERT_SWITCH_LEFT,
                Branch::Right => crate::hud::keys::ALERT_SWITCH_RIGHT,
            },
            args: vec![(
                "distance",
                format!("{}", (p.distance_m / 10.0).round() as i32 * 10),
            )],
        });
    }
    *last = Some((p.distance_m, p.chosen));
}

#[cfg(test)]
mod tests {
    use super::*;
    use opts_sim::pedestrian::{FootLane, Travel};

    fn net() -> PedestrianNet {
        let lane = |a: Vec3, b: Vec3| FootLane::new(vec![a, b], Travel::Both, 1.0).unwrap();
        let mut n = PedestrianNet::default();
        n.insert_tile(
            (0, 0),
            vec![
                lane(Vec3::ZERO, Vec3::new(0.0, 100.0, 0.0)),
                lane(Vec3::new(0.0, 100.0, 0.0), Vec3::new(0.0, 200.0, 0.0)),
                lane(Vec3::new(0.0, 100.0, 0.0), Vec3::new(50.0, 150.0, 0.0)),
            ],
        );
        n
    }

    #[test]
    fn the_points_ahead_name_their_ways_and_the_one_chosen() {
        let n = net();
        let mut t = Train::new(0, 70.0, 1);
        let p = points_ahead(&t, &n, 120.0).expect("points ahead");
        assert!((p.distance_m - 30.0).abs() < 1e-3);
        assert_eq!(p.chosen, Branch::Straight);
        t.branch = Branch::Right;
        assert_eq!(points_ahead(&t, &n, 120.0).unwrap().chosen, Branch::Right);
        // A way the points do not offer is not chosen.
        t.branch = Branch::Left;
        assert_eq!(
            points_ahead(&t, &n, 120.0).unwrap().chosen,
            Branch::Straight
        );
        assert!(points_ahead(&Train::new(1, 10.0, 1), &n, 500.0).is_none());
    }
}
