//! The descriptors of the binding rows and the reset rows the settings screen lists.

use bevy::prelude::*;
use opts_bevy::action_keys::{self};

use crate::settings_registry::descriptor::{
    Category as C, SettingDescriptor as D, SettingKind as K,
};

use super::{MOUSE_DRIVING_KEY};

pub(super) const RESET_ROWS: [(&str, &str, &str); 7] = [
    (
        "controls.reset.all",
        "set-reset-keys-all",
        "set-reset-keys-all-desc",
    ),
    (
        "controls.reset.vehicle",
        "set-reset-keys-vehicle",
        "set-reset-keys-vehicle-desc",
    ),
    (
        "controls.reset.rail",
        "set-reset-keys-rail",
        "set-reset-keys-rail-desc",
    ),
    (
        "controls.reset.drive",
        "set-reset-keys-drive",
        "set-reset-keys-drive-desc",
    ),
    (
        "controls.reset.camera",
        "set-reset-keys-camera",
        "set-reset-keys-camera-desc",
    ),
    (
        "controls.reset.map",
        "set-reset-keys-map",
        "set-reset-keys-map-desc",
    ),
    (
        "controls.reset.ui",
        "set-reset-keys-ui",
        "set-reset-keys-ui-desc",
    ),
];

/// Message key of an action's label and description.
pub(super) fn label_keys(id: &str) -> (&'static str, &'static str) {
    // Leaked once per action at registration: the registry wants `'static` keys.
    let l: &'static str = Box::leak(format!("set-bk-{id}").into_boxed_str());
    let d: &'static str = Box::leak(format!("set-bk-{id}-desc").into_boxed_str());
    (l, d)
}

pub(super) fn leak(text: String) -> &'static str {
    Box::leak(text.into_boxed_str())
}

/// The descriptors: one binding row per built-in action and the reset rows.
#[must_use]
pub fn descriptors() -> Vec<D> {
    let mut v: Vec<D> = action_keys::ACTIONS
        .iter()
        .enumerate()
        .map(|(i, a)| {
            let (label, desc) = label_keys(a.id);
            D::new(
                leak(action_keys::setting_of(a)),
                K::Binding,
                a.default,
                C::Controls,
                200 + i as u16,
                label,
                desc,
            )
            .keywords(Box::leak(
                vec!["key", "keyboard", "binding", a.group].into_boxed_slice(),
            ))
        })
        .collect();
    v.push(
        D::new(
            MOUSE_DRIVING_KEY,
            K::Bool,
            "true",
            C::Controls,
            190,
            "set-mouse-driving",
            "set-mouse-driving-desc",
        )
        .keywords(&["mouse", "wheel", "cursor", "steer", "pedals"]),
    );
    v.extend(
        RESET_ROWS
            .iter()
            .enumerate()
            .map(|(i, (key, label, desc))| {
                D::new(key, K::Action, "", C::Controls, 400 + i as u16, label, desc)
                    .keywords(&["key", "keyboard", "binding", "reset", "default"])
            }),
    );
    v
}
