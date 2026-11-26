//! `opts-challenge`: facade over `opts-challenge-model` (the challenge model) and
//! `opts-challenge-scorer` (the scorer), split in ISS-089. Consumer paths are unchanged.

pub use opts_challenge_model::*;
pub use opts_challenge_scorer::{scorer, Scorer};
