//! `opts-challenge`: the OPTS challenge model and its scoring (M-0172, ISS-048).
//!
//! A [`Challenge`] is a set of [`ScoringRules`] and [`PassCriteria`]. The host turns its own
//! simulation facts (stops, collisions, passengers) into [`ChallengeEvent`]s and feeds them to a
//! [`Scorer`]. When the run ends, [`Scorer::finish`] returns a [`ChallengeResult`], which a
//! [`ResultSink`] may store (the career system, ISS-047, implements the sink; this crate does not).
//!
//! The crate reads no engine or simulation type, so the host owns the mapping and the rules stay
//! testable on their own. The challenge rules are OPTS's own (ADR-0002, clean-room policy).

#![warn(missing_docs)]
#![deny(unsafe_code)]

pub mod challenge;
pub mod event;
pub mod outcome;
pub mod rules;
pub mod scorer;

pub use challenge::{punctual_run, tight_schedule, Challenge, PassCriteria};
pub use event::ChallengeEvent;
pub use outcome::{ChallengeResult, Outcome, ResultSink};
pub use rules::ScoringRules;
pub use scorer::Scorer;
