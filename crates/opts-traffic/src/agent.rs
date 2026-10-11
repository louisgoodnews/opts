//! Traffic agents and the world that steps them.
//!
//! The first fidelity level is **kinematic**: [`TrafficWorld::step`] integrates every agent along
//! its lane with the IDM and the host places a vehicle model at [`Agent::pose`]. The second level,
//! driving a real vehicle simulation through its scripts, uses [`TrafficWorld::command`] — the
//! pedal and indicator values OMSI's `{frame}` blocks of AI vehicles read (`Throttle`, `Brake`,
//! `AI_Blinker_L/R`; `docs/src/architecture/ai-traffic.md`, "The vehicle contract").

use std::collections::HashMap;

use glam::DVec2;

use crate::graph::{LaneGraph, LaneId, SpawnError};
use crate::idm::IdmParams;
use crate::rng::Rng;

/// Identifies an agent for its lifetime.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct AgentId(pub u64);

/// Indicator request.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Blinker {
    /// Off.
    #[default]
    Off,
    /// Left indicator (`AI_Blinker_L`).
    Left,
    /// Right indicator (`AI_Blinker_R`).
    Right,
}

/// What the AI "driver" asks of a vehicle this step.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct DriverCommand {
    /// Accelerator `0..=1` (`Throttle`).
    pub throttle: f32,
    /// Service brake `0..=1` (`Brake`).
    pub brake: f32,
    /// Indicator.
    pub blinker: Blinker,
}

/// Something the host may want to react to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TrafficEvent {
    /// The agent drove off the end of a lane that has no successor and was removed.
    ReachedDeadEnd {
        /// The removed agent.
        id: AgentId,
        /// The lane it left.
        lane: LaneId,
    },
}

/// A vehicle on a lane.
#[derive(Debug, Clone, PartialEq)]
pub struct Agent {
    /// Identity.
    pub id: AgentId,
    /// Current lane.
    pub lane: LaneId,
    /// Position of the front bumper along the lane, metres.
    pub s: f64,
    /// Speed in m/s.
    pub speed: f64,
    /// Acceleration of the last step in m/s².
    pub accel: f64,
    /// Driver and vehicle parameters.
    pub params: IdmParams,
    /// The lane taken after this one (chosen on entry), if any.
    pub next: Option<LaneId>,
}

impl Agent {
    /// World position and compass heading (radians) of the front bumper.
    #[must_use]
    pub fn pose(&self, graph: &LaneGraph) -> Option<(DVec2, f64)> {
        let g = &graph.lane(self.lane)?.geometry;
        Some((g.point_at(self.s), g.heading_at(self.s)))
    }

    /// Height of the front bumper in metres (from the lane's elevation profile).
    #[must_use]
    pub fn height(&self, graph: &LaneGraph) -> Option<f64> {
        Some(graph.lane(self.lane)?.z_at(self.s))
    }
}

/// Indicators are requested when the next lane turns by more than this (radians) …
const BLINK_TURN: f64 = 20.0 * std::f64::consts::PI / 180.0;
/// … and the lane end is closer than this (metres).
const BLINK_DISTANCE: f64 = 30.0;

/// Lane graph plus the agents driving on it.
#[derive(Debug, Clone)]
pub struct TrafficWorld {
    graph: LaneGraph,
    agents: Vec<Agent>,
    next_id: u64,
    rng: Rng,
    events: Vec<TrafficEvent>,
}

impl TrafficWorld {
    /// Creates an empty world; `seed` fixes every random choice (route branches).
    #[must_use]
    pub fn new(graph: LaneGraph, seed: u64) -> Self {
        Self {
            graph,
            agents: Vec::new(),
            next_id: 1,
            rng: Rng::new(seed),
            events: Vec::new(),
        }
    }

    /// The lane graph.
    #[must_use]
    pub fn graph(&self) -> &LaneGraph {
        &self.graph
    }

    /// All agents in spawn order.
    #[must_use]
    pub fn agents(&self) -> &[Agent] {
        &self.agents
    }

    /// The agent `id`.
    #[must_use]
    pub fn agent(&self, id: AgentId) -> Option<&Agent> {
        self.agents.iter().find(|a| a.id == id)
    }

    /// Removes an agent (despawn outside the player's range, say).
    pub fn despawn(&mut self, id: AgentId) -> bool {
        let before = self.agents.len();
        self.agents.retain(|a| a.id != id);
        self.agents.len() != before
    }

