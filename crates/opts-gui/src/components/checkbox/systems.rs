//! Systems of the checkbox: toggling on activation and syncing the indicator visuals.

use bevy::prelude::*;

use super::{CheckboxBuilder, CheckboxGlyph, CheckboxIndicator, M3Checkbox};
use crate::interaction::{activation::Activated, events::CheckboxToggled, state::{CheckState, Disabled}};
use crate::tokens::components::{BackgroundRole, BorderRole, ForegroundRole};

/// System toggling a checkbox when it is [`Activated`].
pub fn checkbox_toggle_system(
    mut activated: MessageReader<Activated>,
    mut checkboxes: Query<&mut M3Checkbox, Without<Disabled>>,
    mut events: MessageWriter<CheckboxToggled>,
) {
    for Activated(entity) in activated.read() {
        if let Ok(mut checkbox) = checkboxes.get_mut(*entity) {
            checkbox.state = match checkbox.state {
                CheckState::Unchecked => CheckState::Checked,
                CheckState::Checked => CheckState::Unchecked,
                CheckState::Indeterminate => CheckState::Checked,
            };
            events.write(CheckboxToggled {
                entity: *entity,
                state: checkbox.state,
            });
        }
    }
}

/// System synchronizing checkbox visual indicator nodes when checkbox state changes.
pub fn checkbox_sync_system(
    changed_query: Query<(Entity, &M3Checkbox), Changed<M3Checkbox>>,
    mut indicator_query: Query<(&CheckboxIndicator, &mut BackgroundRole, &mut BorderRole)>,
    mut glyph_query: Query<(&CheckboxGlyph, &mut Text, &mut ForegroundRole)>,
) {
    for (entity, checkbox) in &changed_query {
        let (bg, border, fg) = CheckboxBuilder::indicator_roles(checkbox.state);
        let glyph_str = CheckboxBuilder::glyph_char(checkbox.state);

        for (indicator, mut bg_role, mut border_role) in &mut indicator_query {
            if indicator.0 == entity {
                bg_role.0 = bg;
                border_role.0 = border;
            }
        }

        for (glyph, mut text, mut fg_role) in &mut glyph_query {
            if glyph.0 == entity {
                text.0 = glyph_str.to_string();
                fg_role.0 = fg;
            }
        }
    }
}
