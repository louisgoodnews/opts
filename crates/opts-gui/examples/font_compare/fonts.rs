//! The candidate families and the files they are read from (missing files are skipped).

use bevy::prelude::*;

/// (family name inside the files, files to register). Missing files are skipped.
pub(super) const CANDIDATES: &[(&str, &[&str])] = &[
    (
        "Noto Sans",
        &[
            "/usr/share/fonts/noto/NotoSans-Regular.ttf",
            "/usr/share/fonts/noto/NotoSans-Medium.ttf",
            "/usr/share/fonts/noto/NotoSans-Bold.ttf",
        ],
    ),
    (
        "Inter",
        &[
            "/usr/share/pycharm/jbr/lib/fonts/Inter-Regular.otf",
            "/usr/share/pycharm/jbr/lib/fonts/Inter-SemiBold.otf",
        ],
    ),
    (
        "DejaVu Sans",
        &[
            "/usr/share/fonts/TTF/DejaVuSans.ttf",
            "/usr/share/fonts/TTF/DejaVuSans-Bold.ttf",
        ],
    ),
    (
        "Open Sans",
        &[
            "/usr/share/fonts/TTF/OpenSans-Regular.ttf",
            "/usr/share/fonts/TTF/OpenSans-SemiBold.ttf",
            "/usr/share/fonts/TTF/OpenSans-Bold.ttf",
        ],
    ),
];

pub(super) const MONO: &[(&str, &[&str])] = &[
    (
        "Noto Sans Mono",
        &["/usr/share/fonts/noto/NotoSansMono-Regular.ttf"],
    ),
    (
        "JetBrains Mono",
        &["/usr/share/pycharm/jbr/lib/fonts/JetBrainsMono-Regular.ttf"],
    ),
];
pub(super) fn load_font_handles(fonts: &mut Assets<Font>) -> Vec<Handle<Font>> {
    let mut handles = Vec::new();
    for (_, files) in CANDIDATES.iter().chain(MONO) {
        for f in *files {
            if let Ok(bytes) = std::fs::read(f) {
                handles.push(fonts.add(Font::from_bytes(bytes)));
            }
        }
    }
    handles
}
