//! The revert prompt of a risky setting: it counts down, and Keep or Revert ends it.

use bevy::prelude::*;
use opts_gui::l10n::L10nArgs;

use crate::settings_registry::{ChangeSource, SettingsApi};

use super::params;
use super::{PendingRevert, RevertButton};

/// Counts the prompt down; at zero (or *Revert*, or Escape) the old value returns.
pub fn revert_prompt_system(
    mut commands: Commands,
    time: Res<Time<Real>>,
    input: params::RevertInput,
    mut args: Query<&mut L10nArgs>,
    mut api: SettingsApi,
    pending: Option<ResMut<PendingRevert>>,
) {
    let params::RevertInput {
        mut clicks,
        mut modal_events,
        buttons,
    } = input;
    let Some(mut p) = pending else { return };
    let mut keep = false;
    let mut revert = false;
    for c in clicks.read() {
        match buttons.get(c.0) {
            Ok(RevertButton::Keep) => keep = true,
            Ok(RevertButton::Revert) => revert = true,
            Err(_) => {}
        }
    }
    if modal_events.read().next().is_some() {
        revert = true;
    }
    p.remaining -= time.delta_secs();
    if p.remaining <= 0.0 {
        revert = true;
    }
    if let Ok(mut a) = args.get_mut(p.countdown) {
        let secs = p.remaining.ceil().max(0.0) as usize;
        if a.0.first().map(|x| &x.1) != Some(&secs.into()) {
            *a = L10nArgs::one("seconds", secs);
        }
    }
    if !keep && !revert {
        return;
    }
    if keep {
        api.save();
    } else {
        let (key, old) = (p.key.clone(), p.old.clone());
        if let Err(e) = api.change(&key, Some(&old), ChangeSource::Revert, false) {
            tracing::warn!(err = %e, "revert refused");
        }
    }
    commands.entity(p.modal).despawn();
    commands.remove_resource::<PendingRevert>();
}
