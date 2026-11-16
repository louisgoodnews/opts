//! Who goes first where two flows meet.
//!
//! OMSI marks junction paths with a `priority` rule (corpus values `192` and `64`; the higher
//! number has the right of way — the paths with `192` are 84 % straight ones, those with `64` only
//! 52 %: through traffic against turning and side-road traffic). Without a rule the AI follows
//! right before left (community wiki, `docs/src/formats/map/tiles.md`). The order, first match:
//!
//! 1. **Priority rules**: a lane with a higher `priority` goes first; a lane without a rule ranks
//!    between `64` and `192` ([`UNMARKED_PRIORITY`] ❓).
//! 2. **Left turns** (StVO §9 (3)): a vehicle turning left gives way to oncoming traffic going
//!    straight or turning right.
//! 3. **Right before left** (StVO §8 (1)): of two flows crossing at an angle, the one coming from
//!    the other's right goes first.
//! 4. Anything else — merging lanes, nearly parallel flows — is [`Precedence::Equal`] and the
//!    vehicles take turns by arrival time (zipper merge).
//!
//! The rules decide who must *yield*; a vehicle already inside the zone is always respected.

use crate::conflict::{Conflict, ConflictKind};
use crate::graph::{LaneGraph, LaneId};

/// Rank of a lane without a `priority` rule, between the two corpus values.
pub const UNMARKED_PRIORITY: u32 = 128;
/// Turns smaller than this (rad) count as going straight.
const TURN_THRESHOLD: f64 = 20.0 * std::f64::consts::PI / 180.0;
/// Approach directions this close to opposite (rad) count as oncoming.
const ONCOMING_TOLERANCE: f64 = 35.0 * std::f64::consts::PI / 180.0;
/// Crossings flatter than this (rad) between the flows have no "from the right".
const MIN_CROSSING: f64 = 20.0 * std::f64::consts::PI / 180.0;

/// The outcome of a right-of-way check for lane A against lane B.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Precedence {
    /// A goes first: B has to yield.
    First,
    /// B goes first: A has to yield.
    Second,
    /// No rule decides; arrival time does.
    Equal,
}

fn wrap(a: f64) -> f64 {
    (a + std::f64::consts::PI).rem_euclid(std::f64::consts::TAU) - std::f64::consts::PI
}

/// Precedence of `lane` over `c.other` in the conflict zone `c` (which belongs to `lane`).
#[must_use]
pub fn precedence(graph: &LaneGraph, lane: LaneId, c: &Conflict) -> Precedence {
    let (Some(a), Some(b)) = (graph.lane(lane), graph.lane(c.other)) else {
        return Precedence::Equal;
    };
    if c.kind == ConflictKind::Merge {
        return Precedence::Equal;
    }
    // 1. Priority rules.
    let (pa, pb) = (
        a.rules.priority.unwrap_or(UNMARKED_PRIORITY),
        b.rules.priority.unwrap_or(UNMARKED_PRIORITY),
    );
    if pa != pb {
        return if pa > pb {
            Precedence::First
        } else {
            Precedence::Second
        };
    }
    // 2. Left turners yield to oncoming traffic that goes straight or right.
    let oncoming = wrap(a.geometry.heading - b.geometry.heading).abs()
        > std::f64::consts::PI - ONCOMING_TOLERANCE;
    if oncoming {
        let (ta, tb) = (a.turn(), b.turn());
        let (a_left, b_left) = (ta < -TURN_THRESHOLD, tb < -TURN_THRESHOLD);
        if a_left && !b_left {
            return Precedence::Second;
        }
        if b_left && !a_left {
            return Precedence::First;
        }
        return Precedence::Equal;
    }
    // 3. Right before left, judged by the flows' headings where they meet.
    let ha = a.geometry.heading_at((c.enter + c.exit) / 2.0);
    let hb = b.geometry.heading_at((c.other_enter + c.other_exit) / 2.0);
    let d = wrap(hb - ha);
    if d.abs() < MIN_CROSSING || d.abs() > std::f64::consts::PI - MIN_CROSSING {
        return Precedence::Equal;
    }
    // B comes from A's right when it heads against A's right-hand side: relative heading in
    // (−180°, 0°) with compass angles (clockwise positive) — e.g. A north, B west-bound = −90°.
    if d < 0.0 {
        Precedence::Second
    } else {
        Precedence::First
    }
}

#[cfg(test)]
mod tests;
