//! Sections of the component showcase (toggles, fields, composites, dialogs).

use bevy::prelude::*;

use crate::components::{
    button::ButtonBuilder,
    card::CardBuilder,
    checkbox::CheckboxBuilder,
    combobox::ComboboxBuilder,
    date_select::DateSelectBuilder,
    datetime_select::DateTimeSelectBuilder,
    divider::DividerBuilder,
    label::LabelBuilder,
    modal::ModalBuilder,
    notebook::NotebookBuilder,
    radio::RadioBuilder,
    switch::SwitchBuilder,
    text_field::TextFieldBuilder,
    toast::ToastBuilder,
    toplevel::ToplevelBuilder,
    treeview::{TreeNode, TreeviewBuilder},
};
use crate::interaction::state::{CheckState, DisabledReason, ErrorState};
use crate::ui_id::UiId;

use super::{column, heading, row};

/// Check box, switch, radio.
pub(super) fn toggle_section(commands: &mut Commands) -> Entity {
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
pub(super) fn field_section(commands: &mut Commands) -> Entity {
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

/// Combo box, tabs, tree, toast.
pub(super) fn composite_section(commands: &mut Commands) -> Entity {
    let composite = column(commands, 8.0);
    let mut rows = vec![heading(commands, "Combo box, tabs, tree, toast")];
    let combo = ComboboxBuilder::new("Route")
        .with_options(vec![
            "Line 100".into(),
            "Line 200".into(),
            "Line 300".into(),
        ])
        .with_selected(1)
        .spawn(commands);
    commands
        .entity(combo)
        .insert(UiId::new("showcase.combobox"));
    let tabs = NotebookBuilder::new()
        .with_tabs(vec!["Timetable".into(), "Fleet".into(), "Staff".into()])
        .spawn(commands);
    commands.entity(tabs).insert(UiId::new("showcase.tabs"));
    let tree = TreeviewBuilder::new()
        .with_node(
            TreeNode::new("net", "Berlin network")
                .expanded(true)
                .with_child(TreeNode::new("m29", "Line M29"))
                .with_child(TreeNode::new("100", "Line 100")),
        )
        .spawn(commands);
    commands.entity(tree).insert(UiId::new("showcase.tree"));
    let toast = ToastBuilder::new("Timetable saved")
        .with_duration(3600.0)
        .with_action("UNDO")
        .spawn(commands);
    commands.entity(toast).insert(UiId::new("showcase.toast"));
    commands.entity(toast).insert(Node {
        position_type: PositionType::Relative,
        padding: UiRect::all(Val::Px(12.0)),
        column_gap: Val::Px(16.0),
        align_items: AlignItems::Center,
        ..default()
    });
    rows.extend([combo, tabs, tree, toast]);
    commands.entity(composite).add_children(&rows);

    composite
}

/// Static pieces and dialogs.
pub(super) fn misc_section(commands: &mut Commands) -> Entity {
    let misc = column(commands, 8.0);
    let mut rows = vec![heading(commands, "Card, divider, date pickers, dialogs")];
    let card = CardBuilder::outlined().spawn(commands);
    let card_text = LabelBuilder::new("A card holds related content.").spawn(commands);
    commands.entity(card).add_child(card_text);
    let divider = DividerBuilder::horizontal().spawn(commands);
    let date = DateSelectBuilder::new(2026, 10, 12).spawn(commands);
    let datetime = DateTimeSelectBuilder::new(2026, 10, 12, 8, 30).spawn(commands);
    let line = row(commands, 12.0);
    let modal = ButtonBuilder::filled("Open dialog")
        .on_click(|cmd, _| {
            let (root, content) = ModalBuilder::new().spawn(cmd);
            let text =
                LabelBuilder::new("Dialogs trap focus; Esc asks the owner to cancel.").spawn(cmd);
            let close = ButtonBuilder::filled("Close")
                .on_click(move |cmd, _| {
                    cmd.entity(root).despawn();
                })
                .spawn(cmd);
            cmd.entity(close).insert(UiId::new("showcase.dialog.close"));
            cmd.entity(content).add_children(&[text, close]);
        })
        .spawn(commands);
    commands
        .entity(modal)
        .insert(UiId::new("showcase.dialog.open"));
    let window = ButtonBuilder::tonal("Open window")
        .on_click(|cmd, _| {
            ToplevelBuilder::new("A floating window")
                .with_size(360.0, 200.0)
                .spawn(cmd);
        })
        .spawn(commands);
    commands
        .entity(window)
        .insert(UiId::new("showcase.window.open"));
    commands.entity(line).add_children(&[modal, window]);
    rows.extend([card, divider, date, datetime, line]);
    commands.entity(misc).add_children(&rows);

    misc
}
