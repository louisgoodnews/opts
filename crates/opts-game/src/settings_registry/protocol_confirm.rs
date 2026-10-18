//! Confirm and revert for risky settings through the control protocol (M-0072).
//!
//! The settings page opens a 15 s prompt for a setting that can make the screen unusable (window
//! mode and size). `settings.set` and `settings.reset` apply and save at once, as before; with
//! `confirm: false` they behave like the page: the value is applied but not saved, and
//! `settings.confirm` keeps it while `settings.revert` (or the timeout) restores the old one.
//! Only one risky change can wait at a time, shared with the page's own prompt.

use bevy::prelude::*;
use opts_bevy::control::state::Outcome;
use opts_cli::protocol::{code, RpcError};
use serde_json::{json, Value};

use super::{apply_change_with, ChangeSource};
use crate::settings::GameSettings;

/// The risky change that waits for `settings.confirm`.
#[derive(Resource, Debug, Clone, PartialEq)]
pub struct ProtocolRevert {
    /// Setting key.
    pub key: String,
    /// Value to return to.
    pub old: String,
    /// The value being tried.
    pub new: String,
    /// Seconds left before it is reverted.
    pub remaining: f32,
}

/// Whether a risky change already waits (this one or the settings page's prompt).
#[must_use]
pub fn is_pending(world: &World) -> bool {
    world.contains_resource::<ProtocolRevert>()
        || world.contains_resource::<crate::settings_screen::PendingRevert>()
}

/// Starts waiting for the confirmation of `key`.
pub(super) fn begin(world: &mut World, key: &str, old: &str, new: &str, seconds: u32) {
    world.insert_resource(ProtocolRevert {
        key: key.to_string(),
        old: old.to_string(),
        new: new.to_string(),
        remaining: seconds as f32,
    });
}

fn nothing_waits() -> Outcome {
    Outcome::err(RpcError::new(
        code::NOT_READY,
        "no risky setting change waits for confirmation",
    ))
}

/// `settings.confirm`: keeps the value being tried and saves it.
pub(crate) fn confirm(world: &mut World, _: &Value) -> Outcome {
    let Some(p) = world.remove_resource::<ProtocolRevert>() else {
        return nothing_waits();
    };
    world.resource_mut::<GameSettings>().save_in_background();
    Outcome::ok(json!({ "key": p.key, "value": p.new, "confirmed": true }))
}

/// `settings.revert`: restores the old value now.
pub(crate) fn revert(world: &mut World, _: &Value) -> Outcome {
    let Some(p) = world.remove_resource::<ProtocolRevert>() else {
        return nothing_waits();
    };
    match restore(world, &p) {
        Ok(()) => Outcome::ok(json!({ "key": p.key, "value": p.old, "reverted": true })),
        Err(e) => Outcome::err(RpcError::new(code::FAILED, e)),
    }
}

fn restore(world: &mut World, p: &ProtocolRevert) -> Result<(), String> {
    apply_change_with(world, &p.key, Some(&p.old), ChangeSource::Revert, false)
        .map(|_| ())
        .map_err(|e| e.to_string())
}

/// Counts the wait down by `seconds` and reverts when it is over. Returns whether it reverted.
pub fn tick_by(world: &mut World, seconds: f32) -> bool {
    let Some(mut p) = world.get_resource_mut::<ProtocolRevert>() else {
        return false;
    };
    p.remaining -= seconds;
    if p.remaining > 0.0 {
        return false;
    }
    let Some(p) = world.remove_resource::<ProtocolRevert>() else {
        return false;
    };
    if let Err(e) = restore(world, &p) {
        tracing::warn!(err = %e, key = %p.key, "timed-out settings change could not be reverted");
    }
    true
}

/// The timeout of a waiting change (real time, so a paused simulation does not stop it).
pub fn tick_protocol_revert(world: &mut World) {
    let dt = world
        .get_resource::<Time<Real>>()
        .map_or(0.0, Time::delta_secs);
    tick_by(world, dt);
}
