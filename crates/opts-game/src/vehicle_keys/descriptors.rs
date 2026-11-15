//! The settings rows of the vehicle key actions, one for buses and one for rail vehicles.

use crate::settings_registry::{Category, SettingDescriptor, SettingKind};

use super::ACTIONS;

/// The settings, one `Binding` row each, in the Controls category.
#[must_use]
pub fn descriptors() -> Vec<SettingDescriptor> {
    let mut rows: Vec<SettingDescriptor> = ACTIONS
        .iter()
        .enumerate()
        .map(|(i, a)| {
            SettingDescriptor::new(
                a.setting,
                SettingKind::Binding,
                a.default,
                Category::Controls,
                100 + i as u16,
                a.label_key,
                a.desc_key,
            )
            .keywords(&["key", "keyboard", "binding", "vehicle", "bus"])
        })
        .collect();
    // The same actions with the keys of rail vehicles.
    rows.extend(ACTIONS.iter().enumerate().filter_map(|(i, a)| {
        Some(
            SettingDescriptor::new(
                a.rail_setting?,
                SettingKind::Binding,
                a.default,
                Category::Controls,
                300 + i as u16,
                rail_label_key(a.id),
                rail_desc_key(a.id),
            )
            .keywords(&[
                "key", "keyboard", "binding", "vehicle", "tram", "train", "rail",
            ]),
        )
    }));
    rows
}

/// Message key of the label of an action's rail-vehicle row.
fn rail_label_key(id: &str) -> &'static str {
    Box::leak(format!("set-vkr-{id}").into_boxed_str())
}

/// Message key of the description of an action's rail-vehicle row.
fn rail_desc_key(id: &str) -> &'static str {
    Box::leak(format!("set-vkr-{id}-desc").into_boxed_str())
}
