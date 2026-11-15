//! Composite and static pieces of the component showcase: combo box, tabs, tree, toast, dialogs.

use bevy::prelude::*;

use crate::components::{
    button::ButtonBuilder,
    card::CardBuilder,
    combobox::ComboboxBuilder,
    date_select::DateSelectBuilder,
    datetime_select::DateTimeSelectBuilder,
    divider::DividerBuilder,
    label::LabelBuilder,
    modal::ModalBuilder,
    notebook::NotebookBuilder,
    toast::ToastBuilder,
    toplevel::ToplevelBuilder,
    treeview::{TreeNode, TreeviewBuilder},
};
use crate::ui_id::UiId;

use super::super::{column, heading, row};

/// Combo box, tabs, tree, toast.
pub(in crate::showcase) fn composite_section(commands: &mut Commands) -> Entity {
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
pub(in crate::showcase) fn misc_section(commands: &mut Commands) -> Entity {
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
