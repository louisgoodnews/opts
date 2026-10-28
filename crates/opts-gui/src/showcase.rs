//! Every component in every state, as one UI: the content of the `gallery` example, the fixture of
//! the CI rule tests and the subject of the state screenshots. Demo text only; none of it is
//! product UI, so this file is exempt from the string-literal rule.

use bevy::prelude::*;

use crate::components::{button::ButtonBuilder, label::LabelBuilder};
use crate::interaction::{layer::ForcedState, state::DisabledReason, tooltip::Tooltip};
use crate::not_yet::{NotAvailableYet, NAV_LIBRARY};
use crate::theme::typography::TypeScale;
use crate::ui_id::UiId;

mod sections;
use sections::*;

/// The states of one control that the gallery shows side by side.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Pin {
    /// Nothing pinned.
    Rest,
    /// Hover layer.
    Hover,
    /// Focus layer and ring.
    Focus,
    /// Pressed layer.
    Pressed,
    /// Disabled with a reason.
    Disabled,
    /// Marked "Feature not available yet!".
    NotYet,
}

impl Pin {
    /// All pins in display order.
    pub const ALL: [Pin; 6] = [
        Pin::Rest,
        Pin::Hover,
        Pin::Focus,
        Pin::Pressed,
        Pin::Disabled,
        Pin::NotYet,
    ];

    fn name(self) -> &'static str {
        match self {
            Pin::Rest => "rest",
            Pin::Hover => "hover",
            Pin::Focus => "focus",
            Pin::Pressed => "pressed",
            Pin::Disabled => "disabled",
            Pin::NotYet => "not-yet",
        }
    }

    /// Applies the pin to a control.
    pub fn apply(self, commands: &mut Commands, entity: Entity) {
        let mut e = commands.entity(entity);
        match self {
            Pin::Rest => {}
            Pin::Hover => {
                e.insert(ForcedState {
                    hovered: true,
                    ..default()
                });
            }
            Pin::Focus => {
                e.insert(ForcedState {
                    focused: true,
                    ..default()
                });
            }
            Pin::Pressed => {
                e.insert(ForcedState {
                    pressed: true,
                    ..default()
                });
            }
            Pin::Disabled => {
                e.insert((
                    crate::interaction::state::Disabled,
                    DisabledReason::new("Needs a saved game"),
                ));
            }
            Pin::NotYet => {
                e.insert(NotAvailableYet::new(&NAV_LIBRARY));
            }
        }
    }
}

fn column(commands: &mut Commands, gap: f32) -> Entity {
    commands
        .spawn(Node {
            flex_direction: FlexDirection::Column,
            row_gap: Val::Px(gap),
            align_items: AlignItems::FlexStart,
            ..default()
        })
        .id()
}

fn row(commands: &mut Commands, gap: f32) -> Entity {
    commands
        .spawn(Node {
            flex_direction: FlexDirection::Row,
            column_gap: Val::Px(gap),
            align_items: AlignItems::Center,
            flex_wrap: FlexWrap::Wrap,
            ..default()
        })
        .id()
}

fn heading(commands: &mut Commands, text: &str) -> Entity {
    LabelBuilder::new(text)
        .with_style(TypeScale::TitleMedium)
        .spawn(commands)
}

/// Buttons: every variant in every pinned state.
fn button_section(commands: &mut Commands) -> Entity {
    // Buttons: every variant in every pinned state.
    let buttons = column(commands, 8.0);
    let mut rows = vec![heading(
        commands,
        "Buttons: rest, hover, focus, pressed, disabled, not yet",
    )];
    type Make = fn(String) -> ButtonBuilder;
    let variants: [(&str, Make); 5] = [
        ("filled", ButtonBuilder::filled),
        ("elevated", ButtonBuilder::elevated),
        ("tonal", ButtonBuilder::tonal),
        ("outlined", ButtonBuilder::outlined),
        ("text", ButtonBuilder::text),
    ];
    for (variant, make) in variants {
        let line = row(commands, 12.0);
        let mut cells = Vec::new();
        for pin in Pin::ALL {
            let b = make(format!("{variant} {}", pin.name())).spawn(commands);
            commands.entity(b).insert(UiId::new(format!(
                "showcase.button.{variant}.{}",
                pin.name()
            )));
            pin.apply(commands, b);
            cells.push(b);
        }
        commands.entity(line).add_children(&cells);
        rows.push(line);
    }
    let tip = ButtonBuilder::outlined("tooltip").spawn(commands);
    commands.entity(tip).insert((
        Tooltip::new("Hover or focus for 0.6 s (Ctrl+T)"),
        UiId::new("showcase.button.tooltip"),
    ));
    rows.push(tip);
    commands.entity(buttons).add_children(&rows);

    buttons
}

/// Spawns the whole showcase under `parent` and returns its root.
pub fn spawn_showcase(commands: &mut Commands, parent: Entity) -> Entity {
    let root = commands
        .spawn(Node {
            flex_direction: FlexDirection::Row,
            flex_wrap: FlexWrap::Wrap,
            column_gap: Val::Px(48.0),
            row_gap: Val::Px(24.0),
            align_items: AlignItems::FlexStart,
            ..default()
        })
        .id();
    let sections = [
        button_section(commands),
        toggle_section(commands),
        field_section(commands),
        composite_section(commands),
        misc_section(commands),
    ];
    commands.entity(root).add_children(&sections);
    commands.entity(parent).add_child(root);
    root
}
