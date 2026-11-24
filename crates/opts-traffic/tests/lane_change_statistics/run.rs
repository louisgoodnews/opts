//! Runs one situation and feeds the observations into the tally.

use std::collections::HashMap;

use opts_traffic::{AgentId, Blinker, TrafficEvent, TrafficWorld};

use super::situation::situation;
use super::tally::{Announce, Tally, Watch};
use super::{
    DT, FOLLOWER_MAX_DECEL, INDICATOR_MIN_S, RECOVERY_S, RECOVERY_SHARE_DESIRED, STEPS,
    UNSAFE_DECEL, WINDOW_S,
};

/// What is observed while the situation runs.
#[derive(Default)]
struct Observer {
    announce: HashMap<AgentId, Announce>,
    watches: Vec<Watch>,
    changed_here: bool,
}

/// Runs seed `seed` for [`STEPS`] steps and adds its changes to `tally`.
pub fn run_situation(seed: u64, tally: &mut Tally) {
    let (mut w, _) = situation(seed);
    let mut obs = Observer::default();
    for _ in 0..STEPS {
        w.step(DT);
        let now = w.clock();
        for e in w.drain_events() {
            if let TrafficEvent::LaneChanged { id, .. } = e {
                on_move(&w, &mut obs, id, now, tally);
            }
        }
        update_indicators(&w, &mut obs.announce, now);
        update_watches(&w, &mut obs.watches, now);
    }
    if obs.changed_here {
        tally.situations_with_change += 1;
    }
    let end = STEPS as f64 * DT;
    for watch in &obs.watches {
        // Only changes whose full observation window fits in the run count.
        if watch.t0 + RECOVERY_S <= end {
            tally.record_watch(watch, seed, FOLLOWER_MAX_DECEL, UNSAFE_DECEL);
        }
    }
}

/// A move by `id` at time `now`: the indicator time before it goes into the tally, and the
/// change is watched from here on.
fn on_move(w: &TrafficWorld, obs: &mut Observer, id: AgentId, now: f64, tally: &mut Tally) {
    obs.changed_here = true;
    tally.changes += 1;
    let st = obs.announce.entry(id).or_default();
    let shown = st.since.map(|on| now - on);
    tally.record_indicator(shown, INDICATOR_MIN_S);
    *st = Announce {
        since: None,
        moved: true,
    };
    let Some(agent) = w.agent(id) else { return };
    let (lane, s) = (agent.lane, agent.s);
    let follower = w
        .agents()
        .iter()
        .filter(|o| o.id != id && o.lane == lane && o.s < s)
        .max_by(|a, b| a.s.total_cmp(&b.s))
        .map(|o| o.id);
    obs.watches.push(Watch {
        changer: id,
        follower,
        t0: now,
        follower_decel: 0.0,
        changer_decel: 0.0,
        recovered_at: None,
        desired: agent.params.desired_speed,
        speed_at_t0: agent.speed,
    });
}

/// Indicator state of every agent: on from the first sight until the move; going off without a
/// move is an abort and clears the announcement.
fn update_indicators(w: &TrafficWorld, announce: &mut HashMap<AgentId, Announce>, now: f64) {
    for id in w.agents().iter().map(|a| a.id).collect::<Vec<_>>() {
        let on = w.command(id).is_some_and(|c| c.blinker != Blinker::Off);
        let st = announce.entry(id).or_default();
        if !on {
            *st = Announce::default();
        } else if !st.moved && st.since.is_none() {
            st.since = Some(now);
        }
    }
}

/// Braking of the new followers and changers within [`WINDOW_S`], and the recovery of the
/// changers within [`RECOVERY_S`], for the changes made so far.
fn update_watches(w: &TrafficWorld, watches: &mut [Watch], now: f64) {
    for watch in watches {
        let elapsed = now - watch.t0;
        if elapsed <= WINDOW_S {
            if let Some(f) = watch.follower.and_then(|f| w.agent(f)) {
                watch.follower_decel = watch.follower_decel.max(-f.accel);
            }
            if let Some(c) = w.agent(watch.changer) {
                watch.changer_decel = watch.changer_decel.max(-c.accel);
            }
        }
        if watch.recovered_at.is_none() && elapsed <= RECOVERY_S {
            if let Some(c) = w.agent(watch.changer) {
                if c.speed >= RECOVERY_SHARE_DESIRED * watch.desired {
                    watch.recovered_at = Some(elapsed);
                }
            }
        }
    }
}
