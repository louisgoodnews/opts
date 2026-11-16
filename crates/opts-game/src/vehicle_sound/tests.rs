//! Unit tests of `vehicle_sound.rs`.

mod ai_speed;
mod names;
mod publish;
mod selection;

use super::*;

fn cfg(text: &str) -> SoundConfig {
    SoundConfig::parse(text)
}
