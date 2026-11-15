//! Toggles and text fields of the component showcase: check box, switch, radio, fields.

use bevy::prelude::*;

use crate::components::{
    checkbox::CheckboxBuilder, radio::RadioBuilder, switch::SwitchBuilder,
    text_field::TextFieldBuilder,
};
use crate::interaction::state::{CheckState, DisabledReason, ErrorState};
use crate::ui_id::UiId;

use super::super::{column, heading, row};

/// Check box, switch, radio.
pub(in crate::showcase) fn toggle_section(commands: &mut Commands) -> Entity {
    let toggles = column(commands, 8.0);
    let mut rows = vec![heading(commands, "Check box, switch, radio")];
    let line = row(commands, 16.0);
    let mut cells = Vec::new();
    for (name, state) in [
        ("unchecked", CheckState::Unchecked),
        ("checked", CheckState::Checked),
        ("mixed", CheckState::Indeterminate),
    ] {
        let c = CheckboxBuilder::new(name).with_state(state).spawn(commands);
        commands
            .entity(c)
            .insert(UiId::new(format!("showcase.checkbox.{name}")));
        cells.push(c);
    }
    let c = CheckboxBuilder::new("disabled")
        .disabled(true)
        .spawn(commands);
    commands.entity(c).insert((
        UiId::new("showcase.checkbox.disabled"),
        DisabledReason::new("Needs a saved game"),
    ));
    cells.push(c);
    let c = CheckboxBuilder::new("error").spawn(commands);
    commands.entity(c).insert((
        UiId::new("showcase.checkbox.error"),
        ErrorState::new("Required"),
    ));
    cells.push(c);
    commands.entity(line).add_children(&cells);
    rows.push(line);
    let line = row(commands, 16.0);
    let on = SwitchBuilder::new("on").with_value(true).spawn(commands);
    let off = SwitchBuilder::new("off").spawn(commands);
    let dis = SwitchBuilder::new("disabled")
        .disabled(true)
        .spawn(commands);
    commands.entity(on).insert(UiId::new("showcase.switch.on"));
    commands
        .entity(off)
        .insert(UiId::new("showcase.switch.off"));
    commands.entity(dis).insert((
        UiId::new("showcase.switch.disabled"),
        DisabledReason::new("Needs a saved game"),
    ));
    let r1 = RadioBuilder::new("one")
        .with_group(1)
        .selected(true)
        .spawn(commands);
    let r2 = RadioBuilder::new("two").with_group(1).spawn(commands);
    let r3 = RadioBuilder::new("three")
        .with_group(1)
        .disabled(true)
        .spawn(commands);
    commands.entity(r1).insert(UiId::new("showcase.radio.one"));
    commands.entity(r2).insert(UiId::new("showcase.radio.two"));
    commands.entity(r3).insert((
        UiId::new("showcase.radio.three"),
        DisabledReason::new("Needs a saved game"),
    ));
    commands
        .entity(line)
        .add_children(&[on, off, dis, r1, r2, r3]);
    rows.push(line);
    commands.entity(toggles).add_children(&rows);

    toggles
}

/// Text fields.
pub(in crate::showcase) fn field_section(commands: &mut Commands) -> Entity {
    let fields = column(commands, 8.0);
    let mut rows = vec![heading(
        commands,
        "Text field: empty, filled, error, disabled",
    )];
    let line = row(commands, 16.0);
    let empty = TextFieldBuilder::outlined("Search")
        .with_placeholder("Search 956 vehicles...")
        .spawn(commands);
    let filled = TextFieldBuilder::filled("Route")
        .with_text("M29")
        .spawn(commands);
    let error = TextFieldBuilder::outlined("Name")
        .with_text("???")
        .spawn(commands);
    let disabled = TextFieldBuilder::outlined("Locked")
        .with_text("read only")
        .spawn(commands);
    commands
        .entity(empty)
        .insert(UiId::new("showcase.field.empty"));
    commands
        .entity(filled)
        .insert(UiId::new("showcase.field.filled"));
    commands.entity(error).insert((
        UiId::new("showcase.field.error"),
        ErrorState::new("Not a valid name"),
    ));
    commands.entity(disabled).insert((
        UiId::new("showcase.field.disabled"),
        crate::interaction::state::Disabled,
        DisabledReason::new("Set by the map"),
    ));
    commands
        .entity(line)
        .add_children(&[empty, filled, error, disabled]);
    rows.push(line);
    commands.entity(fields).add_children(&rows);

    fields
}
