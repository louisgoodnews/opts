//! The set-variable rows: one row per option, with its stepper or toggle.

use super::*;

pub(in crate::in_game::vehicle_menu) fn setvar_rows(
    list: &mut ChildSpawnerCommands,
    items: &[crate::new_game_screen::setvars_discovery::SetvarItem],
    live: bool,
) {
    let grouped = items.iter().any(|i| i.group.is_some());
    let mut group: Option<&str> = Some("");
    for item in items {
        if grouped && group != Some(item.group.as_deref().unwrap_or("other")) {
            let name = item.group.as_deref().unwrap_or("other");
            group = Some(name);
            let mut title = name.to_string();
            if let Some(first) = title.get_mut(..1) {
                first.make_ascii_uppercase();
            }
            label(list, title, 13.0, ColorRole::Primary);
        }
        let step = |delta: i32| {
            if live {
                Action::LiveStep(item.name.clone(), delta)
            } else {
                Action::Step(item.name.clone(), delta)
            }
        };
        let toggle = if live {
            Action::LiveToggle(item.name.clone())
        } else {
            Action::Toggle(item.name.clone())
        };
        list.spawn(Node {
            flex_direction: FlexDirection::Row,
            justify_content: JustifyContent::SpaceBetween,
            align_items: AlignItems::Center,
            width: Val::Percent(100.0),
            ..default()
        })
        .with_children(|row| {
            label(
                row,
                format!("{} ({})", item.label, item.name),
                12.0,
                ColorRole::OnSurface,
            );
            if item.respawn == Respawn::Required {
                label(row, respawn_marker(), 11.0, ColorRole::Error);
            }
            match item.kind {
                SetvarKind::Choice | SetvarKind::Number => {
                    row.spawn(Node {
                        align_items: AlignItems::Center,
                        column_gap: Val::Px(6.0),
                        ..default()
                    })
                    .with_children(|stepper| {
                        button(stepper, "‹", false, step(-1));
                        label(stepper, item.value.to_string(), 12.0, ColorRole::OnSurface);
                        button(stepper, "›", false, step(1));
                    });
                }
                _ => {
                    let text = if item.value == 0 { "off (0)" } else { "on (1)" };
                    button(row, text, item.value != 0, toggle);
                }
            }
        });
    }
}
