//! Unit tests for `super` (`action_keys.rs`).

use super::*;

#[test]
fn defaults_parse_and_are_unique() {
    let keys = ActionKeys::default();
    let mut seen = std::collections::HashSet::new();
    for a in ACTIONS {
        let chord = keys
            .chord(a.id)
            .unwrap_or_else(|| panic!("{} parses", a.id));
        assert_eq!(chord.to_text().as_deref(), Some(a.default), "{}", a.id);
        assert!(seen.insert(a.default), "{} duplicates {}", a.id, a.default);
        assert!(!a.id.is_empty() && !a.group.is_empty());
    }
}

#[test]
fn modifiers_must_match_exactly() {
    let keys = ActionKeys::default();
    let mut input = ButtonInput::<KeyCode>::default();
    input.press(KeyCode::F7);
    assert!(keys.just_pressed(&input, "minimap_toggle"));
    assert!(!keys.just_pressed(&input, "minimap_rotate"));
    input.clear();
    input.release(KeyCode::F7);
    input.clear();
    input.press(KeyCode::ShiftLeft);
    input.press(KeyCode::F7);
    assert!(keys.just_pressed(&input, "minimap_rotate"));
    assert!(!keys.just_pressed(&input, "minimap_toggle"));
}

#[test]
fn a_rebinding_moves_the_key_and_unbinding_silences_it() {
    let mut keys = ActionKeys::default();
    keys.set("help", KeyChord::parse("Ctrl+H").unwrap());
    let mut input = ButtonInput::<KeyCode>::default();
    input.press(KeyCode::F1);
    assert!(!keys.just_pressed(&input, "help"));
    input.clear();
    input.release(KeyCode::F1);
    input.clear();
    input.press(KeyCode::ControlLeft);
    input.press(KeyCode::KeyH);
    assert!(keys.just_pressed(&input, "help"));
    assert_eq!(keys.label("help"), "Ctrl+H");
    keys.set("help", None);
    assert!(!keys.just_pressed(&input, "help"));
    assert_eq!(keys.label("help"), "none");
}

#[test]
fn labels_print_the_key_as_it_is_on_the_keyboard() {
    let mut keys = ActionKeys::default();
    assert_eq!(keys.label("parking_brake"), ".");
    assert_eq!(keys.label("minimap_zoom_in"), "=");
    keys.set("help", KeyChord::parse("Ctrl+Shift+Slash").unwrap());
    assert_eq!(keys.label("help"), "Ctrl+Shift+/");
    assert_eq!(keys.label("minimap_rotate"), "Shift+F7");
}

#[test]
fn the_holder_of_a_chord_is_named() {
    let keys = ActionKeys::default();
    let f6 = KeyChord::parse("F6").unwrap().unwrap();
    assert_eq!(keys.holder_of(f6, "help"), Some("full_map"));
    assert_eq!(keys.holder_of(f6, "full_map"), None);
}

#[test]
fn a_key_that_says_another_letter_is_labelled_with_it() {
    let mut keys = ActionKeys::default();
    assert_eq!(keys.label("rail_lever_down"), "Z");
    // A German keyboard: the key at the Z position says Y.
    assert!(keys.learn_letter(KeyCode::KeyZ, 'y'));
    assert_eq!(keys.label("rail_lever_down"), "Y");
    assert!(!keys.learn_letter(KeyCode::KeyZ, 'Y'), "nothing new");
    // The same key saying Z again (the layout changed) goes back to its US name.
    assert!(keys.learn_letter(KeyCode::KeyZ, 'z'));
    assert_eq!(keys.label("rail_lever_down"), "Z");
    // A key that says what its position says is left alone.
    assert!(!keys.learn_letter(KeyCode::KeyQ, 'q'));
}
