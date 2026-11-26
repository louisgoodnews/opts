//! `opts-challenge-scorer`: the scorer of OPTS challenges (M-0172, ISS-048). The host feeds run
//! events to a [`Scorer`] and [`Scorer::finish`] returns the `ChallengeResult`. Split out of
//! `opts-challenge` (ISS-089), which re-exports it; the model is in `opts-challenge-model`.

#![warn(missing_docs)]
#![deny(unsafe_code)]

pub mod scorer;

pub use scorer::Scorer;
