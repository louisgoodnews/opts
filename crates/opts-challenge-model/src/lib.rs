//! `opts-challenge-model`: the challenge model of OPTS (M-0172, ISS-048): the rules, the challenge,
//! the run events and the result. Split out of `opts-challenge` (ISS-089), which re-exports it.
//!
//! A [`Challenge`] is a set of [`ScoringRules`] and [`PassCriteria`]. The scorer (in
//! `opts-challenge-scorer`) applies run events to it and produces a [`ChallengeResult`].
//!
//! The crate reads no engine or simulation type and has no dependencies. The challenge rules are
//! OPTS's own (ADR-0002, clean-room policy).

#![warn(missing_docs)]
#![deny(unsafe_code)]

pub mod challenge;
pub mod event;
pub mod outcome;
pub mod rules;

pub use challenge::{punctual_run, tight_schedule, Challenge, PassCriteria};
pub use event::ChallengeEvent;
pub use outcome::{ChallengeResult, Outcome, ResultSink};
pub use rules::ScoringRules;