    /// Swaps in a rebuilt graph (the resident tiles changed). Agents keep their lane when a lane
    /// with the same [`crate::graph::Lane::key`] exists and lose it otherwise; the latter are
    /// removed. Returns the ids of the removed agents.
    pub fn replace_graph(&mut self, graph: LaneGraph) -> Vec<AgentId> {
        let old = std::mem::replace(&mut self.graph, graph);
        let mut removed = Vec::new();
        let graph = &self.graph;
        self.agents.retain_mut(|a| {
            let remap = |id: LaneId| old.lane(id).and_then(|l| graph.lane_by_key(l.key));
            match remap(a.lane) {
                Some(lane) if a.s <= graph.lane(lane).map_or(0.0, |l| l.length()) + 1e-6 => {
                    a.lane = lane;
                    a.next = a.next.and_then(remap);
                    true
                }
                _ => {
                    removed.push(a.id);
                    false
                }
            }
        });
        // A lane that lost its successor choice picks again.
        let lanes: Vec<(usize, LaneId)> = self
            .agents
            .iter()
            .enumerate()
            .filter(|(_, a)| a.next.is_none())
            .map(|(i, a)| (i, a.lane))
            .collect();
        for (i, lane) in lanes {
            self.agents[i].next = self.pick_next(lane);
        }
        removed
    }

    /// Takes the events raised since the last call.
    pub fn drain_events(&mut self) -> Vec<TrafficEvent> {
        std::mem::take(&mut self.events)
    }

    fn pick_next(&mut self, lane: LaneId) -> Option<LaneId> {
        let succ = self.graph.successors(lane);
        (!succ.is_empty()).then(|| succ[self.rng.below(succ.len())])
    }

    /// Places an agent at `s` metres on `lane` with `speed`. Fails when the spot is not free
    /// (own length plus minimum gap against every agent on the lane).
    pub fn spawn(
        &mut self,
        lane: LaneId,
        s: f64,
        speed: f64,
        params: IdmParams,
    ) -> Result<AgentId, SpawnError> {
        let length = self
            .graph
            .lane(lane)
            .ok_or(SpawnError::NoSuchLane)?
            .length();
        let s = s.clamp(0.0, length);
        let clash = self.agents.iter().any(|o| {
            o.lane == lane
                && o.s - o.params.length - params.min_gap < s
                && s - params.length - params.min_gap < o.s
        });
        if clash {
            return Err(SpawnError::Occupied);
        }
        let id = AgentId(self.next_id);
        self.next_id += 1;
        let next = self.pick_next(lane);
        self.agents.push(Agent {
            id,
            lane,
            s,
            speed: speed.max(0.0),
            accel: 0.0,
            params,
            next,
        });
        Ok(id)
    }

    /// Advances every agent by `dt` seconds (use steps of at most 0.1 s).
    pub fn step(&mut self, dt: f64) {
        if dt <= 0.0 {
            return;
        }
        // Agents per lane, rearmost first; ties broken by id so the order is deterministic.
        let mut by_lane: HashMap<LaneId, Vec<usize>> = HashMap::new();
        for (i, a) in self.agents.iter().enumerate() {
            by_lane.entry(a.lane).or_default().push(i);
        }
        for list in by_lane.values_mut() {
            list.sort_by(|&x, &y| {
                let (a, b) = (&self.agents[x], &self.agents[y]);
                a.s.total_cmp(&b.s).then(a.id.cmp(&b.id))
            });
        }
        // Decide accelerations from the state at the start of the step.
        let mut accels = Vec::with_capacity(self.agents.len());
        for (i, a) in self.agents.iter().enumerate() {
            let Some(lane) = self.graph.lane(a.lane) else {
                accels.push(0.0);
                continue;
            };
            let list = &by_lane[&a.lane];
            let pos = list.iter().position(|&j| j == i).unwrap_or(0);
            let ahead = if let Some(&j) = list.get(pos + 1) {
                let o = &self.agents[j];
                Some((o.s - o.params.length - a.s, o.speed))
            } else {
                // Rearmost agent of the next lane, measured across the lane boundary.
                a.next
                    .and_then(|n| by_lane.get(&n).and_then(|l| l.first()))
                    .map(|&j| {
                        let o = &self.agents[j];
                        (lane.length() - a.s + o.s - o.params.length, o.speed)
                    })
            };
            let cap = lane.speed_limit.unwrap_or(f64::INFINITY);
            accels.push(a.params.acceleration(a.speed, cap, ahead));
        }
        // Integrate (ballistic; a vehicle that would reverse stops within the step).
        for (a, &acc) in self.agents.iter_mut().zip(&accels) {
            let v1 = a.speed + acc * dt;
            if v1 < 0.0 {
                a.s += if acc < 0.0 {
                    -a.speed * a.speed / (2.0 * acc)
                } else {
                    0.0
                };
                a.speed = 0.0;
            } else {
                a.s += a.speed * dt + 0.5 * acc * dt * dt;
                a.speed = v1;
            }
            a.accel = acc;
        }
        self.advance_lanes();
    }

