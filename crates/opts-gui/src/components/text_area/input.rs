//! Keyboard and pointer handling for [`TextArea`].

use bevy::input::keyboard::{Key, KeyboardInput};
use bevy::input::ButtonState;
use bevy::prelude::*;

use super::{TextArea, TextClipboard};
use crate::interaction::{
    focus::FocusEvent,
    state::{Focused, Pressed},
};

/// Lines moved by PageUp / PageDown.
const PAGE_LINES: isize = 10;

/// Sent whenever the text of a [`TextArea`] changes.
#[derive(Message, Debug, Clone, PartialEq, Eq)]
pub struct TextAreaChanged {
    /// The text area entity.
    pub entity: Entity,
    /// Its new content.
    pub text: String,
}

/// Focuses a text area on click and mirrors the shared [`Focused`] marker into `TextArea::focused`.
pub fn text_area_focus_system(
    mut areas: Query<(Entity, Has<Pressed>, &mut TextArea, Has<Focused>)>,
    mut focus: MessageWriter<FocusEvent>,
) {
    for (entity, pressed, mut area, has_focus) in &mut areas {
        if pressed && !has_focus {
            focus.write(FocusEvent::Set(entity));
        }
        if area.focused != has_focus {
            area.focused = has_focus;
            area.blink_clock = 0.0;
        }
    }
}

/// Applies keyboard input to the focused text area(s).
pub fn text_area_input_system(
    mut keys: MessageReader<KeyboardInput>,
    pressed: Res<ButtonInput<KeyCode>>,
    mut clipboard: NonSendMut<TextClipboard>,
    mut areas: Query<(Entity, &mut TextArea)>,
    mut changed: MessageWriter<TextAreaChanged>,
) {
    let events: Vec<KeyboardInput> = keys.read().cloned().collect();
    if events.is_empty() {
        return;
    }
    let shift = pressed.any_pressed([KeyCode::ShiftLeft, KeyCode::ShiftRight]);
    let ctrl = pressed.any_pressed([
        KeyCode::ControlLeft,
        KeyCode::ControlRight,
        KeyCode::SuperLeft,
        KeyCode::SuperRight,
    ]);
    for (entity, mut area) in &mut areas {
        if !area.focused {
            continue;
        }
        let before = area.buffer.text().to_owned();
        let cursor_before = area.buffer.cursor();
        for ev in events.iter().filter(|e| e.state == ButtonState::Pressed) {
            apply_key(&mut area, ev, shift, ctrl, &mut clipboard);
        }
        if area.buffer.text() != before {
            changed.write(TextAreaChanged {
                entity,
                text: area.buffer.text().to_owned(),
            });
        }
        if area.buffer.text() != before || area.buffer.cursor() != cursor_before {
            area.blink_clock = 0.0;
        }
    }
}

fn apply_key(
    area: &mut TextArea,
    ev: &KeyboardInput,
    shift: bool,
    ctrl: bool,
    clip: &mut TextClipboard,
) {
    let b = &mut area.buffer;
    match ev.key_code {
        KeyCode::ArrowLeft => b.move_left(shift),
        KeyCode::ArrowRight => b.move_right(shift),
        KeyCode::ArrowUp => b.move_up(shift),
        KeyCode::ArrowDown => b.move_down(shift),
        KeyCode::PageUp => b.move_lines(-PAGE_LINES, shift),
        KeyCode::PageDown => b.move_lines(PAGE_LINES, shift),
        KeyCode::Home if ctrl => b.move_doc(false, shift),
        KeyCode::End if ctrl => b.move_doc(true, shift),
        KeyCode::Home => b.home(shift),
        KeyCode::End => b.end(shift),
        KeyCode::Backspace => b.backspace(),
        KeyCode::Delete => b.delete(),
        KeyCode::Enter | KeyCode::NumpadEnter => b.insert_char('\n'),
        KeyCode::KeyA if ctrl => b.select_all(),
        KeyCode::KeyC if ctrl => {
            if let Some(t) = b.selected_text() {
                clip.set(&t);
            }
        }
        KeyCode::KeyX if ctrl => {
            if let Some(t) = b.selected_text() {
                clip.set(&t);
                b.delete_selection();
            }
        }
        KeyCode::KeyV if ctrl => {
            let t = clip.get();
            if !t.is_empty() {
                b.insert_str(&t);
            }
        }
        _ if ctrl => {}
        _ => match &ev.logical_key {
            Key::Character(s) => {
                let printable: String = s.chars().filter(|c| !c.is_control()).collect();
                if !printable.is_empty() {
                    b.insert_str(&printable);
                }
            }
            Key::Space => b.insert_char(' '),
            _ => {}
        },
    }
}

#[cfg(test)]
mod tests;
