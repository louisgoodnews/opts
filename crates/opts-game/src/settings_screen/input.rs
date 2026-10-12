//! Input of the settings page: what the controls do, and the revert prompt of risky settings.

use bevy::prelude::*;
use opts_gui::components::button::ButtonBuilder;
use opts_gui::components::modal::{ModalBuilder, ModalEvent};
use opts_gui::interaction::events::{ButtonClicked, SwitchToggled, TextChanged};
use opts_gui::interaction::focus::DefaultFocus;
use opts_gui::l10n::L10nArgs;
use opts_gui::prelude::*;

use crate::settings_registry::{Apply, Category, ChangeSource, SettingKind, SettingsApi};

use super::page::{SettingsPageState, SettingsSearch};
use super::rows::label;

/// What a control of the page does when activated.
#[derive(Component, Debug, Clone, PartialEq, Eq)]
pub enum SettingOp {
    /// Sets a value.
    Set {
        /// Setting key.
        key: &'static str,
        /// Stored text.
        value: String,
    },
    /// Steps a number or time up or down.
    Step {
        /// Setting key.
        key: &'static str,
        /// -1 or 1.
        dir: i32,
    },
    /// Restores the default.
    Reset {
        /// Setting key.
        key: &'static str,
    },
    /// Starts an action or a chooser: announced as [`SettingActionRequested`].
    Run {
        /// Setting key.
        key: &'static str,
    },
    /// Shows a category.
    Category(Category),
}

/// A switch bound to a boolean setting.
#[derive(Component, Debug, Clone, Copy)]
pub struct SettingSwitch(pub &'static str);

/// An action button or path chooser of the page was used; the owner of the feature handles it.
#[derive(Message, Debug, Clone, Copy, PartialEq, Eq)]
pub struct SettingActionRequested {
    /// The key of the action or path setting.
    pub key: &'static str,
}

/// The risky change waiting for the user's confirmation.
#[derive(Resource, Debug)]
pub struct PendingRevert {
    /// Setting key.
    pub key: String,
    /// Value to return to.
    pub old: String,
    /// The value being tried.
    pub new: String,
    /// Seconds left.
    pub remaining: f32,
    /// The modal.
    pub modal: Entity,
    /// The countdown text.
    pub countdown: Entity,
}

/// Buttons of the revert prompt.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub enum RevertButton {
    /// Keep the new value.
    Keep,
    /// Go back.
    Revert,
}

fn step(d: &crate::settings_registry::SettingDescriptor, cur: &str, dir: i32) -> Option<String> {
    match d.kind {
        SettingKind::Int { min, max, step } => {
            let v: i64 = cur.parse().ok()?;
            let n = (v + step * i64::from(dir)).clamp(min, max);
            (n != v).then(|| n.to_string())
        }
        SettingKind::Float { min, max, step } => {
            let v: f64 = cur.parse().ok()?;
            let n = (v + step * f64::from(dir)).clamp(min, max);
            ((n - v).abs() > f64::EPSILON).then(|| n.to_string())
        }
        SettingKind::Time => {
            let (h, m) = cur.split_once(':')?;
            let total = h.parse::<i32>().ok()? * 60 + m.parse::<i32>().ok()?;
            let n = (total + 15 * dir).rem_euclid(24 * 60);
            Some(format!("{:02}:{:02}", n / 60, n % 60))
        }
        _ => None,
    }
}

/// Applies a page change; risky settings open the revert prompt instead of being saved.
fn change_from_page(
    api: &mut SettingsApi,
    commands: &mut Commands,
    pending: &mut Option<ResMut<PendingRevert>>,
    key: &str,
    value: Option<&str>,
) {
    let apply = api.registry.get(key).map(|d| d.apply);
    let Some(Apply::WithRevert(secs)) = apply.filter(|_| pending.is_none()) else {
        if let Err(e) = api.change(key, value, ChangeSource::Page, true) {
            tracing::warn!(err = %e, "settings page change refused");
        }
        return;
    };
    match api.change(key, value, ChangeSource::Page, false) {
        Ok(c) if c.old != c.new => open_prompt(commands, c.key, c.old, c.new, secs),
        Ok(_) => {}
        Err(e) => tracing::warn!(err = %e, "settings page change refused"),
    }
}

