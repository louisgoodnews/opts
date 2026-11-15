//! Applies the page's changes: a risky change opens the revert prompt, the others are saved at once.

use bevy::prelude::*;
use opts_gui::components::button::ButtonBuilder;
use opts_gui::components::modal::ModalBuilder;
use opts_gui::interaction::focus::DefaultFocus;
use opts_gui::l10n::L10nArgs;
use opts_gui::prelude::*;

use crate::settings_registry::{Apply, ChangeSource, SettingKind, SettingsApi};
use crate::settings_screen::rows::label;

use super::{PendingRevert, RevertButton};

pub(super) fn step(
    d: &crate::settings_registry::SettingDescriptor,
    cur: &str,
    dir: i32,
) -> Option<String> {
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
pub(super) fn change_from_page(
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
