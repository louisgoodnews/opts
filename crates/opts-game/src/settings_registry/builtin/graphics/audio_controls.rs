//! The audio volumes and the controls entry of the settings registry (part of the Graphics
//! builder's list, kept apart to stay under the file-size limit).

use super::*;

/// The audio volumes.
pub(super) fn audio() -> Vec<D> {
    vec![
        D::new(
            "audio.master",
            K::Int {
                min: 0,
                max: 100,
                step: 5,
            },
            "80",
            C::Audio,
            10,
            "set-audio-master",
            "set-audio-master-desc",
        )
        .keywords(&["volume", "sound"])
        .since("0.4"),
        D::new(
            "audio.engine",
            K::Int {
                min: 0,
                max: 100,
                step: 5,
            },
            "80",
            C::Audio,
            20,
            "set-audio-engine",
            "set-audio-engine-desc",
        )
        .keywords(&["volume", "sound"])
        .since("0.4"),
        D::new(
            "audio.effects",
            K::Int {
                min: 0,
                max: 100,
                step: 5,
            },
            "80",
            C::Audio,
            25,
            "set-audio-effects",
            "set-audio-effects-desc",
        )
        .keywords(&["volume", "sound", "doors", "horn", "switches"])
        .since("0.4"),
        D::new(
            "audio.ambience",
            K::Int {
                min: 0,
                max: 100,
                step: 5,
            },
            "60",
            C::Audio,
            30,
            "set-audio-ambience",
            "set-audio-ambience-desc",
        )
        .keywords(&["volume", "sound"])
        .since("0.4"),
        D::new(
            "audio.interface",
            K::Int {
                min: 0,
                max: 100,
                step: 5,
            },
            "60",
            C::Audio,
            40,
            "set-audio-interface",
            "set-audio-interface-desc",
        )
        .keywords(&["volume", "sound", "clicks"])
        .since("0.4"),
    ]
}

/// The controls page entry.
pub(super) fn controls() -> Vec<D> {
    vec![D::new(
        "controls.rebind",
        K::Action,
        "",
        C::Controls,
        20,
        "set-rebind",
        "set-rebind-desc",
    )
    .keywords(&["keys", "keyboard", "gamepad", "wheel", "binding"])
    .not_yet(&SETTINGS_REBIND)]
}