fn open_prompt(commands: &mut Commands, key: String, old: String, new: String, secs: u32) {
    let (root, content) = ModalBuilder::new()
        .with_size_percent(60.0, 34.0)
        .with_width_range(320.0, 520.0)
        .spawn(commands);
    commands
        .entity(root)
        .insert(UiId::new("settings/revert-prompt"));
    label(
        commands,
        content,
        "settings-keep-title",
        TypeScale::TitleLarge,
        ColorRole::OnSurface,
    );
    let countdown = label(
        commands,
        content,
        "settings-keep-body",
        TypeScale::BodyLarge,
        ColorRole::OnSurfaceVariant,
    );
    commands
        .entity(countdown)
        .insert(L10nArgs::one("seconds", secs as usize));
    let row = commands
        .spawn((
            Node {
                column_gap: Val::Px(12.0),
                flex_wrap: FlexWrap::Wrap,
                row_gap: Val::Px(8.0),
                ..default()
            },
            ChildOf(content),
        ))
        .id();
    for (which, key_l, id) in [
        (RevertButton::Revert, "settings-revert", "revert"),
        (RevertButton::Keep, "settings-keep", "keep"),
    ] {
        let b = if which == RevertButton::Keep {
            ButtonBuilder::filled(key_l)
        } else {
            ButtonBuilder::outlined(key_l)
        }
        .with_key(key_l)
        .spawn(commands);
        commands.entity(b).insert((
            which,
            UiId::new(format!("settings/revert-prompt/{id}")),
            ChildOf(row),
        ));
        if which == RevertButton::Revert {
            commands.entity(b).insert(DefaultFocus);
        }
    }
    commands.insert_resource(PendingRevert {
        key,
        old,
        new,
        remaining: secs as f32,
        modal: root,
        countdown,
    });
}

/// Handles clicks, switches and the search field.
#[allow(clippy::too_many_arguments)]
pub fn handle_page_input(
    mut commands: Commands,
    mut clicks: MessageReader<ButtonClicked>,
    mut toggles: MessageReader<SwitchToggled>,
    mut texts: MessageReader<TextChanged>,
    ops: Query<&SettingOp>,
    switches: Query<&SettingSwitch>,
    search: Query<(), With<SettingsSearch>>,
    mut state: ResMut<SettingsPageState>,
    mut api: SettingsApi,
    mut pending: Option<ResMut<PendingRevert>>,
    mut actions: MessageWriter<SettingActionRequested>,
) {
    for t in texts.read() {
        if search.contains(t.entity) && state.query != t.text {
            state.query.clone_from(&t.text);
        }
    }
    for t in toggles.read() {
        if let Ok(s) = switches.get(t.entity) {
            let v = if t.is_on { "true" } else { "false" };
            change_from_page(&mut api, &mut commands, &mut pending, s.0, Some(v));
        }
    }
    for c in clicks.read() {
        let Ok(op) = ops.get(c.0) else { continue };
        match op {
            SettingOp::Set { key, value } => {
                change_from_page(&mut api, &mut commands, &mut pending, key, Some(value));
            }
            SettingOp::Step { key, dir } => {
                let next = api
                    .registry
                    .get(key)
                    .and_then(|d| step(d, &api.text(key), *dir));
                if let Some(v) = next {
                    change_from_page(&mut api, &mut commands, &mut pending, key, Some(&v));
                }
            }
            SettingOp::Reset { key } => {
                change_from_page(&mut api, &mut commands, &mut pending, key, None);
            }
            SettingOp::Run { key } => {
                actions.write(SettingActionRequested { key });
            }
            SettingOp::Category(cat) => {
                if state.category != *cat || !state.query.is_empty() {
                    state.category = *cat;
                    state.query.clear();
                    state.clear_search_field = true;
                }
            }
        }
    }
}

/// Counts the prompt down; at zero (or *Revert*, or Escape) the old value returns.
#[allow(clippy::too_many_arguments)]
pub fn revert_prompt_system(
    mut commands: Commands,
    time: Res<Time<Real>>,
    mut clicks: MessageReader<ButtonClicked>,
    mut modal_events: MessageReader<ModalEvent>,
    buttons: Query<&RevertButton>,
    mut args: Query<&mut L10nArgs>,
    mut api: SettingsApi,
    pending: Option<ResMut<PendingRevert>>,
) {
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