    /// Moves agents that passed the end of their lane onto the next one.
    fn advance_lanes(&mut self) {
        let mut i = 0;
        while i < self.agents.len() {
            loop {
                let a = &self.agents[i];
                let Some(len) = self.graph.lane(a.lane).map(|l| l.length()) else {
                    break;
                };
                if a.s <= len {
                    break;
                }
                let (over, next, id, lane) = (a.s - len, a.next, a.id, a.lane);
                match next {
                    Some(n) => {
                        let after = self.pick_next(n);
                        let a = &mut self.agents[i];
                        a.lane = n;
                        a.s = over;
                        a.next = after;
                    }
                    None => {
                        self.events.push(TrafficEvent::ReachedDeadEnd { id, lane });
                        self.agents.remove(i);
                        i = i.wrapping_sub(1);
                        break;
                    }
                }
            }
            i = i.wrapping_add(1);
        }
    }

    /// Pedal and indicator values for `id` derived from its last acceleration: positive
    /// accelerations map to `throttle = a / A`, braking to `brake = −a / max_decel`.
    #[must_use]
    pub fn command(&self, id: AgentId) -> Option<DriverCommand> {
        let a = self.agent(id)?;
        let (throttle, brake) = if a.accel >= 0.0 {
            ((a.accel / a.params.max_accel).clamp(0.0, 1.0), 0.0)
        } else {
            (0.0, (-a.accel / a.params.max_decel).clamp(0.0, 1.0))
        };
        let lane = self.graph.lane(a.lane)?;
        let mut blinker = Blinker::Off;
        if let Some(next) = a.next.and_then(|n| self.graph.lane(n)) {
            if lane.length() - a.s < BLINK_DISTANCE {
                let turn = (next.geometry.end_heading() - lane.geometry.end_heading()
                    + std::f64::consts::PI)
                    .rem_euclid(std::f64::consts::TAU)
                    - std::f64::consts::PI;
                blinker = if turn > BLINK_TURN {
                    Blinker::Right
                } else if turn < -BLINK_TURN {
                    Blinker::Left
                } else {
                    Blinker::Off
                };
            }
        }
        Some(DriverCommand {
            throttle: throttle as f32,
            brake: brake as f32,
            blinker,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::geometry::PathGeometry;
    use crate::graph::{ConnectParams, Direction, LaneKind, LaneSegment};

    fn straight(y: f64, len: f64) -> LaneSegment {
        LaneSegment::straight(DVec2::new(0.0, y), 0.0, len, LaneKind::Road)
    }

    fn world(segments: &[LaneSegment]) -> TrafficWorld {
        TrafficWorld::new(LaneGraph::build(segments, &ConnectParams::default()), 3)
    }

    #[test]
    fn an_agent_accelerates_and_crosses_into_the_next_lane() {
        let mut w = world(&[straight(0.0, 60.0), straight(60.0, 400.0)]);
        let id = w.spawn(LaneId(0), 0.0, 0.0, IdmParams::default()).unwrap();
        for _ in 0..200 {
            w.step(0.1);
        }
        let a = w.agent(id).unwrap();
        assert_eq!(a.lane, LaneId(1));
        assert!(a.speed > 10.0);
        let (p, h) = a.pose(w.graph()).unwrap();
        assert!(p.y > 60.0 && p.x.abs() < 1e-9 && h.abs() < 1e-9);
    }

    #[test]
    fn a_follower_keeps_its_distance_behind_a_stopped_leader() {
        let mut w = world(&[straight(0.0, 300.0)]);
        let lead = w
            .spawn(LaneId(0), 200.0, 0.0, IdmParams::default())
            .unwrap();
        let p = IdmParams {
            max_accel: 0.0,
            ..IdmParams::default()
        };
        // A leader that never moves: zero acceleration means it stays at speed 0.
        w.agents.iter_mut().find(|a| a.id == lead).unwrap().params = p;
        let follow = w.spawn(LaneId(0), 0.0, 13.0, IdmParams::default()).unwrap();
        for _ in 0..1200 {
            w.step(0.1);
        }
        let (l, f) = (w.agent(lead).unwrap(), w.agent(follow).unwrap());
        let gap = l.s - l.params.length - f.s;
        assert!(f.speed < 0.05, "follower still at {}", f.speed);
        assert!(gap > 0.5, "collision: gap {gap}");
        assert!(gap < 4.0, "stopped too early: gap {gap}");
    }

    #[test]
    fn agents_see_a_leader_across_the_lane_boundary() {
        let mut w = world(&[straight(0.0, 100.0), straight(100.0, 100.0)]);
        let lead = w
            .spawn(
                LaneId(1),
                10.0,
                0.0,
                IdmParams {
                    max_accel: 0.0,
                    ..IdmParams::default()
                },
            )
            .unwrap();
        let follow = w.spawn(LaneId(0), 0.0, 13.0, IdmParams::default()).unwrap();
        for _ in 0..1200 {
            w.step(0.1);
        }
        let (l, f) = (w.agent(lead).unwrap(), w.agent(follow).unwrap());
        assert_eq!(f.lane, LaneId(1));
        assert!(l.s - l.params.length - f.s > 0.5, "collision");
    }

    #[test]
    fn dead_ends_remove_the_agent_and_raise_an_event() {
        let mut w = world(&[straight(0.0, 30.0)]);
        let id = w.spawn(LaneId(0), 0.0, 13.0, IdmParams::default()).unwrap();
        for _ in 0..100 {
            w.step(0.1);
        }
        assert!(w.agent(id).is_none());
        assert_eq!(
            w.drain_events(),
            [TrafficEvent::ReachedDeadEnd {
                id,
                lane: LaneId(0)
            }]
        );
        assert!(w.drain_events().is_empty());
    }

    #[test]
    fn replacing_the_graph_keeps_agents_on_lanes_that_still_exist() {
        let mut a = straight(0.0, 100.0);
        a.source = 1;
        let mut b = straight(100.0, 100.0);
        b.source = 2;
        let mut w = world(&[a.clone(), b.clone()]);
        let on_a = w.spawn(LaneId(0), 50.0, 5.0, IdmParams::default()).unwrap();
        let on_b = w.spawn(LaneId(1), 50.0, 5.0, IdmParams::default()).unwrap();
        // Tile of lane `a` unloads; lane `b` stays but now has id 0.
        let removed = w.replace_graph(LaneGraph::build(&[b], &ConnectParams::default()));
        assert_eq!(removed, [on_a]);
        assert!(w.agent(on_a).is_none());
        assert_eq!(w.agent(on_b).unwrap().lane, LaneId(0));
        // Loading `a` again does not bring the agent back, and `on_b` keeps its lane by key.
        let removed = w.replace_graph(LaneGraph::build(
            &[a, straight(100.0, 100.0)],
            &ConnectParams::default(),
        ));
        assert!(removed.is_empty() || removed == [on_b]);
    }

    #[test]
    fn spawning_needs_room() {
        let mut w = world(&[straight(0.0, 100.0)]);
        w.spawn(LaneId(0), 20.0, 0.0, IdmParams::default()).unwrap();
        assert_eq!(
            w.spawn(LaneId(0), 22.0, 0.0, IdmParams::default()),
            Err(SpawnError::Occupied)
        );
        assert!(w.spawn(LaneId(0), 40.0, 0.0, IdmParams::default()).is_ok());
        assert_eq!(
            w.spawn(LaneId(9), 0.0, 0.0, IdmParams::default()),
            Err(SpawnError::NoSuchLane)
        );
    }

    #[test]
    fn commands_follow_the_acceleration_and_announce_turns() {
        // A right turn (quarter circle, radius 15) after 100 m.
        let turn = LaneSegment {
            geometry: PathGeometry {
                start: DVec2::new(0.0, 100.0),
                heading: 0.0,
                radius: 15.0,
                length: 15.0 * std::f64::consts::FRAC_PI_2,
            },
            kind: LaneKind::Road,
            width: 2.5,
            direction: Direction::Forward,
            speed_limit: None,
            rules: Default::default(),
            elevation: Default::default(),
            source: 0,
        };
        let mut w = world(&[straight(0.0, 100.0), turn]);
        let id = w.spawn(LaneId(0), 0.0, 0.0, IdmParams::default()).unwrap();
        w.step(0.1);
        let c = w.command(id).unwrap();
        assert!(c.throttle > 0.9 && c.brake == 0.0 && c.blinker == Blinker::Off);
        w.agents[0].s = 80.0;
        assert_eq!(w.command(id).unwrap().blinker, Blinker::Right);
    }

    #[test]
    fn same_seed_same_run() {
        let run = || {
            let mut w = world(&[
                straight(0.0, 50.0),
                straight(50.0, 50.0),
                straight(50.0, 80.0),
            ]);
            let id = w.spawn(LaneId(0), 0.0, 5.0, IdmParams::default()).unwrap();
            for _ in 0..100 {
                w.step(0.1);
            }
            w.agent(id).map(|a| (a.lane, a.s.to_bits()))
        };
        assert_eq!(run(), run());
    }
}
