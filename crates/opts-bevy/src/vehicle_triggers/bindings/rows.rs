//! The key rows of the DirectInput scan codes (`DIK_*`), by code from the first key of the row.

use bevy::prelude::KeyCode as K;
use bevy::prelude::KeyCode;

pub(super) const LETTERS_Q: [KeyCode; 10] = [
    K::KeyQ,
    K::KeyW,
    K::KeyE,
    K::KeyR,
    K::KeyT,
    K::KeyY,
    K::KeyU,
    K::KeyI,
    K::KeyO,
    K::KeyP,
];
pub(super) const LETTERS_A: [KeyCode; 9] = [
    K::KeyA,
    K::KeyS,
    K::KeyD,
    K::KeyF,
    K::KeyG,
    K::KeyH,
    K::KeyJ,
    K::KeyK,
    K::KeyL,
];
pub(super) const LETTERS_Z: [KeyCode; 7] = [
    K::KeyZ,
    K::KeyX,
    K::KeyC,
    K::KeyV,
    K::KeyB,
    K::KeyN,
    K::KeyM,
];
pub(super) const DIGITS: [KeyCode; 10] = [
    K::Digit1,
    K::Digit2,
    K::Digit3,
    K::Digit4,
    K::Digit5,
    K::Digit6,
    K::Digit7,
    K::Digit8,
    K::Digit9,
    K::Digit0,
];
pub(super) const FKEYS: [KeyCode; 10] = [
    K::F1,
    K::F2,
    K::F3,
    K::F4,
    K::F5,
    K::F6,
    K::F7,
    K::F8,
    K::F9,
    K::F10,
];
