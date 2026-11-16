//! The player's voices over a real script engine and a real (temporary) sound file.

use std::sync::Arc;

use bevy::prelude::*;
use opts_bevy::audio::SoundEmitter;
use opts_bevy::{omsi_to_bevy_pos, MainCamera, VehicleScripts};

use crate::audio_content::ClipLoader;
use crate::sound_logic::playback_of;

use super::*;
use opts_bevy::audio::AudioStats;
use opts_core::formats::sound::SoundConfig;
use opts_sim::script::ScriptEngine;

mod cabin;
mod loops;
mod rig;
use rig::*;
